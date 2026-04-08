//! speech-to-text: POST /v1/speech-to-text (multipart)

use serde::Serialize;
use std::path::Path;

use crate::cli::SttArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct SttResult {
    file: String,
    model: String,
    language_code: Option<String>,
    duration_seconds: Option<f64>,
    text: String,
    output_path: Option<String>,
    diarized: bool,
}

pub async fn run(ctx: Ctx, args: SttArgs) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::InvalidInput(format!(
            "file does not exist: {}",
            path.display()
        )));
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

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
        .part("file", file_part)
        .text("model_id", args.model.clone())
        .text("diarize", args.diarize.to_string())
        .text("tag_audio_events", args.audio_events.to_string());

    if args.timestamps {
        form = form.text("timestamps_granularity", "word");
    }
    if let Some(lang) = &args.language {
        form = form.text("language_code", lang.clone());
    }

    let resp: serde_json::Value = client
        .post_multipart_json("/v1/speech-to-text", form)
        .await?;

    let text = if args.diarize {
        format_diarized(&resp)
    } else {
        resp.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    };

    // Save to file if requested.
    let output_path = if let Some(out) = &args.output {
        tokio::fs::write(out, &text).await.map_err(AppError::Io)?;
        Some(out.clone())
    } else {
        None
    };

    let result = SttResult {
        file: path.display().to_string(),
        model: args.model.clone(),
        language_code: resp
            .get("language_code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        duration_seconds: resp
            .get("additional_formats")
            .and_then(|a| a.get("duration"))
            .and_then(|d| d.as_f64())
            .or_else(|| resp.get("duration").and_then(|d| d.as_f64())),
        text: text.clone(),
        output_path,
        diarized: args.diarize,
    };

    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        if let Some(out) = &r.output_path {
            println!(
                "{} saved transcript -> {} ({} chars)",
                "+".green(),
                out.bold(),
                r.text.len()
            );
        }
        // Print the transcript itself even when saved — human mode wants it.
        println!("{}", r.text);
    });

    Ok(())
}

fn format_diarized(resp: &serde_json::Value) -> String {
    let words = resp.get("words").and_then(|w| w.as_array()).cloned();
    let Some(words) = words else {
        return resp
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
    };

    let mut lines: Vec<String> = Vec::new();
    let mut current_speaker: Option<String> = None;
    let mut current_text: Vec<String> = Vec::new();

    for word in &words {
        let ty = word.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if ty == "spacing" {
            continue;
        }
        let speaker = word
            .get("speaker_id")
            .and_then(|s| s.as_str())
            .unwrap_or("speaker_0");
        let text = word.get("text").and_then(|t| t.as_str()).unwrap_or("");
        if text.is_empty() {
            continue;
        }

        match &current_speaker {
            Some(s) if s == speaker => current_text.push(text.trim().to_string()),
            _ => {
                if let Some(prev) = &current_speaker {
                    lines.push(format!(
                        "{}: {}",
                        prev.to_uppercase().replace('_', " "),
                        current_text.join(" ")
                    ));
                    current_text.clear();
                }
                current_speaker = Some(speaker.to_string());
                current_text.push(text.trim().to_string());
            }
        }
    }
    if let Some(prev) = &current_speaker {
        lines.push(format!(
            "{}: {}",
            prev.to_uppercase().replace('_', " "),
            current_text.join(" ")
        ));
    }
    lines.join("\n\n")
}
