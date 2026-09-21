//! `dict add-rules-to ID --rule ...` — append one or more rules to an
//! existing dictionary (server creates a new version).
//!
//! POST /v1/pronunciation-dictionaries/{id}/add-rules
//! Body: { rules: [ … ] }

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    id: String,
    rules: Vec<String>,
    alias_rules: Vec<String>,
) -> Result<(), AppError> {
    let rules_json = super::collect_rules(rules, alias_rules)?;
    let rule_count = rules_json.len();
    let body = serde_json::json!({ "rules": rules_json });

    let path = format!("/v1/pronunciation-dictionaries/{id}/add-rules");
    let resp: serde_json::Value = client.post_json(&path, &body).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} appended {} rules to {}",
            "+".green(),
            rule_count,
            v.get("id").and_then(|x| x.as_str()).unwrap_or(&id).dimmed()
        );
        if let Some(ver) = v.get("version_id").and_then(|x| x.as_str()) {
            println!("  new version: {ver}");
        }
    })?;
    Ok(())
}
