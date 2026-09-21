//! voices show — `/v1/voices/{voice_id}`.
//!
//! Response passes through as JSON. The human rendering surfaces the Apr 7
//! 2026 additions (is_bookmarked, recording_quality, labelling_status) when
//! the server populates them.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, voice_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/voices/{voice_id}");
    let resp: serde_json::Value = client.get_json(&path).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let name = v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("(unnamed)");
        let id = v.get("voice_id").and_then(|n| n.as_str()).unwrap_or("");
        let category = v.get("category").and_then(|n| n.as_str()).unwrap_or("");
        let description = v.get("description").and_then(|n| n.as_str()).unwrap_or("");

        println!("{} {}", name.bold(), format!("({id})").dimmed());
        if !category.is_empty() {
            println!("  category: {category}");
        }
        if !description.is_empty() {
            println!("  description: {description}");
        }

        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Field", "Value"]);
        let mut rows: Vec<(&str, String)> = Vec::new();

        if let Some(b) = v.get("is_bookmarked").and_then(|x| x.as_bool()) {
            rows.push(("is_bookmarked", b.to_string()));
        }
        if let Some(rq) = v.get("recording_quality").and_then(|x| x.as_str()) {
            rows.push(("recording_quality", rq.to_string()));
        }
        if let Some(ls) = v.get("labelling_status").and_then(|x| x.as_str()) {
            rows.push(("labelling_status", ls.to_string()));
        }
        if let Some(rqr) = v.get("recording_quality_reason").and_then(|x| x.as_str()) {
            if !rqr.is_empty() {
                rows.push(("recording_quality_reason", rqr.to_string()));
            }
        }
        if let Some(labels) = v.get("labels").and_then(|x| x.as_object()) {
            for (k, lv) in labels {
                rows.push(("label", format!("{k}={}", lv.as_str().unwrap_or(""))));
            }
        }
        if !rows.is_empty() {
            for (k, val) in rows {
                t.add_row(vec![k.to_string(), val]);
            }
            println!("{t}");
        }

        // Full JSON at the bottom for anything we don't render.
        println!();
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    })?;
    Ok(())
}
