//! `phone batch show` — GET /v1/convai/batch-calling/{id}
//!
//! Returns the full batch detail including per-call status.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, batch_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/batch-calling/{batch_id}");
    let resp: serde_json::Value = client.get_json(&path).await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })?;
    Ok(())
}
