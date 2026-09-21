//! `agents list` — enumerate conversational AI agents in your workspace.

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/convai/agents").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let agents = v
            .get("agents")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        if agents.is_empty() {
            println!("(no agents)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Agent ID", "Name"]);
        for a in &agents {
            t.add_row(vec![
                a.get("agent_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                a.get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
