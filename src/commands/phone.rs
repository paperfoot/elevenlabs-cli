//! phone: list numbers / make outbound calls

use crate::cli::PhoneAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: PhoneAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        PhoneAction::List => list(ctx, &client).await,
        PhoneAction::Call {
            agent_id,
            from_id,
            to,
        } => call(ctx, &client, agent_id, from_id, to).await,
    }
}

async fn list(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    // Endpoint returns a bare JSON array in most versions.
    let resp: serde_json::Value = client.get_json("/v1/convai/phone-numbers").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let list = v
            .as_array()
            .cloned()
            .or_else(|| v.get("phone_numbers").and_then(|p| p.as_array()).cloned())
            .unwrap_or_default();
        if list.is_empty() {
            println!("(no phone numbers)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Phone", "Phone ID", "Provider", "Label", "Agent"]);
        for p in &list {
            let assigned = p
                .get("assigned_agent")
                .and_then(|a| a.get("agent_name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            t.add_row(vec![
                p.get("phone_number")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                p.get("phone_number_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                p.get("provider")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                p.get("label").and_then(|x| x.as_str()).unwrap_or("").into(),
                assigned.into(),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}

async fn call(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    from_id: String,
    to: String,
) -> Result<(), AppError> {
    // Determine provider type by looking up the phone.
    let list: serde_json::Value = client.get_json("/v1/convai/phone-numbers").await?;
    let arr = list
        .as_array()
        .cloned()
        .or_else(|| {
            list.get("phone_numbers")
                .and_then(|p| p.as_array())
                .cloned()
        })
        .unwrap_or_default();
    let phone = arr.iter().find(|p| {
        p.get("phone_number_id")
            .and_then(|v| v.as_str())
            .map(|s| s == from_id)
            .unwrap_or(false)
    });
    let provider = phone
        .and_then(|p| p.get("provider"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_lowercase();

    let path = match provider.as_str() {
        "twilio" => "/v1/convai/twilio/outbound-call",
        "sip_trunk" => "/v1/convai/sip-trunk/outbound-call",
        "" => {
            return Err(AppError::InvalidInput(format!(
                "phone number {from_id} not found in your account"
            )));
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "unsupported phone provider: {other}"
            )));
        }
    };

    let body = serde_json::json!({
        "agent_id": agent_id,
        "agent_phone_number_id": from_id,
        "to_number": to,
    });
    let resp: serde_json::Value = client.post_json(path, &body).await?;
    let result = serde_json::json!({
        "provider": provider,
        "agent_id": agent_id,
        "from_phone_number_id": from_id,
        "to": to,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} call placed via {} to {}",
            "+".green(),
            r["provider"].as_str().unwrap_or("").bold(),
            r["to"].as_str().unwrap_or("").bold()
        );
    });
    Ok(())
}
