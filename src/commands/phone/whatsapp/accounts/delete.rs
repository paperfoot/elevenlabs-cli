//! `phone whatsapp accounts delete` — DELETE /v1/convai/whatsapp-accounts/{id}

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    account_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput {
            msg: format!(
                "deleting WhatsApp account '{account_id}' is irreversible — pass --yes to confirm"
            ),
            suggestion: None,
        });
    }
    let path = format!("/v1/convai/whatsapp-accounts/{account_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({
        "whatsapp_account_id": account_id,
        "deleted": true,
    });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!(
            "{} deleted WhatsApp account {}",
            "-".red(),
            account_id.dimmed()
        );
    })?;
    Ok(())
}
