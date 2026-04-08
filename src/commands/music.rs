//! music compose / plan

use crate::cli::MusicAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn dispatch(ctx: Ctx, action: MusicAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        MusicAction::Compose {
            prompt,
            length_ms,
            output,
        } => compose(ctx, &client, prompt, length_ms, output).await,
        MusicAction::Plan { prompt, length_ms } => plan(ctx, &client, prompt, length_ms).await,
    }
}

async fn compose(
    ctx: Ctx,
    client: &ElevenLabsClient,
    prompt: String,
    length_ms: Option<u32>,
    output: Option<String>,
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::InvalidInput("prompt is empty".into()));
    }
    let mut body = serde_json::Map::new();
    body.insert("prompt".into(), serde_json::Value::String(prompt.clone()));
    if let Some(ms) = length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }

    // POST /v1/music streams back audio bytes directly.
    let audio = client
        .post_json_bytes("/v1/music", &serde_json::Value::Object(body))
        .await?;
    let bytes_written = audio.len();

    let out_path = crate::commands::resolve_output_path(output, "music", "mp3");
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "prompt": prompt,
        "length_ms": length_ms,
        "output": out_path.display().to_string(),
        "bytes_written": bytes_written,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0
        );
    });
    Ok(())
}

async fn plan(
    ctx: Ctx,
    client: &ElevenLabsClient,
    prompt: String,
    length_ms: Option<u32>,
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::InvalidInput("prompt is empty".into()));
    }
    let mut body = serde_json::Map::new();
    body.insert("prompt".into(), serde_json::Value::String(prompt));
    if let Some(ms) = length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }
    let resp: serde_json::Value = client
        .post_json("/v1/music/plan", &serde_json::Value::Object(body))
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}
