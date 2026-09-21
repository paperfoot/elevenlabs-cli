//! voices add-shared — POST /v1/voices/add/{public_user_id}/{voice_id}.
//!
//! Clone a voice from the shared library into the caller's collection.
//! Grounded against elevenlabs-python voices/raw_client.py `share` method:
//! JSON body `{new_name, bookmarked?}`. Path segments MUST be the public
//! user id (not internal user_id) and the public voice_id.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    public_user_id: &str,
    voice_id: &str,
    new_name: String,
    bookmarked: Option<bool>,
) -> Result<(), AppError> {
    if public_user_id.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "public_user_id is required".into(),
            suggestion: None,
        });
    }
    if voice_id.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "voice_id is required".into(),
            suggestion: None,
        });
    }
    if new_name.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "--name is required (the name under which the shared voice will be saved)".into(),
            suggestion: None,
        });
    }

    let mut body = serde_json::Map::new();
    body.insert(
        "new_name".into(),
        serde_json::Value::String(new_name.clone()),
    );
    if let Some(b) = bookmarked {
        body.insert("bookmarked".into(), serde_json::Value::Bool(b));
    }

    let path = format!("/v1/voices/add/{public_user_id}/{voice_id}");
    let resp: serde_json::Value = client
        .post_json(&path, &serde_json::Value::Object(body))
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let saved_id = v
            .get("voice_id")
            .and_then(|x| x.as_str())
            .unwrap_or(voice_id);
        let saved_name = v.get("name").and_then(|x| x.as_str()).unwrap_or(&new_name);
        println!(
            "{} added shared voice: {} ({})",
            "+".green(),
            saved_name.bold(),
            saved_id.dimmed()
        );
    })?;
    Ok(())
}
