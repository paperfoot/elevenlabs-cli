//! voices delete — DELETE /v1/voices/{voice_id}. Destructive: --yes required.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    voice_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!("deleting '{voice_id}' is irreversible — pass --yes to confirm"),
            suggestion: None,
        });
    }
    let path = format!("/v1/voices/{voice_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "voice_id": voice_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted voice {}", "-".red(), voice_id.dimmed());
    })?;
    Ok(())
}
