//! POST /v1/dubbing/resource/{id}/dub

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    dubbing_id: &str,
    patch: Option<String>,
) -> Result<(), AppError> {
    let body = super::super::load_patch_body(patch).await?;
    let path = format!("/v1/dubbing/resource/{dubbing_id}/dub");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} re-dub queued for dub {}",
            "+".green(),
            dubbing_id.dimmed()
        );
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })?;
    Ok(())
}
