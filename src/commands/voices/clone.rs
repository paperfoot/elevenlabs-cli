//! voices clone — instant voice clone (IVC) from sample files.
//! POST /v1/voices/add (multipart).

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    name: String,
    files: Vec<String>,
    description: Option<String>,
) -> Result<(), AppError> {
    if files.is_empty() {
        return Err(AppError::InvalidInput {
            msg: "at least one sample file required".into(),
            suggestion: None,
        });
    }
    let mut form = reqwest::multipart::Form::new().text("name", name.clone());
    if let Some(d) = description.clone() {
        form = form.text("description", d);
    }

    for f in &files {
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
            .unwrap_or_else(|| "sample.mp3".to_string());
        let mime = crate::commands::mime_for_path(path);
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        form = form.part("files", part);
    }

    let resp: serde_json::Value = client.post_multipart_json("/v1/voices/add", form).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} cloned voice: {} ({})",
            "+".green(),
            v.get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&name)
                .bold(),
            v.get("voice_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    })?;
    Ok(())
}
