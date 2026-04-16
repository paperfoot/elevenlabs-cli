//! text-to-speech
//!
//! Routes to one of four endpoints based on flags:
//!   - default:              POST /v1/text-to-speech/{voice_id}
//!   - --stream:             POST /v1/text-to-speech/{voice_id}/stream
//!   - --with-timestamps:    POST /v1/text-to-speech/{voice_id}/with-timestamps
//!     (returns JSON: audio_base64 + per-character alignment)
//!
//! Grounded against `BodyTextToSpeechFull`, `BodyTextToSpeechFullWithTimestamps`,
//! `StreamTextToSpeechRequest`, and `AudioWithTimestampsResponse` in
//! @elevenlabs/elevenlabs-js v2.43. `enable_logging`, `output_format`, and
//! `optimize_streaming_latency` are query params; everything else is JSON body.

use base64::Engine as _;
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cli::TtsArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct TtsResult {
    voice_id: String,
    model_id: String,
    endpoint: String,
    characters: usize,
    output_format: String,
    output_path: Option<String>,
    alignment_path: Option<String>,
    bytes_written: usize,
}

pub async fn run(ctx: Ctx, mut args: TtsArgs) -> Result<(), AppError> {
    // Read stdin if `-`.
    if args.text == "-" {
        let mut s = String::new();
        tokio::io::stdin()
            .read_to_string(&mut s)
            .await
            .map_err(AppError::Io)?;
        args.text = s.trim().to_string();
    }
    if args.text.trim().is_empty() {
        return Err(AppError::InvalidInput("text is empty".into()));
    }

    // Validation: combined stream+with-timestamps uses a separate NDJSON
    // protocol that we don't support yet. Tell the user which to pick.
    if args.stream && args.with_timestamps {
        return Err(AppError::InvalidInput(
            "--stream + --with-timestamps together uses an NDJSON stream protocol \
             not yet supported. Use one or the other in v0.1.4."
                .into(),
        ));
    }
    if args.previous_request_ids.len() > 3 {
        return Err(AppError::InvalidInput(
            "--previous-request-id may be specified at most 3 times".into(),
        ));
    }
    if args.next_request_ids.len() > 3 {
        return Err(AppError::InvalidInput(
            "--next-request-id may be specified at most 3 times".into(),
        ));
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    let voice_id = if let Some(id) = &args.voice_id {
        id.clone()
    } else if let Some(name) = &args.voice {
        resolve_voice_name(&client, name).await?
    } else {
        cfg.default_voice_id()
    };

    let model_id = args.model.clone().unwrap_or_else(|| cfg.default_model_id());
    let output_format = args
        .format
        .clone()
        .unwrap_or_else(|| cfg.default_output_format());

    // Body (JSON).
    let body = build_body(&args, &model_id);

    // Query params (output_format, enable_logging, optimize_streaming_latency).
    let mut query: Vec<(&str, String)> = vec![("output_format", output_format.clone())];
    if args.no_logging {
        query.push(("enable_logging", "false".to_string()));
    }
    if let Some(o) = args.optimize_streaming_latency {
        query.push(("optimize_streaming_latency", o.to_string()));
    }

    // Route to the right endpoint.
    let (path, endpoint_label) = if args.with_timestamps {
        (
            format!("/v1/text-to-speech/{voice_id}/with-timestamps"),
            "with-timestamps",
        )
    } else if args.stream {
        (format!("/v1/text-to-speech/{voice_id}/stream"), "stream")
    } else {
        (format!("/v1/text-to-speech/{voice_id}"), "convert")
    };

    let characters = args.text.chars().count();

    if args.with_timestamps {
        // JSON response: decode audio_base64, save alignment JSON separately.
        let resp: serde_json::Value = client.post_json_with_query(&path, &query, &body).await?;
        let audio_b64 = resp
            .get("audio_base64")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::Http("with-timestamps response missing audio_base64".into())
            })?;
        let audio = base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .map_err(|e| AppError::Http(format!("decode audio base64: {e}")))?;

        let ext = extension_for_format(&output_format);
        let out_path = crate::commands::resolve_output_path(args.output.clone(), "tts", ext);
        tokio::fs::write(&out_path, &audio)
            .await
            .map_err(AppError::Io)?;

        // Persist alignment JSON. Default to <audio>.timings.json if no path given.
        let alignment_path = args
            .save_timestamps
            .clone()
            .unwrap_or_else(|| format!("{}.timings.json", out_path.display()));
        let alignment_obj = serde_json::json!({
            "alignment": resp.get("alignment"),
            "normalized_alignment": resp.get("normalized_alignment"),
        });
        let pretty = serde_json::to_vec_pretty(&alignment_obj)
            .map_err(|e| AppError::Http(format!("serialize alignment: {e}")))?;
        tokio::fs::write(&alignment_path, pretty)
            .await
            .map_err(AppError::Io)?;

        let result = TtsResult {
            voice_id: voice_id.clone(),
            model_id: model_id.clone(),
            endpoint: endpoint_label.to_string(),
            characters,
            output_format,
            output_path: Some(out_path.display().to_string()),
            alignment_path: Some(alignment_path),
            bytes_written: audio.len(),
        };

        output::print_success_or(ctx, &result, print_human);
        return Ok(());
    }

    // Raw audio bytes path (default + --stream).
    let audio = client
        .post_json_bytes_with_query(&path, &query, &body)
        .await?;
    let bytes_written = audio.len();

    if args.stdout {
        let mut out = tokio::io::stdout();
        out.write_all(&audio).await.map_err(AppError::Io)?;
        out.flush().await.map_err(AppError::Io)?;
        return Ok(());
    }

    let ext = extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output.clone(), "tts", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = TtsResult {
        voice_id,
        model_id,
        endpoint: endpoint_label.to_string(),
        characters,
        output_format,
        output_path: Some(out_path.display().to_string()),
        alignment_path: None,
        bytes_written,
    };

    output::print_success_or(ctx, &result, print_human);
    Ok(())
}

