//! music stream — POST /v1/music/stream
//!
//! Streaming variant of `music compose`: the response body is streamed
//! chunk-by-chunk and written to disk as it arrives. Useful for long
//! tracks where you don't want to wait for the full response before the
//! file is playable (or when the caller is piping to `ffplay`).

use crate::cli::StreamArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: StreamArgs,
) -> Result<(), AppError> {
    let body = super::build_compose_body(
        args.prompt.as_deref(),
        args.length_ms,
        args.composition_plan.as_deref(),
        args.model.as_deref(),
        args.force_instrumental,
        args.seed,
        args.respect_sections_durations,
        args.store_for_inpainting,
        args.sign_with_c2pa,
    )
    .await?;

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());
    let query = [("output_format", output_format.as_str())];

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    let mut file = tokio::fs::File::create(&out_path)
        .await
        .map_err(AppError::Io)?;

    let bytes_written =
        super::stream_post_json_bytes(client, "/v1/music/stream", &query, &body, &mut file).await?;

    let result = serde_json::json!({
        "prompt": args.prompt,
        "composition_plan_file": args.composition_plan,
        "length_ms": args.length_ms,
        "seed": args.seed,
        "output": out_path.display().to_string(),
        "output_format": output_format,
        "bytes_written": bytes_written,
        "streamed": true,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB, streamed)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0
        );
    })?;
    Ok(())
}
