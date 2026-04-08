//! user info / subscription

use crate::cli::UserAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: UserAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        UserAction::Info => info(ctx, &client).await,
        UserAction::Subscription => subscription(ctx, &client).await,
    }
}

async fn info(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/user").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        if let Some(user_id) = v.get("user_id").and_then(|s| s.as_str()) {
            println!("{} {}", "user_id:".dimmed(), user_id);
        }
        if let Some(first) = v.get("first_name").and_then(|s| s.as_str()) {
            println!("{} {}", "name:".dimmed(), first);
        }
        if let Some(tier) = v
            .get("subscription")
            .and_then(|s| s.get("tier"))
            .and_then(|t| t.as_str())
        {
            println!("{} {}", "tier:".dimmed(), tier);
        }
    });
    Ok(())
}

async fn subscription(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/user/subscription").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let get = |k: &str| v.get(k).cloned();
        println!("{}", "Subscription".bold());
        if let Some(tier) = get("tier") {
            println!("  {} {}", "tier:".dimmed(), tier);
        }
        if let Some(status) = get("status") {
            println!("  {} {}", "status:".dimmed(), status);
        }
        if let Some(used) = get("character_count") {
            if let Some(limit) = get("character_limit") {
                println!("  {} {} / {}", "characters:".dimmed(), used, limit);
            }
        }
        if let Some(reset) = get("next_character_count_reset_unix") {
            println!("  {} {}", "reset_unix:".dimmed(), reset);
        }
    });
    Ok(())
}
