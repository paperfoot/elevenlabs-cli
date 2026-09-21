//! voices similar — POST /v1/similar-voices.
//!
//! Multipart body:
//!   - audio_file (required, file upload)
//!   - similarity_threshold (optional, 0..2)
//!   - top_k (optional, 1..100)
//!
use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub struct SimilarArgs {
    pub audio_file: String,
    pub similarity_threshold: Option<f32>,
    pub top_k: Option<u32>,
}

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: SimilarArgs) -> Result<(), AppError> {
    let path = Path::new(&args.audio_file);
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
        .unwrap_or_else(|| "audio".to_string());
    let mime = crate::commands::mime_for_path(path);

    let file_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
    let mut form = reqwest::multipart::Form::new().part("audio_file", file_part);

    if let Some(t) = args.similarity_threshold {
        form = form.text("similarity_threshold", t.to_string());
    }
    if let Some(k) = args.top_k {
        form = form.text("top_k", k.to_string());
    }
    let resp: serde_json::Value = client
        .post_multipart_json("/v1/similar-voices", form)
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let list = v
            .get("voices")
            .and_then(|vs| vs.as_array())
            .cloned()
            .unwrap_or_default();
        if list.is_empty() {
            println!("(no similar voices found)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Voice ID", "Name", "Category", "Description"]);
        for voice in &list {
            t.add_row(vec![
                voice
                    .get("voice_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                voice
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                voice
                    .get("category")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("description")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect::<String>(),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}
