//! conversations list / show

use crate::cli::ConversationsAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: ConversationsAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        ConversationsAction::List {
            agent_id,
            page_size,
            cursor,
        } => list(ctx, &client, agent_id, page_size, cursor).await,
        ConversationsAction::Show { conversation_id } => show(ctx, &client, &conversation_id).await,
    }
}

async fn list(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: Option<String>,
    page_size: u32,
    cursor: Option<String>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = vec![("page_size", page_size.min(100).to_string())];
    if let Some(a) = agent_id {
        params.push(("agent_id", a));
    }
    if let Some(c) = cursor {
        params.push(("cursor", c));
    }
    let resp: serde_json::Value = client
        .get_json_with_query("/v1/convai/conversations", &params)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let convs = v
            .get("conversations")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        if convs.is_empty() {
            println!("(no conversations)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Conv ID", "Agent", "Status", "Duration", "Messages"]);
        for c in &convs {
            t.add_row(vec![
                c.get("conversation_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                c.get("agent_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                c.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                c.get("call_duration_secs")
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                c.get("message_count")
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
            ]);
        }
        println!("{t}");
        if let Some(next) = v.get("next_cursor").and_then(|x| x.as_str()) {
            println!("{} --cursor {}", "more:".dimmed(), next);
        }
    });
    Ok(())
}

async fn show(ctx: Ctx, client: &ElevenLabsClient, conversation_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/conversations/{conversation_id}");
    let resp: serde_json::Value = client.get_json(&path).await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}
