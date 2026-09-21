//! music upload — POST /v1/music/upload
//!
//! Contract (verified against elevenlabs-python/src/elevenlabs/music/
//! raw_client.py `upload`, September 2026):
//!
//!   - multipart body, required field `file`
//!   - optional form field `extract_composition_plan` (model ID; booleans deprecated)
//!   - response: JSON `{ song_id, composition_plan?, ... }`
//!
//! Historical note: pre-v0.2 this CLI also sent `name` and
//! `composition_plan` form fields. Neither exists in the SDK — both
//! have been removed. See `plans/cli-snippets/fixes/beta/cli.rs` for
//! the clap-level argument change.
//!
//! `--extract-composition-plan` increases latency server-side (see the
//! Python SDK docstring) but returns the generated plan inline so it
//! can be piped straight into `music compose --composition-plan <file>`.

use std::path::Path;

use crate::cli::UploadArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: UploadArgs) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::bad_input(format!(
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

    let mut form = reqwest::multipart::Form::new().part("file", file_part);
    if args.extract_composition_plan {
        form = form.text(
            "extract_composition_plan",
            args.model
                .as_deref()
                .unwrap_or(super::DEFAULT_MODEL)
                .to_string(),
        );
    }

    let resp: serde_json::Value = client.post_multipart_json("/v1/music/upload", form).await?;

    let song_id = resp
        .get("song_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "song_id": song_id,
        "extract_composition_plan": args.extract_composition_plan,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} uploaded {} song_id={}",
            "+".green(),
            r["input"].as_str().unwrap_or("").bold(),
            r["song_id"].as_str().unwrap_or("(unknown)").dimmed()
        );
    })?;
    Ok(())
}
