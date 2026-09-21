//! `dict set-rules ID --rule ... [--case-sensitive] [--word-boundaries]` —
//! replace every rule in a dictionary with the given set.
//!
//! POST /v1/pronunciation-dictionaries/{id}/set-rules
//! Body: { rules: [ … ], case_sensitive?: bool, word_boundaries?: bool }

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    id: String,
    rules: Vec<String>,
    alias_rules: Vec<String>,
    case_sensitive: Option<bool>,
    word_boundaries: Option<bool>,
) -> Result<(), AppError> {
    let rules_json = super::collect_rules(rules, alias_rules)?;

    let mut body = serde_json::Map::new();
    body.insert("rules".into(), serde_json::Value::Array(rules_json));
    if let Some(cs) = case_sensitive {
        body.insert("case_sensitive".into(), serde_json::Value::Bool(cs));
    }
    if let Some(wb) = word_boundaries {
        body.insert("word_boundaries".into(), serde_json::Value::Bool(wb));
    }

    let path = format!("/v1/pronunciation-dictionaries/{id}/set-rules");
    let resp: serde_json::Value = client
        .post_json(&path, &serde_json::Value::Object(body))
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} replaced rules for {}",
            "~".yellow(),
            v.get("id").and_then(|x| x.as_str()).unwrap_or(&id).dimmed()
        );
        if let Some(ver) = v.get("version_id").and_then(|x| x.as_str()) {
            println!("  new version: {ver}");
        }
    })?;
    Ok(())
}
