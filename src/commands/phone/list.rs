//! `phone list` — GET /v1/convai/phone-numbers

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    // Endpoint returns a bare JSON array in most versions; some responses wrap
    // it in `{ "phone_numbers": [...] }`. Handle both shapes.
    let resp: serde_json::Value = client.get_json("/v1/convai/phone-numbers").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let list = v
            .as_array()
            .cloned()
            .or_else(|| v.get("phone_numbers").and_then(|p| p.as_array()).cloned())
            .unwrap_or_default();
        if list.is_empty() {
            println!("(no phone numbers)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Phone", "Phone ID", "Provider", "Label", "Agent"]);
        for p in &list {
            let assigned = p
                .get("assigned_agent")
                .and_then(|a| a.get("agent_name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            t.add_row(vec![
                p.get("phone_number")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                p.get("phone_number_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                p.get("provider")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                p.get("label").and_then(|x| x.as_str()).unwrap_or("").into(),
                assigned.into(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
