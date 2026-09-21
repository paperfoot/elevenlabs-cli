//! voices save-preview — POST /v1/text-to-voice.
//!
//! Persists a designed preview to the caller's voice library. v0.1.1 hit
//! `/v1/text-to-voice/create-voice-from-preview` which 404s; the correct
//! path is `/v1/text-to-voice` (see elevenlabs-python text_to_voice/raw_client.py).

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    generated_voice_id: String,
    voice_name: String,
    voice_description: String,
) -> Result<(), AppError> {
    let body = serde_json::json!({
        "generated_voice_id": generated_voice_id,
        "voice_name": voice_name,
        "voice_description": voice_description,
    });
    let resp: serde_json::Value = client.post_json("/v1/text-to-voice", &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} saved voice: {} ({})",
            "+".green(),
            v.get("name").and_then(|x| x.as_str()).unwrap_or("").bold(),
            v.get("voice_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    })?;
    Ok(())
}
