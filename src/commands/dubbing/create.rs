//! `dubbing create` — POST /v1/dubbing
//!
//! Multipart body. The ElevenLabs dubbing endpoint accepts EITHER:
//!   * `file=<binary>`  (local source media), or
//!   * `source_url=<string>` (publicly reachable URL).
//!
//! Plus scalar params (`target_lang`, `source_lang`, `num_speakers`, etc.).
//!
//! Grounded against `elevenlabs-python/src/elevenlabs/dubbing/raw_client.py`
//! — the SDK uses the same multipart form shape.

use std::path::Path;

use crate::cli::DubbingCreateArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    args: DubbingCreateArgs,
) -> Result<(), AppError> {
    // Exactly one of --file / --source-url is required.
    match (&args.file, &args.source_url) {
        (Some(_), Some(_)) => {
            return Err(AppError::bad_input_with(
                "pass only one of --file or --source-url, not both",
                format!(
                    "elevenlabs dubbing create --target-lang {} --file <path>  \
                     (or --source-url <https://…>, but not both)",
                    args.target_lang
                ),
            ));
        }
        (None, None) => {
            return Err(AppError::bad_input_with(
                "either --file <path> or --source-url <url> is required",
                format!(
                    "elevenlabs dubbing create --target-lang {} --file <path>  \
                     (or --source-url <https://…>)",
                    args.target_lang
                ),
            ));
        }
        _ => {}
    }

    let mut form = reqwest::multipart::Form::new().text("target_lang", args.target_lang.clone());

    if let Some(src) = &args.source_lang {
        form = form.text("source_lang", src.clone());
    }
    if let Some(n) = args.num_speakers {
        form = form.text("num_speakers", n.to_string());
    }
    if let Some(w) = args.watermark {
        form = form.text("watermark", w.to_string());
    }
    if let Some(s) = args.start_time {
        form = form.text("start_time", s.to_string());
    }
    if let Some(e) = args.end_time {
        form = form.text("end_time", e.to_string());
    }
    if let Some(hr) = args.highest_resolution {
        form = form.text("highest_resolution", hr.to_string());
    }
    if let Some(drop_bg) = args.drop_background_audio {
        form = form.text("drop_background_audio", drop_bg.to_string());
    }
    if let Some(pf) = args.use_profanity_filter {
        form = form.text("use_profanity_filter", pf.to_string());
    }
    if let Some(ds) = args.dubbing_studio {
        form = form.text("dubbing_studio", ds.to_string());
    }
    if let Some(dv) = args.disable_voice_cloning {
        form = form.text("disable_voice_cloning", dv.to_string());
    }
    if let Some(m) = &args.mode {
        form = form.text("mode", m.clone());
    }

    // Attach source.
    if let Some(url) = &args.source_url {
        form = form.text("source_url", url.clone());
    } else if let Some(f) = &args.file {
        let path = Path::new(f);
        if !path.exists() {
            return Err(AppError::InvalidInput {
                msg: format!("file does not exist: {}", path.display()),
                suggestion: None,
            });
        }
        let bytes = crate::commands::read_file_bytes(path).await?;
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "source".to_string());
        let mime = crate::commands::mime_for_path(path);
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        form = form.part("file", part);
    }

    let resp: serde_json::Value = client.post_multipart_json("/v1/dubbing", form).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let dubbing_id = v
            .get("dubbing_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let expected = v
            .get("expected_duration_sec")
            .and_then(|x| x.as_f64())
            .unwrap_or_default();
        println!(
            "{} queued dubbing {} (target={}, expected~{:.1}s)",
            "+".green(),
            dubbing_id.bold(),
            args.target_lang.bold(),
            expected,
        );
        println!(
            "  {} {}",
            "poll:".dimmed(),
            format!("elevenlabs dubbing show {dubbing_id}").dimmed()
        );
    })?;
    Ok(())
}
