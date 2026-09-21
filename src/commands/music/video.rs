//! music video-to-music — POST /v1/music/video-to-music
//!
//! Contract (verified against elevenlabs-python/src/elevenlabs/music/
//! raw_client.py `video_to_music`, April 2026):
//!
//!   - multipart body, repeatable part `videos` (one per input video);
//!     videos are combined in order server-side
//!   - optional form fields: `description`, repeated `tags`,
//!     `sign_with_c2pa`
//!   - optional query: `output_format`
//!   - response: raw audio bytes (same as `music compose`), file
//!     extension derived from the requested `output_format`
//!
//! Historical note: pre-v0.2 this CLI used the multipart part name
//! `file` and sent `model_id`. Neither matches the SDK — the part is
//! `videos`, and there is no model_id field on this endpoint. See
//! `plans/cli-snippets/fixes/beta/cli.rs` for the clap-level changes.
//!
//! The CLI accepts a single `--file` for now. Each invocation attaches
//! one video under the `videos` part name so multi-file support is a
//! flag-level change rather than a rewrite.

use std::path::Path;

use crate::cli::VideoToMusicArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: VideoToMusicArgs,
) -> Result<(), AppError> {
    let path = Path::new(&args.file);
    if !path.exists() {
        return Err(AppError::bad_input(format!(
            "video file does not exist: {}",
            path.display()
        )));
    }

    let bytes = crate::commands::read_file_bytes(path).await?;
    let mime = crate::commands::mime_for_path(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".to_string());
    // Part name is `videos` per the SDK — NOT `file`. The CLI sends one
    // per invocation; the server accepts a list.
    let video_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new().part("videos", video_part);
    if let Some(desc) = &args.description {
        form = form.text("description", desc.clone());
    }
    for tag in &args.tags {
        form = form.text("tags", tag.clone());
    }
    if args.sign_with_c2pa {
        form = form.text("sign_with_c2pa", "true".to_string());
    }

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());
    let query = [("output_format", output_format.as_str())];

    let audio = client
        .post_multipart_bytes_with_query("/v1/music/video-to-music", &query, form)
        .await?;
    let bytes_written = audio.len();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "output": out_path.display().to_string(),
        "output_format": output_format,
        "description": args.description,
        "tags": args.tags,
        "sign_with_c2pa": args.sign_with_c2pa,
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0
        );
    })?;
    Ok(())
}
