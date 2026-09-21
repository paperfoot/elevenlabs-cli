//! `agents delete` — remove an agent. Irreversible server-side.
//!
//! Requires `--yes` because deletion cascades: conversations, attached
//! knowledge-base entries, and tool-dependency edges all disappear with
//! the agent.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!(
                "refusing to delete agent '{agent_id}' without --yes \
                 (agent deletion is irreversible)"
            ),
            suggestion: Some(format!("elevenlabs agents delete {agent_id} --yes")),
        });
    }
    let path = format!("/v1/convai/agents/{agent_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "agent_id": agent_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted agent {}", "-".red(), agent_id.dimmed());
    })?;
    Ok(())
}
