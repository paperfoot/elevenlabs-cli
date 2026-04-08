//! text-to-speech: POST /v1/text-to-speech/{voice_id}

use serde::Serialize;
use std::io::{Read, Write};

use crate::cli::TtsArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct TtsResult {
    voice_id: String,
    model_id: String,
    characters: usize,
    output_format: String,
    output_path: Option<String>,
    bytes_written: usize,
}

pub async fn run(ctx: Ctx, mut args: TtsArgs) -> Result<(), AppError> {
    // Read stdin if `-`
    if args.text == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(AppError::Io)?;
        args.text = s.trim().to_string();
    }
    if args.text.trim().is_empty() {
        return Err(AppError::InvalidInput("text is empty".into()));
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    // Resolve voice_id (explicit > --voice search > config default)
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

    // Voice settings — only include fields the user explicitly set (or
    // sensible defaults). The API accepts partial voice_settings.
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
        serde_json::Value::String(model_id.clone()),
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

    let path = format!("/v1/text-to-speech/{voice_id}");
    let query = [("output_format", output_format.as_str())];
    let audio = client
        .post_json_bytes_with_query(&path, &query, &serde_json::Value::Object(body))
        .await?;

    let bytes_written = audio.len();

    if args.stdout {
        std::io::stdout().write_all(&audio).map_err(AppError::Io)?;
        // Don't print the envelope when writing raw bytes to stdout.
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
        characters: args.text.chars().count(),
        output_format,
        output_path: Some(out_path.display().to_string()),
        bytes_written,
    };

    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        let size_kb = r.bytes_written as f64 / 1024.0;
        println!(
            "{} {} ({:.1} KB, {} chars, voice={}, model={})",
            "+".green(),
            r.output_path.as_deref().unwrap_or("(stdout)").bold(),
            size_kb,
            r.characters,
            r.voice_id.dimmed(),
            r.model_id.dimmed(),
        );
    });

    Ok(())
}

async fn resolve_voice_name(client: &ElevenLabsClient, name: &str) -> Result<String, AppError> {
    let query = [("search", name)];
    let resp: serde_json::Value = client.get_json_with_query("/v1/voices", &query).await?;
    let voices = resp
        .get("voices")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Prefer exact name match, else first result.
    for v in &voices {
        if v.get("name").and_then(|n| n.as_str()) == Some(name) {
            if let Some(id) = v.get("voice_id").and_then(|n| n.as_str()) {
                return Ok(id.to_string());
            }
        }
    }
    if let Some(v) = voices.first() {
        if let Some(id) = v.get("voice_id").and_then(|n| n.as_str()) {
            return Ok(id.to_string());
        }
    }
    Err(AppError::InvalidInput(format!(
        "no voice found matching '{name}'"
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
