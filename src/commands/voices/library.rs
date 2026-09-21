//! voices library — `/v1/shared-voices` (0-indexed pagination).

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub struct LibraryArgs {
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
    pub category: Option<String>,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub accent: Option<String>,
    pub language: Option<String>,
    pub locale: Option<String>,
    pub use_case: Option<String>,
    pub featured: bool,
    pub min_notice_days: Option<u32>,
    pub include_custom_rates: bool,
    pub include_live_moderated: bool,
    pub reader_app_enabled: bool,
    pub owner_id: Option<String>,
    pub sort: Option<String>,
}

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: LibraryArgs) -> Result<(), AppError> {
    // The API uses zero-based pages; page 0 contains the first results.
    let mut params: Vec<(&str, String)> = vec![
        ("page", args.page.to_string()),
        ("page_size", args.page_size.to_string()),
    ];
    if let Some(s) = args.search {
        params.push(("search", s));
    }
    if let Some(v) = args.category {
        params.push(("category", v));
    }
    if let Some(v) = args.gender {
        params.push(("gender", v));
    }
    if let Some(v) = args.age {
        params.push(("age", v));
    }
    if let Some(v) = args.accent {
        params.push(("accent", v));
    }
    if let Some(v) = args.language {
        params.push(("language", v));
    }
    if let Some(v) = args.locale {
        params.push(("locale", v));
    }
    if let Some(v) = args.use_case {
        params.push(("use_cases", v));
    }
    if args.featured {
        params.push(("featured", "true".to_string()));
    }
    if let Some(v) = args.min_notice_days {
        params.push(("min_notice_period_days", v.to_string()));
    }
    if args.include_custom_rates {
        params.push(("include_custom_rates", "true".to_string()));
    }
    if args.include_live_moderated {
        params.push(("include_live_moderated", "true".to_string()));
    }
    if args.reader_app_enabled {
        params.push(("reader_app_enabled", "true".to_string()));
    }
    if let Some(v) = args.owner_id {
        params.push(("owner_id", v));
    }
    if let Some(v) = args.sort {
        params.push(("sort", v));
    }

    let resp: serde_json::Value = client
        .get_json_with_query("/v1/shared-voices", &params)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        let list = v
            .get("voices")
            .and_then(|vs| vs.as_array())
            .cloned()
            .unwrap_or_default();
        if list.is_empty() {
            println!("(no shared voices match)");
            return;
        }
        use owo_colors::OwoColorize;
        let mut t = comfy_table::Table::new();
        t.set_header(vec![
            "Voice ID",
            "Name",
            "Public User",
            "Gender",
            "Age",
            "Accent",
            "Use Case",
        ]);
        for voice in &list {
            t.add_row(vec![
                voice
                    .get("voice_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                voice
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                voice
                    .get("public_owner_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                voice
                    .get("gender")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("age")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("accent")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("use_case")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
            ]);
        }
        println!("{t}");
        println!(
            "\nAdd one to your library: {}",
            "elevenlabs voices add-shared <public_owner_id> <voice_id> --name <new_name>".bold()
        );
    })?;
    Ok(())
}
