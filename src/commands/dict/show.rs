//! `dict show ID` — full detail blob for one pronunciation dictionary.
//!
//! GET /v1/pronunciation-dictionaries/{id}

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, id: &str) -> Result<(), AppError> {
    let path = format!("/v1/pronunciation-dictionaries/{id}");
    let resp: serde_json::Value = client.get_json(&path).await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })?;
    Ok(())
}
