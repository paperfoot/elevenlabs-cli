//! `agents update` — PATCH arbitrary partial config into an existing agent.
//!
//! The ElevenLabs agent config surface is too wide to model as CLI flags —
//! `conversation_config.agent.prompt.system_prompt`,
//! `conversation_config.tts.voice_id`,
//! `conversation_config.agent.prompt.tools`, etc. Rather than modelling each
//! field we accept a JSON file path whose contents become the PATCH body
//! verbatim. This is the same pass-through pattern used for `agents tools`.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::commands::agents::agent_config::validate_patch;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
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

    validate_patch(&body)?;

    let url = format!("/v1/convai/agents/{agent_id}");
    let resp: serde_json::Value = client.patch_json(&url, &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} updated agent {}",
            "~".yellow(),
            v.get("agent_id")
                .and_then(|x| x.as_str())
                .unwrap_or(&agent_id)
                .dimmed()
        );
    })?;
    Ok(())
}
