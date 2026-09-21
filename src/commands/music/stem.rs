//! music stem-separation — POST /v1/music/stem-separation
//!
//! Contract (verified against elevenlabs-python/src/elevenlabs/music/
//! raw_client.py `separate_stems`, April 2026):
//!
//!   - multipart body, required field `file` (a local audio file)
//!   - optional form fields: `stem_variation_id`, `sign_with_c2pa`
//!   - optional query: `output_format`
//!   - response: a streaming **ZIP archive** containing one audio file
//!     per stem (name + content decided by the server)
//!
//! Historical note: pre-v0.2 this CLI POSTed JSON and expected a base64
//! stem map, and supported a `song_id` text field. Both were wrong.
//! The file field is the only accepted input per the SDK. See
//! `plans/cli-snippets/fixes/beta/cli.rs` for the clap-level changes.
//!
//! Strategy: buffer the ZIP in memory (stems are small relative to the
//! source track) and unzip each entry into `--output-dir` using the
//! `zip` crate. `--output-dir` defaults to `./stems_<timestamp>`.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::cli::StemSeparationArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    args: StemSeparationArgs,
) -> Result<(), AppError> {
    // Resolve the input audio. The server only accepts a file upload —
    // no song_id form, per the SDK.
    let source = args.file.trim();
    if source.is_empty() {
        return Err(AppError::bad_input_with(
            "provide a local audio file path",
            "elevenlabs music stem-separation <FILE> [--output-dir <DIR>]",
        ));
    }
    let path = Path::new(source);
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
    if let Some(id) = &args.stem_variation_id {
        form = form.text("stem_variation_id", id.clone());
    }
    if args.sign_with_c2pa {
        form = form.text("sign_with_c2pa", "true".to_string());
    }

    // output_format is a query parameter, not a form field. We only
    // include it when the user explicitly requested a format so the
    // server can pick its default.
    let zip_bytes: Vec<u8> = match &args.output_format {
        Some(fmt) => {
            let query = [("output_format", fmt.as_str())];
            client
                .post_multipart_bytes_with_query("/v1/music/stem-separation", &query, form)
                .await?
                .to_vec()
        }
        None => client
            .post_multipart_bytes("/v1/music/stem-separation", form)
            .await?
            .to_vec(),
    };

    let out_dir = args
        .output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("stems_{}", crate::commands::now_timestamp())));
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(AppError::Io)?;

    // Unzip on a blocking thread so the async runtime isn't stalled on
    // IO. spawn_blocking returns a JoinError if the task panics; flatten
    // it into AppError.
    let out_dir_blocking = out_dir.clone();
    let written: Vec<serde_json::Value> =
        tokio::task::spawn_blocking(move || extract_zip_entries(&zip_bytes, &out_dir_blocking))
            .await
            .map_err(|e| AppError::Http(format!("zip extraction task panicked: {e}")))??;

    let result = serde_json::json!({
        "input": path.display().to_string(),
        "output_dir": out_dir.display().to_string(),
        "output_format": args.output_format,
        "stems_written": written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} stems -> {}",
            "+".green(),
            r["stems_written"].as_array().map(|a| a.len()).unwrap_or(0),
            r["output_dir"].as_str().unwrap_or("").bold()
        );
    })?;
    Ok(())
}

/// Extract every file entry from a ZIP archive buffer into `out_dir`.
/// Returns a JSON array describing each written file.
fn extract_zip_entries(
    zip_bytes: &[u8],
    out_dir: &Path,
) -> Result<Vec<serde_json::Value>, AppError> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| AppError::Http(format!("stem-separation response is not a valid zip: {e}")))?;

    let mut written: Vec<serde_json::Value> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Http(format!("read zip entry {i}: {e}")))?;

        if entry.is_dir() {
            continue;
        }

        // Sanitize the filename defensively so a malicious archive can't
        // escape out_dir via `..` or absolute paths (Zip Slip).
        let sanitized = sanitize_zip_name(entry.name());
        if sanitized.is_empty() {
            continue;
        }
        let target = out_dir.join(&sanitized);

        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
        }

        let mut out = std::fs::File::create(&target).map_err(AppError::Io)?;
        let bytes = std::io::copy(&mut entry, &mut out).map_err(AppError::Io)?;

        let stem_name = Path::new(&sanitized)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| sanitized.clone());

        written.push(serde_json::json!({
            "stem": stem_name,
            "filename": sanitized,
            "path": target.display().to_string(),
            "bytes": bytes,
        }));
    }

    Ok(written)
}

/// Strip dangerous components from a zip entry name. Returns a relative
/// path (platform-native separators) that is safe to join onto the
/// output directory.
fn sanitize_zip_name(name: &str) -> String {
    use std::path::Component;
    let raw = Path::new(name);
    let mut out = PathBuf::new();
    for comp in raw.components() {
        // Keep only `Normal`; drop RootDir/CurDir/ParentDir/Prefix so absolute
        // or escaping paths can never land outside `out_dir`.
        if let Component::Normal(c) = comp {
            out.push(c);
        }
    }
    out.to_string_lossy().to_string()
}
