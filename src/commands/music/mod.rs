//! music — compose, plan, detailed, stream, upload, stem separation, video-to-music.
//!
//! Endpoints grounded against the ElevenLabs API reference (Apr 2026):
//!   - POST /v1/music                   (compose — audio bytes)
//!   - POST /v1/music/plan              (composition plan — JSON)
//!   - POST /v1/music/detailed          (JSON with audio_base64 + metadata)
//!   - POST /v1/music/stream            (streamed audio bytes)
//!   - POST /v1/music/upload            (multipart audio upload for inpainting)
//!   - POST /v1/music/stem-separation   (split into stems)
//!   - POST /v1/music/video-to-music    (video → score)
//!
//! One submodule per user-facing action; shared helpers (request-body
//! assembly, composition-plan ingestion) live below.

pub mod compose;
pub mod detailed;
pub mod plan;
pub mod stem;
pub mod stream;
pub mod upload;
pub mod video;

use crate::cli::MusicAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

/// Current music model. Legacy section-based plans retain music_v1 unless overridden.
pub(crate) const DEFAULT_MODEL: &str = "music_v2_5";

pub async fn dispatch(ctx: Ctx, action: MusicAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        MusicAction::Compose(args) => compose::run(ctx, &cfg, &client, args).await,
        MusicAction::Plan {
            prompt,
            length_ms,
            model,
        } => plan::run(ctx, &client, prompt, length_ms, model).await,
        MusicAction::Detailed(args) => detailed::run(ctx, &cfg, &client, args).await,
        MusicAction::Stream(args) => stream::run(ctx, &cfg, &client, args).await,
        MusicAction::Upload(args) => upload::run(ctx, &client, args).await,
        MusicAction::StemSeparation(args) => stem::run(ctx, &client, args).await,
        MusicAction::VideoToMusic(args) => video::run(ctx, &cfg, &client, args).await,
    }
}

// ── Shared helpers ─────────────────────────────────────────────────────────

/// Assemble the JSON body for /v1/music, /v1/music/detailed, and
/// /v1/music/stream. They all accept the same shape.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_compose_body(
    prompt: Option<&str>,
    length_ms: Option<u32>,
    composition_plan_path: Option<&str>,
    model: Option<&str>,
    force_instrumental: bool,
    seed: Option<u32>,
    respect_sections_durations: bool,
    store_for_inpainting: bool,
    sign_with_c2pa: bool,
) -> Result<serde_json::Value, AppError> {
    if prompt.is_none() && composition_plan_path.is_none() {
        return Err(AppError::InvalidInput {
            msg: "provide either a PROMPT or --composition-plan <file>".into(),
            suggestion: None,
        });
    }

    let mut body = serde_json::Map::new();
    if let Some(p) = prompt {
        if p.trim().is_empty() {
            return Err(AppError::InvalidInput {
                msg: "prompt is empty".into(),
                suggestion: None,
            });
        }
        body.insert("prompt".into(), serde_json::Value::String(p.to_string()));
    }
    if let Some(ms) = length_ms {
        body.insert("music_length_ms".into(), serde_json::json!(ms));
    }
    if let Some(plan_path) = composition_plan_path {
        let content = tokio::fs::read_to_string(plan_path)
            .await
            .map_err(AppError::Io)?;
        let mut plan: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| AppError::InvalidInput {
                msg: format!("--composition-plan is not valid JSON: {e}"),
                suggestion: None,
            })?;
        // `music plan > plan.json` writes the normal CLI success envelope.
        if plan.get("status").and_then(|v| v.as_str()) == Some("success")
            && plan.get("data").is_some_and(|v| v.is_object())
        {
            plan = plan["data"].take();
        }
        let legacy = plan.get("sections").is_some();
        let chunked = plan.get("chunks").is_some();
        let effective_model = model.unwrap_or(if legacy { "music_v1" } else { DEFAULT_MODEL });
        if (legacy && matches!(effective_model, "music_v2" | "music_v2_5"))
            || (chunked && effective_model == "music_v1")
        {
            let compatible_model = if legacy { "music_v1" } else { DEFAULT_MODEL };
            return Err(AppError::InvalidInput {
                msg: format!("composition plan format is incompatible with {effective_model}"),
                suggestion: Some(format!(
                    "Generate with: elevenlabs music compose --composition-plan <file> --model {compatible_model}"
                )),
            });
        }
        body.insert("model_id".into(), serde_json::json!(effective_model));
        body.insert("composition_plan".into(), plan);
    }
    body.entry("model_id")
        .or_insert_with(|| serde_json::json!(model.unwrap_or(DEFAULT_MODEL)));
    if force_instrumental {
        body.insert("force_instrumental".into(), serde_json::Value::Bool(true));
    }
    if let Some(seed) = seed {
        body.insert("seed".into(), serde_json::json!(seed));
    }
    if respect_sections_durations {
        body.insert(
            "respect_sections_durations".into(),
            serde_json::Value::Bool(true),
        );
    }
    if store_for_inpainting {
        body.insert("store_for_inpainting".into(), serde_json::Value::Bool(true));
    }
    if sign_with_c2pa {
        body.insert("sign_with_c2pa".into(), serde_json::Value::Bool(true));
    }
    Ok(serde_json::Value::Object(body))
}

/// Stream POST JSON body, writing the response bytes to `out` as they
/// arrive. Returns the total number of bytes written. Used by the /stream
/// endpoint so the file starts playable before the full response lands.
pub(crate) async fn stream_post_json_bytes<B: serde::Serialize, Q: serde::Serialize + ?Sized>(
    client: &ElevenLabsClient,
    path: &str,
    query: &Q,
    body: &B,
    out: &mut tokio::fs::File,
) -> Result<usize, AppError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let resp = client
        .http
        .post(client.url(path))
        .query(query)
        .json(body)
        .send()
        .await?;

    // Inline a trimmed version of `check_status`: on error bodies we still
    // want to surface the API message, so buffer the text. On success we
    // stream chunk-by-chunk. We also run the truncated body through
    // `redact_secrets` before it reaches the error envelope — the central
    // `check_status` path already does this, but this streaming helper
    // drives `reqwest` directly so it must redact manually. A misbehaving
    // upstream proxy that echoed the auth header must not leak the key.
    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        let msg = if body.is_empty() {
            format!("HTTP {code}")
        } else {
            crate::client::redact_secrets(&body.chars().take(300).collect::<String>())
        };
        return Err(match code {
            401 | 403 => AppError::AuthFailed(msg),
            429 => AppError::RateLimited(msg),
            _ => AppError::Api {
                status: code,
                message: msg,
            },
        });
    }

    let mut total: usize = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(AppError::from)?;
        total += bytes.len();
        out.write_all(&bytes).await.map_err(AppError::Io)?;
    }
    out.flush().await.map_err(AppError::Io)?;
    Ok(total)
}
