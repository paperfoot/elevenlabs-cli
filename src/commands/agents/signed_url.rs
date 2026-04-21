//! `agents signed-url <agent_id>` — GET /v1/convai/conversation/get-signed-url
//!
//! Returns a short-lived signed URL that can be embedded in a widget / web
//! session without manually managing auth tokens. The server adds the
//! `agent_id` as a query parameter and returns `{ signed_url }`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, agent_id: &str) -> Result<(), AppError> {
    let resp: serde_json::Value = client
        .get_json_with_query(
            "/v1/convai/conversation/get-signed-url",
            &[("agent_id", agent_id)],
        )
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        if let Some(url) = v.get("signed_url").and_then(|x| x.as_str()) {
            println!("{} {}", "+".green(), url.bold());
        } else {
            println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
        }
    });
    Ok(())
}
