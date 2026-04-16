//! music compose / plan
//!
//! Grounded against `BodyComposeMusicV1MusicPost` in the Fern-generated JS SDK:
//!   - `/v1/music` for compose (JSON body, output_format as query param)
//!   - `/v1/music/plan` for plan (JSON body)
//!   - length: 3000-600000 ms (enforced by clap)

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
            format,
            output,
            composition_plan,
            model,
            force_instrumental,
            seed,
            respect_sections_durations,
            store_for_inpainting,
            sign_with_c2pa,
        } => {
            compose(
                ctx,
                &cfg,
                &client,
                ComposeArgs {
                    prompt,
                    length_ms,
                    format,
                    output,
                    composition_plan,
                    model,
                    force_instrumental,
                    seed,
                    respect_sections_durations,
                    store_for_inpainting,
                    sign_with_c2pa,
                },
            )
            .await
        }
        MusicAction::Plan {
            prompt,
            length_ms,
            model,
        } => plan(ctx, &client, prompt, length_ms, model).await,
    }
}

struct ComposeArgs {
    prompt: Option<String>,
    length_ms: Option<u32>,
    format: Option<String>,
    output: Option<String>,
    composition_plan: Option<String>,
    model: Option<String>,
    force_instrumental: bool,
    seed: Option<u32>,
    respect_sections_durations: bool,
    store_for_inpainting: bool,
    sign_with_c2pa: bool,
}

async fn compose(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: ComposeArgs,
) -> Result<(), AppError> {
    // Validate: exactly one of prompt / composition_plan (clap should catch
    // this but double-check for clearer error messages).
    if args.prompt.is_none() && args.composition_plan.is_none() {
        return Err(AppError::InvalidInput(
            "provide either a PROMPT or --composition-plan <file>".into(),
        ));
    }

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());

    let mut body = serde_json::Map::new();
    if let Some(p) = &args.prompt {
        if p.trim().is_empty() {
            return Err(AppError::InvalidInput("prompt is empty".into()));
        }
        body.insert("prompt".into(), serde_json::Value::String(p.clone()));
    }
    if let Some(ms) = args.length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }
    if let Some(plan_path) = &args.composition_plan {
        let content = tokio::fs::read_to_string(plan_path)
            .await
            .map_err(AppError::Io)?;
        let plan: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            AppError::InvalidInput(format!("--composition-plan is not valid JSON: {e}"))
        })?;
        body.insert("composition_plan".into(), plan);
    }
    if let Some(m) = &args.model {
        body.insert("model_id".into(), serde_json::Value::String(m.clone()));
    }
    if args.force_instrumental {
        body.insert("force_instrumental".into(), serde_json::Value::Bool(true));
    }
    if let Some(seed) = args.seed {
        body.insert("seed".into(), serde_json::json!(seed));
    }
    if args.respect_sections_durations {
        body.insert(
            "respect_sections_durations".into(),
            serde_json::Value::Bool(true),
        );
    }
    if args.store_for_inpainting {
        body.insert("store_for_inpainting".into(), serde_json::Value::Bool(true));
    }
    if args.sign_with_c2pa {
        body.insert("sign_with_c2pa".into(), serde_json::Value::Bool(true));
    }

    // POST /v1/music streams back audio bytes directly. Pass output_format
    // as a query param so the server picks the right codec — matches the
    // official Python + JS SDKs.
    let query = [("output_format", output_format.as_str())];
    let audio = client
        .post_json_bytes_with_query("/v1/music", &query, &serde_json::Value::Object(body))
        .await?;
    let bytes_written = audio.len();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    tokio::fs::write(&out_path, &audio)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "prompt": args.prompt,
        "composition_plan_file": args.composition_plan,
        "length_ms": args.length_ms,
        "seed": args.seed,
        "force_instrumental": args.force_instrumental,
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
    model: Option<String>,
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::InvalidInput("prompt is empty".into()));
    }
    let mut body = serde_json::Map::new();
    body.insert("prompt".into(), serde_json::Value::String(prompt));
    if let Some(ms) = length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }
    if let Some(m) = model {
        body.insert("model_id".into(), serde_json::Value::String(m));
    }
    let resp: serde_json::Value = client
        .post_json("/v1/music/plan", &serde_json::Value::Object(body))
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}
