//! `dubbing get-audio <dubbing_id> <language_code>` —
//! GET /v1/dubbing/{id}/audio/{lang}
//!
//! Downloads the dubbed audio track for the given language as raw bytes and
//! writes them to disk (default: dub_<id>_<lang>.mp4 — final extension is
//! left to the user via --output).

use std::path::PathBuf;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    dubbing_id: &str,
    language_code: &str,
    output: Option<String>,
) -> Result<(), AppError> {
    let path = format!("/v1/dubbing/{dubbing_id}/audio/{language_code}");
    let bytes = super::get_bytes(client, &path).await?;
    let bytes_written = bytes.len();

    let out_path: PathBuf = match output {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(format!("dub_{dubbing_id}_{language_code}.mp4")),
    };
    tokio::fs::write(&out_path, &bytes)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "dubbing_id": dubbing_id,
        "language_code": language_code,
        "output": out_path.display().to_string(),
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB, lang={})",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0,
            r["language_code"].as_str().unwrap_or("").dimmed()
        );
    })?;
    Ok(())
}
