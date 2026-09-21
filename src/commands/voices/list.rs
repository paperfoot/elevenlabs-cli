//! voices list — `/v2/voices` with the full filter/sort/pagination set.
//!
//! The v2 endpoint is the only one that honours search/sort/page_size and
//! exposes the newer `voice_type` taxonomy (including `non-community`, added
//! Apr 13, 2026).

use serde::{Deserialize, Serialize};

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct VoiceSummary {
    voice_id: String,
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    preview_url: Option<String>,
    #[serde(default)]
    is_bookmarked: Option<bool>,
}

pub struct ListArgs {
    pub search: Option<String>,
    pub sort: String,
    pub direction: String,
    pub limit: u32,
    pub next_page_token: Option<String>,
    pub voice_type: Option<String>,
    pub category: Option<String>,
    pub fine_tuning_state: Option<String>,
    pub collection_id: Option<String>,
    pub include_total_count: bool,
    pub voice_ids: Vec<String>,
}

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: ListArgs) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = vec![
        ("sort", args.sort),
        ("sort_direction", args.direction),
        ("page_size", args.limit.to_string()),
    ];
    if let Some(s) = args.search {
        params.push(("search", s));
    }
    if let Some(t) = args.next_page_token {
        params.push(("next_page_token", t));
    }
    if let Some(vt) = args.voice_type {
        params.push(("voice_type", vt));
    }
    if let Some(c) = args.category {
        params.push(("category", c));
    }
    if let Some(s) = args.fine_tuning_state {
        params.push(("fine_tuning_state", s));
    }
    if let Some(c) = args.collection_id {
        params.push(("collection_id", c));
    }
    if args.include_total_count {
        params.push(("include_total_count", "true".to_string()));
    }
    for id in args.voice_ids {
        // The v2 endpoint accepts repeated `voice_ids` params. Reqwest serialises
        // a list-of-tuples with repeated keys exactly right.
        params.push(("voice_ids", id));
    }

    let resp: serde_json::Value = client.get_json_with_query("/v2/voices", &params).await?;
    let voices: Vec<VoiceSummary> = resp
        .get("voices")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    output::print_success_or(ctx, &voices, |list| {
        use owo_colors::OwoColorize;
        if list.is_empty() {
            println!("(no voices)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Voice ID", "Name", "Category", "Bookmarked"]);
        for v in list {
            t.add_row(vec![
                v.voice_id.dimmed().to_string(),
                v.name.bold().to_string(),
                v.category.clone().unwrap_or_default(),
                match v.is_bookmarked {
                    Some(true) => "yes".into(),
                    Some(false) => "no".into(),
                    None => "".into(),
                },
            ]);
        }
        println!("{t}");
    });
    Ok(())
}
