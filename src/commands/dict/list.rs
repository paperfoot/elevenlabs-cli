//! `dict list` — enumerate pronunciation dictionaries.
//!
//! GET /v1/pronunciation-dictionaries
//! Filters: `cursor`, `page_size`, `search`.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    cursor: Option<String>,
    page_size: Option<u32>,
    search: Option<String>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = Vec::new();
    if let Some(c) = cursor {
        params.push(("cursor", c));
    }
    if let Some(n) = page_size {
        params.push(("page_size", n.to_string()));
    }
    if let Some(s) = search {
        params.push(("search", s));
    }

    let resp: serde_json::Value = client
        .get_json_with_query("/v1/pronunciation-dictionaries", &params)
        .await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("pronunciation_dictionaries")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no pronunciation dictionaries)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["ID", "Name", "Version", "Rules", "Archived"]);
        for it in &items {
            t.add_row(vec![
                it.get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                it.get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                it.get("latest_version_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                it.get("latest_version_rules_num")
                    .and_then(|x| x.as_u64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| {
                        it.get("rules_num")
                            .and_then(|x| x.as_u64())
                            .map(|n| n.to_string())
                            .unwrap_or_default()
                    }),
                it.get("archived_time_unix")
                    .map(|v| if v.is_null() { "" } else { "yes" })
                    .unwrap_or("")
                    .to_string(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
