//! `phone whatsapp call` — POST /v1/convai/whatsapp/outbound-call
//!
//! Body shape (matches elevenlabs-python raw_client exactly):
//!
//!   {
//!     "whatsapp_phone_number_id": "<sending business number id>",
//!     "whatsapp_user_id": "<recipient user id>",
//!     "whatsapp_call_permission_request_template_name": "<name>",
//!     "whatsapp_call_permission_request_template_language_code": "<e.g. en_US>",
//!     "agent_id": "<agent id>"
//!   }
//!
//! WhatsApp requires the recipient to have previously approved a
//! call-permission-request template before an outbound voice call is
//! allowed. There is no free-form call path — template is always required.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    whatsapp_phone_number_id: String,
    whatsapp_user_id: String,
    permission_template_name: String,
    permission_template_language_code: String,
) -> Result<(), AppError> {
    let body = serde_json::json!({
        "whatsapp_phone_number_id": whatsapp_phone_number_id,
        "whatsapp_user_id": whatsapp_user_id,
        "whatsapp_call_permission_request_template_name": permission_template_name,
        "whatsapp_call_permission_request_template_language_code": permission_template_language_code,
        "agent_id": agent_id,
    });
    let resp: serde_json::Value = client
        .post_json("/v1/convai/whatsapp/outbound-call", &body)
        .await?;
    let result = serde_json::json!({
        "agent_id": agent_id,
        "whatsapp_phone_number_id": whatsapp_phone_number_id,
        "whatsapp_user_id": whatsapp_user_id,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} WhatsApp call placed to user {}",
            "+".green(),
            r["whatsapp_user_id"].as_str().unwrap_or("").bold()
        );
    })?;
    Ok(())
}
