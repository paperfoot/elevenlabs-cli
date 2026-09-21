//! `dubbing list` — GET /v1/dubbing
//!
//! Supports filters `dubbing_status`, `filter_by_creator`, `page_size`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    dubbing_status: Option<String>,
    filter_by_creator: Option<String>,
    page_size: Option<u32>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(v) = dubbing_status {
        params.push(("dubbing_status", v));
    }
    if let Some(v) = filter_by_creator {
        params.push(("filter_by_creator", v));
    }
    if let Some(v) = page_size {
        params.push(("page_size", v.min(100).to_string()));
    }

    let resp: serde_json::Value = client.get_json_with_query("/v1/dubbing", &params).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("dubs")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no dubs)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Dubbing ID", "Name", "Status", "Target", "Created"]);
        for d in &items {
            t.add_row(vec![
                d.get("dubbing_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                d.get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                d.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                d.get("target_languages")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default(),
                d.get("created_at_unix")
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
