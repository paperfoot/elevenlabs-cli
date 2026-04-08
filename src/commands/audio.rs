//! audio isolate / convert (speech-to-speech voice conversion)

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
        AudioAction::Isolate { file, output } => isolate(ctx, &client, file, output).await,
        AudioAction::Convert {
            file,
            voice_id,
            voice,
            model,
            output,
        } => convert(ctx, &cfg, &client, file, voice_id, voice, model, output).await,
    }
}

async fn isolate(
    ctx: Ctx,
    client: &ElevenLabsClient,
    file: String,
    output: Option<String>,
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
    let form = reqwest::multipart::Form::new().part("audio", file_part);

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

#[allow(clippy::too_many_arguments)]
async fn convert(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    file: String,
    voice_id: Option<String>,
    voice: Option<String>,
    model: Option<String>,
    output: Option<String>,
) -> Result<(), AppError> {
    let path = Path::new(&file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "file does not exist: {}",
            path.display()
        )));
    }

    let voice_id = if let Some(id) = voice_id {
        id
    } else if let Some(name) = voice {
        resolve_voice_by_name(client, &name).await?
    } else {
        cfg.default_voice_id()
    };

    let model_id = model.unwrap_or_else(|| "eleven_multilingual_sts_v2".to_string());

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

    let form = reqwest::multipart::Form::new()
        .text("model_id", model_id.clone())
        .part("audio", file_part);

    let url_path = format!("/v1/speech-to-speech/{voice_id}");
    let audio = client.post_multipart_bytes(&url_path, form).await?;
    let bytes_written = audio.len();

    let out_path = crate::commands::resolve_output_path(output, "sts", "mp3");
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "output": out_path.display().to_string(),
        "voice_id": voice_id,
        "model_id": model_id,
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
    // Same client-side match strategy as tts.rs — the server-side search
    // isn't a strict substring filter, so we resolve locally.
    let query = [("search", name)];
    let resp: serde_json::Value = client.get_json_with_query("/v1/voices", &query).await?;
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
