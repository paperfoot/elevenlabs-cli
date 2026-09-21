//! `phone whatsapp accounts list` — GET /v1/convai/whatsapp-accounts

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/convai/whatsapp-accounts").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let items = v
            .as_array()
            .cloned()
            .or_else(|| {
                v.get("whatsapp_accounts")
                    .or_else(|| v.get("accounts"))
                    .and_then(|a| a.as_array())
                    .cloned()
            })
            .unwrap_or_default();
        if items.is_empty() {
            println!("(no WhatsApp accounts)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Account ID", "Display Name", "Phone", "Status"]);
        for a in &items {
            t.add_row(vec![
                a.get("id")
                    .or_else(|| a.get("account_id"))
                    .or_else(|| a.get("whatsapp_account_id"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                a.get("display_name")
                    .or_else(|| a.get("name"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                a.get("phone_number")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                a.get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
