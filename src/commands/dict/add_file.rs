//! `dict add-file NAME FILE` — upload a pronunciation dictionary from a PLS
//! XML (or lexicon-format) file.
//!
//! POST /v1/pronunciation-dictionaries/add-from-file
//! Multipart: `file`, `name`, `description`, `workspace_access`.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    name: String,
    file: String,
    description: Option<String>,
    workspace_access: Option<String>,
) -> Result<(), AppError> {
    let path = Path::new(&file);
    if !path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("dictionary file does not exist: {}", path.display()),
            suggestion: None,
        });
    }
    let bytes = crate::commands::read_file_bytes(path).await?;
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "dictionary.pls".to_string());
    let mime = crate::commands::mime_for_path(path);
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(&mime)
        .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("name", name.clone());
    if let Some(d) = description {
        form = form.text("description", d);
    }
    if let Some(wa) = workspace_access {
        form = form.text("workspace_access", wa);
    }

    let resp: serde_json::Value = client
        .post_multipart_json("/v1/pronunciation-dictionaries/add-from-file", form)
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} uploaded dictionary: {} ({})",
            "+".green(),
            v.get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&name)
                .bold(),
            v.get("id").and_then(|x| x.as_str()).unwrap_or("").dimmed()
        );
        if let Some(ver) = v.get("version_id").and_then(|x| x.as_str()) {
            println!("  version: {ver}");
        }
    })?;
    Ok(())
}
