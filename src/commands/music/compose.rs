//! music compose — POST /v1/music
//!
//! Grounded against `BodyComposeMusicV1MusicPost`:
//!   - `/v1/music` (JSON body, output_format as query param)
//!   - length: 3000-600000 ms (enforced by clap)
//!
//! Returns raw audio bytes in the response body. `output_format` is a
//! query-string parameter, not a body field.

use crate::cli::ComposeArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: ComposeArgs,
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
    let audio = client
        .post_json_bytes_with_query("/v1/music", &query, &body)
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
        "output_format": output_format,
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
    })?;
    Ok(())
}
