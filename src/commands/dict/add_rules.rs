//! `dict add-rules NAME --rule WORD:PHONEME ...` — create a pronunciation
//! dictionary in-line from one or more rules (no file upload).
//!
//! POST /v1/pronunciation-dictionaries/add-from-rules
//! Body: { name, description?, workspace_access?, rules: [ … ] }

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    name: String,
    description: Option<String>,
    workspace_access: Option<String>,
    rules: Vec<String>,
    alias_rules: Vec<String>,
) -> Result<(), AppError> {
    let rules_json = super::collect_rules(rules, alias_rules)?;

    let mut body = serde_json::Map::new();
    body.insert("name".into(), serde_json::Value::String(name.clone()));
    body.insert("rules".into(), serde_json::Value::Array(rules_json));
    if let Some(d) = description {
        body.insert("description".into(), serde_json::Value::String(d));
    }
    if let Some(wa) = workspace_access {
        body.insert("workspace_access".into(), serde_json::Value::String(wa));
    }

    let resp: serde_json::Value = client
        .post_json(
            "/v1/pronunciation-dictionaries/add-from-rules",
            &serde_json::Value::Object(body),
        )
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} created dictionary: {} ({})",
            "+".green(),
            v.get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&name)
                .bold(),
            v.get("id").and_then(|x| x.as_str()).unwrap_or("").dimmed()
        );
        if let Some(ver) = v.get("version_id").and_then(|x| x.as_str()) {
            println!("  version: {ver}");
        }
    })?;
    Ok(())
}
