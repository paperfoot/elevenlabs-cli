//! `agents tools update --patch <json_file>` — PATCH an existing tool with
//! arbitrary partial JSON body.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    tool_id: String,
    patch: String,
) -> Result<(), AppError> {
    let path = Path::new(&patch);
    if !path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("patch file does not exist: {}", path.display()),
            suggestion: None,
        });
    }
    let body_text = tokio::fs::read_to_string(path)
        .await
        .map_err(AppError::Io)?;
    let body: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| AppError::InvalidInput {
            msg: format!("patch file {} is not valid JSON: {e}", path.display()),
            suggestion: None,
        })?;

    let url = format!("/v1/convai/tools/{tool_id}");
    let resp: serde_json::Value = client.patch_json(&url, &body).await?;
    output::print_success_or(ctx, &resp, |_| {
        use owo_colors::OwoColorize;
        println!("{} updated tool {}", "~".yellow(), tool_id.dimmed());
    })?;
    Ok(())
}
