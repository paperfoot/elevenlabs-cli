//! Conversational AI agents: list / show / create / delete / add-knowledge.

use std::path::Path;

use crate::cli::AgentsAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: AgentsAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        AgentsAction::List => list(ctx, &client).await,
        AgentsAction::Show { agent_id } => show(ctx, &client, &agent_id).await,
        AgentsAction::Create {
            name,
            system_prompt,
            first_message,
            voice_id,
            language,
            llm,
            temperature,
            model_id,
        } => {
            create(
                ctx,
                &cfg,
                &client,
                name,
                system_prompt,
                first_message,
                voice_id,
                language,
                llm,
                temperature,
                model_id,
            )
            .await
        }
        AgentsAction::Delete { agent_id } => delete(ctx, &client, &agent_id).await,
        AgentsAction::AddKnowledge {
            agent_id,
            name,
            url,
            file,
            text,
        } => add_knowledge(ctx, &client, agent_id, name, url, file, text).await,
    }
}

async fn list(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
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
    });
    Ok(())
}

async fn show(ctx: Ctx, client: &ElevenLabsClient, agent_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/agents/{agent_id}");
    let resp: serde_json::Value = client.get_json(&path).await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    name: String,
    system_prompt: String,
    first_message: Option<String>,
    voice_id: Option<String>,
    language: String,
    llm: String,
    temperature: f32,
    model_id: String,
) -> Result<(), AppError> {
    let voice_id = voice_id.unwrap_or_else(|| cfg.default_voice_id());

    // Build the same conversation_config the MCP server uses.
    let conversation_config = serde_json::json!({
        "agent": {
            "language": language,
            "prompt": {
                "prompt": system_prompt,
                "llm": llm,
                "tools": [{"type": "system", "name": "end_call", "description": ""}],
                "knowledge_base": [],
                "temperature": temperature,
            },
            "first_message": first_message,
            "dynamic_variables": { "dynamic_variable_placeholders": {} }
        },
        "asr": {
            "quality": "high",
            "provider": "elevenlabs",
            "user_input_audio_format": "pcm_16000",
            "keywords": []
        },
        "tts": {
            "voice_id": voice_id,
            "model_id": model_id,
            "agent_output_audio_format": "pcm_16000",
            "optimize_streaming_latency": 3,
            "stability": 0.5,
            "similarity_boost": 0.8
        },
        "turn": { "turn_timeout": 7 },
        "conversation": {
            "max_duration_seconds": 300,
            "client_events": [
                "audio", "interruption", "user_transcript",
                "agent_response", "agent_response_correction"
            ]
        },
        "language_presets": {},
        "is_blocked_ivc": false,
        "is_blocked_non_ivc": false
    });

    let platform_settings = serde_json::json!({
        "widget": {
            "variant": "full",
            "avatar": { "type": "orb", "color_1": "#6DB035", "color_2": "#F5CABB" },
            "feedback_mode": "during"
        },
        "evaluation": {},
        "auth": { "allowlist": [] },
        "overrides": {},
        "call_limits": { "agent_concurrency_limit": -1, "daily_limit": 100000 },
        "privacy": {
            "record_voice": true,
            "retention_days": 730,
            "delete_transcript_and_pii": true,
            "delete_audio": true,
            "apply_to_existing_conversations": false
        },
        "data_collection": {}
    });

    let body = serde_json::json!({
        "name": name,
        "conversation_config": conversation_config,
        "platform_settings": platform_settings,
    });

    let resp: serde_json::Value = client.post_json("/v1/convai/agents/create", &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} created agent {} ({})",
            "+".green(),
            name.bold(),
            v.get("agent_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    });
    Ok(())
}

async fn delete(ctx: Ctx, client: &ElevenLabsClient, agent_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/convai/agents/{agent_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "agent_id": agent_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted agent {}", "-".red(), agent_id.dimmed());
    });
    Ok(())
}

async fn add_knowledge(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    name: String,
    url: Option<String>,
    file: Option<String>,
    text: Option<String>,
) -> Result<(), AppError> {
    let sources = [url.is_some(), file.is_some(), text.is_some()]
        .iter()
        .filter(|x| **x)
        .count();
    if sources == 0 {
        return Err(AppError::InvalidInput(
            "provide one of --url, --file, or --text".into(),
        ));
    }
    if sources > 1 {
        return Err(AppError::InvalidInput(
            "provide only one of --url, --file, or --text".into(),
        ));
    }

    let resp: serde_json::Value = if let Some(u) = url {
        let body = serde_json::json!({ "name": name, "url": u });
        client
            .post_json("/v1/convai/knowledge-base/url", &body)
            .await?
    } else if let Some(t) = text {
        let body = serde_json::json!({ "name": name, "text": t });
        client
            .post_json("/v1/convai/knowledge-base/text", &body)
            .await?
    } else {
        let f = file.unwrap();
        let path = Path::new(&f);
        if !path.exists() {
            return Err(AppError::InvalidInput(format!(
                "file does not exist: {}",
                path.display()
            )));
        }
        let bytes = crate::commands::read_file_bytes(path).await?;
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let mime = crate::commands::mime_for_path(path);
        let file_part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        let form = reqwest::multipart::Form::new()
            .text("name", name.clone())
            .part("file", file_part);
        client
            .post_multipart_json("/v1/convai/knowledge-base/file", form)
            .await?
    };

    let result = serde_json::json!({
        "agent_id": agent_id,
        "name": name,
        "document": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        let doc_id = r["document"]
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        println!(
            "{} added knowledge '{}' to agent {} (doc {})",
            "+".green(),
            r["name"].as_str().unwrap_or("").bold(),
            r["agent_id"].as_str().unwrap_or("").dimmed(),
            doc_id.dimmed()
        );
    });
    Ok(())
}
