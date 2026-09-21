//! `dubbing delete <dubbing_id> --yes` — DELETE /v1/dubbing/{id}

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    dubbing_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!("deleting '{dubbing_id}' is irreversible — pass --yes to confirm"),
            suggestion: None,
        });
    }
    let path = format!("/v1/dubbing/{dubbing_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "dubbing_id": dubbing_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted dubbing {}", "-".red(), dubbing_id.dimmed());
    })?;
    Ok(())
}
