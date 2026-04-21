//! `dubbing get-transcript <dubbing_id> <language_code> --format <srt|webvtt|json>`
//! — GET /v1/dubbing/{id}/transcripts/{lang}/format/{format_type}
//!
//! The format-in-path transcript endpoint lives under the plural
//! `/transcripts/` segment per the ElevenLabs OpenAPI spec (operationId
//! `get_dubbing_transcripts`). v0.2.x of this CLI called the singular
//! `/transcript/` path, which returns a raw transcript instead of the
//! formatted one — fixed in v0.3.0. Per the SDK it returns `text/*` for
//! srt/webvtt and `application/json` for json. We stream bytes and pick
//! an extension to match `--format` unless the caller overrides via
//! `--output`.

use std::path::PathBuf;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    dubbing_id: &str,
    language_code: &str,
    format: &str,
    output: Option<String>,
) -> Result<(), AppError> {
    // Validate format (clap enforces this too, but defence-in-depth keeps
    // unit tests honest).
    let ext = match format {
        "srt" => "srt",
        "webvtt" => "vtt",
        "json" => "json",
        other => {
            return Err(AppError::InvalidInput {
                msg: format!("invalid --format '{other}'; expected one of: srt, webvtt, json"),
                suggestion: None,
            });
        }
    };

    let path = format!("/v1/dubbing/{dubbing_id}/transcripts/{language_code}/format/{format}");
    let bytes = super::get_bytes(client, &path).await?;
    let bytes_written = bytes.len();

    let out_path: PathBuf = match output {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(format!("dub_{dubbing_id}_{language_code}.{ext}")),
    };
    tokio::fs::write(&out_path, &bytes)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "dubbing_id": dubbing_id,
        "language_code": language_code,
        "format": format,
        "output": out_path.display().to_string(),
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB, {}/{})",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0,
            r["language_code"].as_str().unwrap_or("").dimmed(),
            r["format"].as_str().unwrap_or("").dimmed(),
        );
    });
    Ok(())
}
