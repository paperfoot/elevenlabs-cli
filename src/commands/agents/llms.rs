//! `agents llms` — GET /v1/convai/llm/list
//!
//! Returns the LLMs the Agents backend currently accepts for
//! conversation_config.agent.prompt.llm. This is the safety net for the
//! "--llm accepts any string but the server silently fails at conversation
//! time" footgun — call this once before `agents create --llm …` to pin
//! down a value that's guaranteed to work.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/convai/llm/list").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        // Response shape per spec: { llms: [{ llm_id, display_name, ... }] }.
        // Older responses may return a raw array — handle both.
        let items = v
            .get("llms")
            .and_then(|l| l.as_array())
            .cloned()
            .or_else(|| v.as_array().cloned())
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no LLMs listed)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["LLM ID", "Display name", "Notes"]);
        for item in &items {
            let id = item
                .get("llm_id")
                .or_else(|| item.get("id"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let display = item
                .get("display_name")
                .or_else(|| item.get("name"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let mut notes = Vec::new();
            if item
                .get("requires_custom_llm")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
            {
                notes.push("custom_llm only".to_string());
            }
            if let Some(tag) = item.get("provider").and_then(|x| x.as_str()) {
                notes.push(tag.to_string());
            }
            t.add_row(vec![
                id.bold().to_string(),
                display.to_string(),
                notes.join(", "),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}
