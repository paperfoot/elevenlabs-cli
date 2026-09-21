//! `agents tools list`

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let resp: serde_json::Value = client.get_json("/v1/convai/tools").await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let tools = v
            .get("tools")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        if tools.is_empty() {
            println!("(no tools)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Tool ID", "Name", "Type"]);
        for tool in &tools {
            // API returns { id, tool_config: { name, description, type, ... } }
            // (see elevenlabs-python raw_client). Tolerate either shape.
            let id = tool
                .get("id")
                .or_else(|| tool.get("tool_id"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let cfg = tool.get("tool_config").unwrap_or(tool);
            let name = cfg.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let kind = cfg.get("type").and_then(|x| x.as_str()).unwrap_or("");
            t.add_row(vec![
                id.dimmed().to_string(),
                name.bold().to_string(),
                kind.to_string(),
            ]);
        }
        println!("{t}");
    })?;
    Ok(())
}
