//! `phone batch list` — GET /v1/convai/batch-calling/workspace
//!
//! The SDK (elevenlabs-python batch_calls/raw_client.py `list`) only
//! supports `limit` and `last_doc` query params. The CLI's user-facing
//! flag names stay `--page-size` and `--cursor` for ergonomic consistency
//! with every other `list` command in this tree, but we map internally
//! to the SDK-correct names.
//!
//! The pre-v0.2.0 CLI also exposed `--status` and `--agent-id` filters.
//! Those do not exist server-side, so we accept the options in the
//! function signature (to avoid breaking the dispatch layer) but silently
//! ignore them. The flags should be removed from `src/cli.rs` — see
//! `plans/cli-snippets/fixes/gamma/NOTES.md`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    page_size: Option<u32>,
    cursor: Option<String>,
    _status: Option<String>,
    _agent_id: Option<String>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(ps) = page_size {
        // SDK field: `limit`. User-facing flag stays `--page-size`.
        params.push(("limit", ps.min(100).to_string()));
    }
    if let Some(c) = cursor {
        // SDK field: `last_doc`. User-facing flag stays `--cursor`.
        params.push(("last_doc", c));
    }
    // `_status` and `_agent_id` are intentionally dropped — the
    // workspace listing endpoint has no server-side filters per SDK.

    let resp: serde_json::Value = client
        .get_json_with_query("/v1/convai/batch-calling/workspace", &params)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("batch_calls")
            .or_else(|| v.get("batches"))
            .and_then(|d| d.as_array())
            .cloned()
            .or_else(|| v.as_array().cloned())
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no batches)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec![
            "Batch ID",
            "Name",
            "Status",
            "Agent",
            "Total",
            "Completed",
            "Created",
        ]);
        for b in &items {
            t.add_row(vec![
                b.get("id")
                    .or_else(|| b.get("batch_id"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                b.get("call_name")
                    .or_else(|| b.get("name"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                b.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                b.get("agent_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                b.get("total_calls_dispatched")
                    .or_else(|| b.get("total"))
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                b.get("total_calls_completed")
                    .or_else(|| b.get("completed"))
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                b.get("created_at_unix")
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            ]);
        }
        println!("{t}");
        // SDK's response field for the next page token is `last_doc`.
        if let Some(next) = v
            .get("next_doc")
            .or_else(|| v.get("last_doc"))
            .or_else(|| v.get("next_cursor"))
            .and_then(|x| x.as_str())
        {
            if !next.is_empty() {
                println!("\nnext page: --cursor {next}");
            }
        }
    })?;
    Ok(())
}
