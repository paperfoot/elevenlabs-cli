//! generation history: list / delete

use crate::cli::HistoryAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: HistoryAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        HistoryAction::List { page_size } => list(ctx, &client, page_size).await,
        HistoryAction::Delete { history_item_id } => delete(ctx, &client, &history_item_id).await,
    }
}

async fn list(ctx: Ctx, client: &ElevenLabsClient, page_size: u32) -> Result<(), AppError> {
    let params = [("page_size", page_size.min(1000).to_string())];
    let resp: serde_json::Value = client.get_json_with_query("/v1/history", &params).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .get("history")
            .and_then(|h| h.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            println!("(empty history)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["History ID", "Voice", "Text", "Chars", "Date"]);
        for it in &items {
            let text = it
                .get("text")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            t.add_row(vec![
                it.get("history_item_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                it.get("voice_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                text,
                it.get("character_count_change_from")
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                it.get("date_unix")
                    .and_then(|x| x.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}

async fn delete(
    ctx: Ctx,
    client: &ElevenLabsClient,
    history_item_id: &str,
) -> Result<(), AppError> {
    let path = format!("/v1/history/{history_item_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "history_item_id": history_item_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!(
            "{} deleted history item {}",
            "-".red(),
            history_item_id.dimmed()
        );
    });
    Ok(())
}
