//! `dict remove-rules ID --word foo [--word bar …]` — drop the rule(s) that
//! match the given `string_to_replace` values. Repeatable.
//!
//! POST /v1/pronunciation-dictionaries/{id}/remove-rules
//! Body: { rule_strings: [ "foo", "bar", … ] }

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    id: String,
    words: Vec<String>,
) -> Result<(), AppError> {
    if words.is_empty() {
        return Err(AppError::InvalidInput {
            msg: "pass at least one --word <string_to_replace> to remove".into(),
            suggestion: None,
        });
    }
    let trimmed: Vec<String> = words
        .into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            msg: "all --word values were empty after trimming".into(),
            suggestion: None,
        });
    }
    let count = trimmed.len();
    let body = serde_json::json!({ "rule_strings": trimmed });

    let path = format!("/v1/pronunciation-dictionaries/{id}/remove-rules");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} removed {} rules from {}",
            "-".red(),
            count,
            v.get("id").and_then(|x| x.as_str()).unwrap_or(&id).dimmed()
        );
        if let Some(ver) = v.get("version_id").and_then(|x| x.as_str()) {
            println!("  new version: {ver}");
        }
    })?;
    Ok(())
}
