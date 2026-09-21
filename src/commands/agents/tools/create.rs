//! `agents tools create --config <json_file>` — create a tool from a JSON
//! body. Pass-through pattern: the file contents become the POST body.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, config: String) -> Result<(), AppError> {
    let path = Path::new(&config);
    if !path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("config file does not exist: {}", path.display()),
            suggestion: None,
        });
    }
    let body_text = tokio::fs::read_to_string(path)
        .await
        .map_err(AppError::Io)?;
    let body: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| AppError::InvalidInput {
            msg: format!("config file {} is not valid JSON: {e}", path.display()),
            suggestion: None,
        })?;

    let resp: serde_json::Value = client.post_json("/v1/convai/tools", &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let id = v
            .get("id")
            .or_else(|| v.get("tool_id"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let name = v
            .get("tool_config")
            .and_then(|c| c.get("name"))
            .or_else(|| v.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        println!(
            "{} created tool {} ({})",
            "+".green(),
            name.bold(),
            id.dimmed()
        );
    })?;
    Ok(())
}
