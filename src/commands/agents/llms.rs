//! `agents llms` — GET /v1/convai/llm/list
//!
//! Returns the LLMs the Agents backend currently accepts for
//! conversation_config.agent.prompt.llm. This is the safety net for the
//! "--llm accepts any string but the server silently fails at conversation
//! time" footgun — call this once before `agents create --llm …` to pin
//! down a value that's guaranteed to work.
//!
//! Response shape (LLMListResponseModel in the OpenAPI spec):
//!   {
//!     "llms": [
//!       { "llm": "gemini-2.5-flash",
//!         "is_checkpoint": false,
//!         "max_tokens_limit": 8192,
//!         "max_context_limit": 1048576,
//!         "supports_image_input": true,
//!         "supports_document_input": true,
//!         "supports_parallel_tool_calls": true,
//!         "available_reasoning_efforts": null,
//!         "deprecation_info": null },
//!       ...
//!     ],
//!     "default_deprecation_config": { ... }
//!   }

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/convai/llm/list").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("llms")
            .and_then(|l| l.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no LLMs listed)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec![
            "LLM",
            "Ctx (tok)",
            "Max out",
            "Img",
            "Doc",
            "Parallel tools",
            "Reasoning",
            "Status",
        ]);
        for item in &items {
            let id = item.get("llm").and_then(|x| x.as_str()).unwrap_or("");
            let ctx_tok = item
                .get("max_context_limit")
                .and_then(|x| x.as_u64())
                .map(format_tokens)
                .unwrap_or_default();
            let max_out = item
                .get("max_tokens_limit")
                .and_then(|x| x.as_u64())
                .map(format_tokens)
                .unwrap_or_default();
            let img = mark_bool(item.get("supports_image_input"));
            let doc = mark_bool(item.get("supports_document_input"));
            let parallel = mark_bool(item.get("supports_parallel_tool_calls"));
            let reasoning = item
                .get("available_reasoning_efforts")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let status = if item
                .get("deprecation_info")
                .map(|d| !d.is_null())
                .unwrap_or(false)
            {
                "deprecated".yellow().to_string()
            } else if item
                .get("is_checkpoint")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
            {
                "checkpoint".dimmed().to_string()
            } else {
                String::new()
            };
            t.add_row(vec![
                id.bold().to_string(),
                ctx_tok,
                max_out,
                img.to_string(),
                doc.to_string(),
                parallel.to_string(),
                reasoning,
                status,
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

fn mark_bool(v: Option<&serde_json::Value>) -> &'static str {
    match v.and_then(|x| x.as_bool()) {
        Some(true) => "y",
        Some(false) => " ",
        None => "?",
    }
}
