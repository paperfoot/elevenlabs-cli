//! audio isolate / convert (speech-to-speech voice conversion)
//!
//! Grounded against:
//!   - `BodyAudioIsolationV1AudioIsolationPost` for isolate (multipart)
//!   - `BodySpeechToSpeechV1SpeechToSpeechVoiceIdPost` for convert (multipart,
//!     with output_format / enable_logging / optimize_streaming_latency as
//!     query params — matching the Fern-generated client).

use serde::Serialize;
use std::path::Path;

use crate::cli::AudioAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct AudioResult {
    input: String,
    output: String,
    bytes_written: usize,
}

pub async fn dispatch(ctx: Ctx, action: AudioAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        AudioAction::Isolate {
            file,
            output,
            pcm_16k,
        } => isolate(ctx, &client, file, output, pcm_16k).await,
        AudioAction::Convert {
            file,
            voice_id,
            voice,
            model,
            output,
            format,
            stability,
            similarity,
            style,
            speaker_boost,
            speed,
            seed,
            remove_background_noise,
            optimize_streaming_latency,
            pcm_16k,
            no_logging,
        } => {
            convert(
                ctx,
                &cfg,
                &client,
                ConvertArgs {
                    file,
                    voice_id,
                    voice,
                    model,
                    output,
                    format,
                    stability,
                    similarity,
                    style,
                    speaker_boost,
                    speed,
                    seed,
                    remove_background_noise,
                    optimize_streaming_latency,
                    pcm_16k,
                    no_logging,
                },
            )
            .await
        }
    }
}

async fn isolate(
    ctx: Ctx,
    client: &ElevenLabsClient,
    file: String,
    output: Option<String>,
    pcm_16k: bool,
) -> Result<(), AppError> {
    let path = Path::new(&file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "file does not exist: {}",
            path.display()
        )));
    }
    let bytes = crate::commands::read_file_bytes(path).await?;
    let mime = crate::commands::mime_for_path(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());

    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
    let mut form = reqwest::multipart::Form::new().part("audio", file_part);
    if pcm_16k {
        form = form.text("file_format", "pcm_s16le_16");
    }

    let audio = client
        .post_multipart_bytes("/v1/audio-isolation", form)
        .await?;
    let bytes_written = audio.len();

    let out_path = crate::commands::resolve_output_path(output, "iso", "mp3");
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = AudioResult {
        input: path.display().to_string(),
        output: out_path.display().to_string(),
        bytes_written,
    };
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r.output.bold(),
            r.bytes_written as f64 / 1024.0
        );
    });
    Ok(())
}

struct ConvertArgs {
    file: String,
    voice_id: Option<String>,
    voice: Option<String>,
    model: Option<String>,
    output: Option<String>,
    format: Option<String>,
    stability: Option<f32>,
    similarity: Option<f32>,
    style: Option<f32>,
    speaker_boost: Option<bool>,
    speed: Option<f32>,
    seed: Option<u32>,
    remove_background_noise: bool,
    optimize_streaming_latency: Option<u32>,
    pcm_16k: bool,
    no_logging: bool,
}

async fn convert(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: ConvertArgs,
) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "file does not exist: {}",
            path.display()
        )));
    }

    let voice_id = if let Some(id) = args.voice_id {
        id
    } else if let Some(name) = args.voice {
        resolve_voice_by_name(client, &name).await?
    } else {
        cfg.default_voice_id()
    };

    let model_id = args
        .model
        .unwrap_or_else(|| "eleven_multilingual_sts_v2".to_string());

    let output_format = args
        .format
        .clone()
        .unwrap_or_else(|| cfg.default_output_format());

    let bytes = crate::commands::read_file_bytes(path).await?;
    let mime = crate::commands::mime_for_path(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());
    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new()
        .text("model_id", model_id.clone())
        .part("audio", file_part);

    // voice_settings is a JSON-stringified form field per the SDK schema.
    let mut voice_settings = serde_json::Map::new();
    if let Some(v) = args.stability {
        voice_settings.insert("stability".into(), serde_json::json!(v));
    }
    if let Some(v) = args.similarity {
        voice_settings.insert("similarity_boost".into(), serde_json::json!(v));
    }
    if let Some(v) = args.style {
        voice_settings.insert("style".into(), serde_json::json!(v));
    }
    if let Some(v) = args.speaker_boost {
        voice_settings.insert("use_speaker_boost".into(), serde_json::json!(v));
    }
    if let Some(v) = args.speed {
        voice_settings.insert("speed".into(), serde_json::json!(v));
    }
    if !voice_settings.is_empty() {
        let vs_json = serde_json::to_string(&serde_json::Value::Object(voice_settings))
            .map_err(|e| AppError::Http(format!("serialize voice_settings: {e}")))?;
        form = form.text("voice_settings", vs_json);
    }
    if let Some(seed) = args.seed {
        form = form.text("seed", seed.to_string());
    }
    if args.remove_background_noise {
        form = form.text("remove_background_noise", "true");
    }
    if args.pcm_16k {
        form = form.text("file_format", "pcm_s16le_16");
    }

    // Query params per the Fern client: output_format, enable_logging,
    // optimize_streaming_latency.
    let mut query: Vec<(&str, String)> = vec![("output_format", output_format.clone())];
    if args.no_logging {
        query.push(("enable_logging", "false".to_string()));
    }
    if let Some(o) = args.optimize_streaming_latency {
        query.push(("optimize_streaming_latency", o.to_string()));
    }

    let url_path = format!("/v1/speech-to-speech/{voice_id}");
    let audio = client
        .post_multipart_bytes_with_query(&url_path, &query, form)
        .await?;
    let bytes_written = audio.len();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "sts", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "output": out_path.display().to_string(),
        "voice_id": voice_id,
        "model_id": model_id,
        "output_format": output_format,
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB, voice={})",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0,
            r["voice_id"].as_str().unwrap_or("").dimmed()
        );
    });
    Ok(())
}

async fn resolve_voice_by_name(client: &ElevenLabsClient, name: &str) -> Result<String, AppError> {
    // Same client-side match strategy as tts.rs. /v2/voices is required
    // because /v1/voices silently ignores the `search` param.
    let query = [("search", name)];
    let resp: serde_json::Value = client.get_json_with_query("/v2/voices", &query).await?;
    let voices = resp
        .get("voices")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let needle = name.to_lowercase();
    let mut exact: Option<&serde_json::Value> = None;
    let mut prefix: Option<&serde_json::Value> = None;
    let mut substring: Option<&serde_json::Value> = None;
    for v in &voices {
        let Some(vname) = v.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let lower = vname.to_lowercase();
        if lower == needle {
            exact = Some(v);
            break;
        }
        if prefix.is_none() && lower.starts_with(&needle) {
            prefix = Some(v);
        }
        if substring.is_none() && lower.contains(&needle) {
            substring = Some(v);
        }
    }
    if let Some(v) = exact.or(prefix).or(substring) {
        if let Some(id) = v.get("voice_id").and_then(|n| n.as_str()) {
            return Ok(id.to_string());
        }
    }
    Err(AppError::InvalidInput(format!(
        "no voice in your library matches '{name}'. \
         List voices with: elevenlabs voices list"
    )))
}
