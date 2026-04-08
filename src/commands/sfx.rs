//! sound effects: POST /v1/sound-generation

use serde::Serialize;

use crate::cli::SfxArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct SfxResult {
    prompt: String,
    duration_seconds: Option<f32>,
    looping: bool,
    output_path: String,
    output_format: String,
    bytes_written: usize,
}

pub async fn run(ctx: Ctx, args: SfxArgs) -> Result<(), AppError> {
    if args.text.trim().is_empty() {
        return Err(AppError::InvalidInput("text is empty".into()));
    }
    if let Some(d) = args.duration {
        if !(0.5..=22.0).contains(&d) {
            return Err(AppError::InvalidInput(
                "duration must be between 0.5 and 22 seconds".into(),
            ));
        }
    }

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    let output_format = args
        .format
        .clone()
        .unwrap_or_else(|| cfg.default_output_format());

    let mut body = serde_json::Map::new();
    body.insert("text".into(), serde_json::Value::String(args.text.clone()));
    if let Some(d) = args.duration {
        body.insert("duration_seconds".into(), serde_json::json!(d));
    }
    if let Some(p) = args.prompt_influence {
        body.insert("prompt_influence".into(), serde_json::json!(p));
    }
    body.insert("loop".into(), serde_json::json!(args.looping));

    let query = [("output_format", output_format.as_str())];
    let audio = client
        .post_json_bytes_with_query(
            "/v1/sound-generation",
            &query,
            &serde_json::Value::Object(body),
        )
        .await?;
    let bytes_written = audio.len();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output.clone(), "sfx", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = SfxResult {
        prompt: args.text.clone(),
        duration_seconds: args.duration,
        looping: args.looping,
        output_path: out_path.display().to_string(),
        output_format,
        bytes_written,
    };

    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        let size_kb = r.bytes_written as f64 / 1024.0;
        println!(
            "{} {} ({:.1} KB, {}s)",
            "+".green(),
            r.output_path.bold(),
            size_kb,
            r.duration_seconds
                .map(|d| format!("{d:.1}"))
                .unwrap_or_else(|| "auto".into()),
        );
    });
    Ok(())
}
