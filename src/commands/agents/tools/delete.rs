//! `agents tools delete --yes` — irreversible. Mirrors `voices delete` in
//! requiring `--yes` so a typo doesn't nuke a tool silently.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    tool_id: String,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!(
                "refusing to delete tool {tool_id} without --yes (deletion is irreversible)"
            ),
            suggestion: None,
        });
    }
    let url = format!("/v1/convai/tools/{tool_id}");
    client.delete(&url).await?;
    let result = serde_json::json!({ "tool_id": tool_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted tool {}", "-".red(), tool_id.dimmed());
    })?;
    Ok(())
}
