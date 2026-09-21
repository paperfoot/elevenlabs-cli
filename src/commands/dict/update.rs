//! `dict update ID` — update name/description or archive a dictionary.
//!
//! PATCH /v1/pronunciation-dictionaries/{id}
//!
//! Archiving is destructive-ish (the dictionary is hidden from default listings
//! and cannot be attached to new TTS jobs) but it is reversible server-side, so
//! we do NOT require a `--yes` confirmation. The `--archive` flag is surfaced
//! in `--help` so agents know what it does.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    id: String,
    name: Option<String>,
    description: Option<String>,
    archive: bool,
) -> Result<(), AppError> {
    if name.is_none() && description.is_none() && !archive {
        return Err(AppError::InvalidInput {
            msg: "nothing to update — pass --name, --description, or --archive".into(),
            suggestion: None,
        });
    }

    let mut body = serde_json::Map::new();
    if let Some(n) = name {
        body.insert("name".into(), serde_json::Value::String(n));
    }
    if let Some(d) = description {
        body.insert("description".into(), serde_json::Value::String(d));
    }
    if archive {
        body.insert("archived".into(), serde_json::Value::Bool(true));
    }

    let path = format!("/v1/pronunciation-dictionaries/{id}");
    let resp: serde_json::Value = client
        .patch_json(&path, &serde_json::Value::Object(body))
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} updated dictionary {} ({})",
            "~".yellow(),
            v.get("name").and_then(|x| x.as_str()).unwrap_or("").bold(),
            v.get("id").and_then(|x| x.as_str()).unwrap_or(&id).dimmed()
        );
        if archive {
            println!("  {}", "archived".yellow());
        }
    })?;
    Ok(())
}
