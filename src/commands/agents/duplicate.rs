//! `agents duplicate` — clone an existing agent configuration.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    name: Option<String>,
) -> Result<(), AppError> {
    let url = format!("/v1/convai/agents/{agent_id}/duplicate");
    // API accepts an optional `name` override for the new agent.
    let body = match &name {
        Some(n) => serde_json::json!({ "name": n }),
        None => serde_json::json!({}),
    };
    let resp: serde_json::Value = client.post_json(&url, &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let new_id = v.get("agent_id").and_then(|x| x.as_str()).unwrap_or("");
        println!(
            "{} duplicated {} -> {}",
            "+".green(),
            agent_id.dimmed(),
            new_id.bold()
        );
    })?;
    Ok(())
}
