//! `phone batch delete` — DELETE /v1/convai/batch-calling/{id}
//!
//! Irreversible. Requires `--yes`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    batch_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!("deleting batch '{batch_id}' is irreversible — pass --yes to confirm"),
            suggestion: None,
        });
    }
    let path = format!("/v1/convai/batch-calling/{batch_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "batch_id": batch_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted batch {}", "-".red(), batch_id.dimmed());
    })?;
    Ok(())
}
