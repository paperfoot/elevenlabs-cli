//! forced alignment — POST /v1/forced-alignment (multipart).
//!
//! Aligns a known transcript to a given audio recording, producing
//! per-character and per-word start/end timings plus a loss value.
//!
//! Grounded against `BodyForcedAlignmentV1ForcedAlignmentPost` in
//! elevenlabs-python. Request shape:
//!   - `file`              (required, multipart): audio bytes
//!   - `text`              (required, form text): transcript
//!   - `enabled_spooled_file` (optional, form text): "true" for large uploads
//!
//! Response shape:
//!   {
//!     "characters": [{ "text", "start", "end" }, ...],
//!     "words":      [{ "text", "start", "end", "loss" }, ...],
//!     "loss":       <f64>
//!   }

use serde::Serialize;
use std::path::Path;

use crate::cli::AlignArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct AlignResult {
    input: String,
    transcript_chars: usize,
    word_count: usize,
    character_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    loss: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_path: Option<String>,
    /// Raw per-word timings array from the API.
    #[serde(default)]
    words: Vec<serde_json::Value>,
    /// Raw per-character timings array from the API.
    #[serde(default)]
    characters: Vec<serde_json::Value>,
}

pub async fn run(ctx: Ctx, args: AlignArgs) -> Result<(), AppError> {
    let audio_path = Path::new(&args.audio);
    if !audio_path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("audio file does not exist: {}", audio_path.display()),
            suggestion: None,
        });
    }

    // Resolve the transcript: `--transcript-file <path>` wins, otherwise we
    // use the positional. If the positional looks like an existing file
    // (heuristic: short, no newlines, and exists on disk), treat as a path.
    let transcript = resolve_transcript(&args).await?;
    if transcript.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg:
                "transcript is empty — provide --transcript-file <path> or a non-empty positional \
             text argument"
                    .into(),
            suggestion: None,
        });
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    let bytes = crate::commands::read_file_bytes(audio_path).await?;
    let mime = crate::commands::mime_for_path(audio_path);
    let filename = audio_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());

    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("text", transcript.clone());
    if args.enabled_spooled_file {
        form = form.text("enabled_spooled_file", "true");
    }

    let resp: serde_json::Value = client
        .post_multipart_json("/v1/forced-alignment", form)
        .await?;

    // Optional: persist the full JSON to disk.
    let output_path = if let Some(out) = &args.output {
        let pretty = serde_json::to_vec_pretty(&resp)
            .map_err(|e| AppError::Http(format!("serialize alignment: {e}")))?;
        tokio::fs::write(out, pretty).await.map_err(AppError::Io)?;
        Some(out.clone())
    } else {
        None
    };

    let words_arr: Vec<serde_json::Value> = resp
        .get("words")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let chars_arr: Vec<serde_json::Value> = resp
        .get("characters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let loss = resp.get("loss").and_then(|v| v.as_f64());

    let result = AlignResult {
        input: audio_path.display().to_string(),
        transcript_chars: transcript.chars().count(),
        word_count: words_arr.len(),
        character_count: chars_arr.len(),
        loss,
        output_path,
        words: words_arr,
        characters: chars_arr,
    };
    output::print_success_or(ctx, &result, print_human)?;
    Ok(())
}

async fn resolve_transcript(args: &AlignArgs) -> Result<String, AppError> {
    if let Some(path) = &args.transcript_file {
        return tokio::fs::read_to_string(Path::new(path))
            .await
            .map_err(|e| AppError::InvalidInput {
                msg: format!("read transcript file {path}: {e}"),
                suggestion: None,
            });
    }
    let text = args
        .transcript
        .as_ref()
        .ok_or_else(|| AppError::InvalidInput {
            msg: "forced alignment requires a transcript — pass it as the second positional \
             or use --transcript-file <path>"
                .into(),
            suggestion: None,
        })?;

    // If the argument looks like a small existing file path, load it. Large
    // pasted texts with newlines stay inline.
    let looks_like_path = text.len() < 512
        && !text.contains('\n')
        && Path::new(text).exists()
        && Path::new(text).is_file();
    if looks_like_path {
        return tokio::fs::read_to_string(Path::new(text))
            .await
            .map_err(|e| AppError::InvalidInput {
                msg: format!("read transcript file {text}: {e}"),
                suggestion: None,
            });
    }
    Ok(text.clone())
}

fn print_human(r: &AlignResult) {
    use owo_colors::OwoColorize;
    let loss = r.loss.map(|l| format!(" loss={l:.3}")).unwrap_or_default();
    println!(
        "{} aligned {} ({} chars -> {} words, {} chars){}",
        "+".green(),
        r.input.bold(),
        r.transcript_chars,
        r.word_count,
        r.character_count,
        loss.dimmed(),
    );
    if let Some(p) = &r.output_path {
        println!("  saved: {}", p.bold());
    }

    if r.words.is_empty() {
        return;
    }
    let mut t = comfy_table::Table::new();
    t.set_header(vec!["#", "word", "start", "end", "loss"]);
    let limit = 50usize.min(r.words.len());
    for (i, w) in r.words.iter().take(limit).enumerate() {
        let text = w.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let start = w
            .get("start")
            .and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}"))
            .unwrap_or_default();
        let end = w
            .get("end")
            .and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}"))
            .unwrap_or_default();
        let word_loss = w
            .get("loss")
            .and_then(|v| v.as_f64())
            .map(|v| format!("{v:.3}"))
            .unwrap_or_default();
        t.add_row(vec![i.to_string(), text.to_string(), start, end, word_loss]);
    }
    println!("{t}");
    if r.words.len() > limit {
        println!(
            "... {} more words (saved JSON has full detail)",
            r.words.len() - limit
        );
    }
}
