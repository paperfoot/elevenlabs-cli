//! POST /v1/dubbing/resource/{id}/migrate-segments
//!
//! Used to migrate legacy segment metadata to the current schema. Takes an
//! optional `--patch <PATH>` for any non-trivial body override.

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
    let path = format!("/v1/dubbing/resource/{dubbing_id}/migrate-segments");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} segment migration queued for dub {}",
            "+".green(),
            dubbing_id.dimmed()
        );
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })?;
    Ok(())
}
