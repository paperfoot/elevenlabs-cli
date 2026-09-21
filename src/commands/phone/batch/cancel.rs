//! `phone batch cancel` — POST /v1/convai/batch-calling/{id}/cancel
//!
//! Cancel is reversible via `phone batch retry`, so this does not require
//! `--yes`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, batch_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/batch-calling/{batch_id}/cancel");
    let body = serde_json::json!({});
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    let result = serde_json::json!({
        "batch_id": batch_id,
        "cancelled": true,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} cancelled batch {}", "~".yellow(), batch_id.dimmed());
    })?;
    Ok(())
}