fn print_human(r: &TtsResult) {
    use owo_colors::OwoColorize;
    let size_kb = r.bytes_written as f64 / 1024.0;
    println!(
        "{} {} ({:.1} KB, {} chars, voice={}, model={}, endpoint={})",
        "+".green(),
        r.output_path.as_deref().unwrap_or("(stdout)").bold(),
        size_kb,
        r.characters,
        r.voice_id.dimmed(),
        r.model_id.dimmed(),
        r.endpoint.dimmed(),
    );
    if let Some(p) = &r.alignment_path {
        println!("  alignment: {}", p.bold());
    }
}

fn build_body(args: &TtsArgs, model_id: &str) -> serde_json::Value {
    let mut voice_settings = serde_json::Map::new();
    voice_settings.insert(
        "stability".into(),
        serde_json::json!(args.stability.unwrap_or(0.5)),
    );
    voice_settings.insert(
        "similarity_boost".into(),
        serde_json::json!(args.similarity.unwrap_or(0.75)),
    );
    voice_settings.insert("style".into(), serde_json::json!(args.style.unwrap_or(0.0)));
    voice_settings.insert(
        "use_speaker_boost".into(),
        serde_json::json!(args.speaker_boost.unwrap_or(true)),
    );
    voice_settings.insert("speed".into(), serde_json::json!(args.speed.unwrap_or(1.0)));

    let mut body = serde_json::Map::new();
    body.insert("text".into(), serde_json::Value::String(args.text.clone()));
    body.insert(
        "model_id".into(),
        serde_json::Value::String(model_id.into()),
    );
    body.insert(
        "voice_settings".into(),
        serde_json::Value::Object(voice_settings),
    );
    if let Some(lang) = &args.language {
        body.insert(
            "language_code".into(),
            serde_json::Value::String(lang.clone()),
        );
    }
    if let Some(s) = args.seed {
        body.insert("seed".into(), serde_json::json!(s));
    }
    if let Some(t) = &args.previous_text {
        body.insert("previous_text".into(), serde_json::Value::String(t.clone()));
    }
    if let Some(t) = &args.next_text {
        body.insert("next_text".into(), serde_json::Value::String(t.clone()));
    }
    if !args.previous_request_ids.is_empty() {
        body.insert(
            "previous_request_ids".into(),
            serde_json::Value::Array(
                args.previous_request_ids
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if !args.next_request_ids.is_empty() {
        body.insert(
            "next_request_ids".into(),
            serde_json::Value::Array(
                args.next_request_ids
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if let Some(m) = &args.apply_text_normalization {
        body.insert(
            "apply_text_normalization".into(),
            serde_json::Value::String(m.clone()),
        );
    }
    if args.apply_language_text_normalization {
        body.insert(
            "apply_language_text_normalization".into(),
            serde_json::Value::Bool(true),
        );
    }
    if args.use_pvc_as_ivc {
        body.insert("use_pvc_as_ivc".into(), serde_json::Value::Bool(true));
    }
    serde_json::Value::Object(body)
}

async fn resolve_voice_name(client: &ElevenLabsClient, name: &str) -> Result<String, AppError> {
    // /v2/voices is the search-enabled endpoint. /v1/voices ignores query
    // params, which caused the silent-pick regression in v0.1.0.
    let query = [("search", name)];
    let resp: serde_json::Value = client.get_json_with_query("/v2/voices", &query).await?;
    let voices = resp
        .get("voices")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let needle_lower = name.to_lowercase();
    let mut exact: Option<&serde_json::Value> = None;
    let mut prefix: Option<&serde_json::Value> = None;
    let mut substring: Option<&serde_json::Value> = None;

    for v in &voices {
        let Some(vname) = v.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let lower = vname.to_lowercase();
        if lower == needle_lower {
            exact = Some(v);
            break;
        }
        if prefix.is_none() && lower.starts_with(&needle_lower) {
            prefix = Some(v);
        }
        if substring.is_none() && lower.contains(&needle_lower) {
            substring = Some(v);
        }
    }

    let chosen = exact.or(prefix).or(substring);
    if let Some(v) = chosen {
        if let Some(id) = v.get("voice_id").and_then(|n| n.as_str()) {
            return Ok(id.to_string());
        }
    }
    Err(AppError::InvalidInput(format!(
        "no voice in your library matches '{name}'. \
         List voices with: elevenlabs voices list"
    )))
}

pub fn extension_for_format(fmt: &str) -> &'static str {
    if fmt.starts_with("mp3") {
        "mp3"
    } else if fmt.starts_with("pcm") {
        "pcm"
    } else if fmt.starts_with("ulaw") {
        "ulaw"
    } else if fmt.starts_with("alaw") {
        "alaw"
    } else if fmt.starts_with("opus") {
        "opus"
    } else if fmt.starts_with("flac") {
        "flac"
    } else {
        "bin"
    }
}
