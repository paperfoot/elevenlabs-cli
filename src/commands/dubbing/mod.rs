//! Dubbing: multilingual dub of a video/audio source with optional editable
//! Studio-mode resource operations.
//!
//! Endpoints (grounded against the elevenlabs-python raw_client files under
//! `src/elevenlabs/dubbing/`):
//!   - POST   /v1/dubbing                               — dubbing::create
//!   - GET    /v1/dubbing                               — dubbing::list
//!   - GET    /v1/dubbing/{id}                          — dubbing::show
//!   - DELETE /v1/dubbing/{id}                          — dubbing::delete
//!   - GET    /v1/dubbing/{id}/audio/{lang}             — dubbing::audio
//!   - GET    /v1/dubbing/{id}/transcripts/{lang}/format/{fmt}  (dubbing::transcript)
//!   - POST   /v1/dubbing/resource/{id}/transcribe
//!   - POST   /v1/dubbing/resource/{id}/translate
//!   - POST   /v1/dubbing/resource/{id}/dub
//!   - POST   /v1/dubbing/resource/{id}/render/{lang}
//!   - POST   /v1/dubbing/resource/{id}/migrate-segments

pub mod audio;
pub mod create;
pub mod delete;
pub mod list;
pub mod resource;
pub mod show;
pub mod transcript;

use crate::cli::{DubbingAction, DubbingResourceAction};
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(ctx: Ctx, action: DubbingAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        DubbingAction::Create(args) => create::run(ctx, &client, args).await,
        DubbingAction::List {
            dubbing_status,
            filter_by_creator,
            page_size,
        } => list::run(ctx, &client, dubbing_status, filter_by_creator, page_size).await,
        DubbingAction::Show { dubbing_id } => show::run(ctx, &client, &dubbing_id).await,
        DubbingAction::Delete { dubbing_id, yes } => {
            delete::run(ctx, &client, &dubbing_id, yes).await
        }
        DubbingAction::GetAudio {
            dubbing_id,
            language_code,
            output,
        } => audio::run(ctx, &client, &dubbing_id, &language_code, output).await,
        DubbingAction::GetTranscript {
            dubbing_id,
            language_code,
            format,
            output,
        } => transcript::run(ctx, &client, &dubbing_id, &language_code, &format, output).await,
        DubbingAction::Resource { action } => dispatch_resource(ctx, &client, action).await,
    }
}

async fn dispatch_resource(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: DubbingResourceAction,
) -> Result<(), AppError> {
    match action {
        DubbingResourceAction::Transcribe { dubbing_id, patch } => {
            resource::transcribe::run(ctx, client, &dubbing_id, patch).await
        }
        DubbingResourceAction::Translate { dubbing_id, patch } => {
            resource::translate::run(ctx, client, &dubbing_id, patch).await
        }
        DubbingResourceAction::Dub { dubbing_id, patch } => {
            resource::dub::run(ctx, client, &dubbing_id, patch).await
        }
        DubbingResourceAction::Render {
            dubbing_id,
            language_code,
            patch,
        } => resource::render::run(ctx, client, &dubbing_id, &language_code, patch).await,
        DubbingResourceAction::MigrateSegments { dubbing_id, patch } => {
            resource::migrate::run(ctx, client, &dubbing_id, patch).await
        }
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────────

/// Read and parse a `--patch <PATH>` JSON body. Returns `{}` when no path
/// was provided so POSTs always carry a valid JSON body.
pub(crate) async fn load_patch_body(patch: Option<String>) -> Result<serde_json::Value, AppError> {
    let Some(p) = patch else {
        return Ok(serde_json::json!({}));
    };
    let path = std::path::Path::new(&p);
    if !path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("patch file does not exist: {}", path.display()),
            suggestion: None,
        });
    }
    let bytes = tokio::fs::read(path).await.map_err(AppError::Io)?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::InvalidInput {
        msg: format!("patch file '{}' is not valid JSON: {e}", path.display()),
        suggestion: None,
    })
}

/// GET with raw-bytes response. The shared client exposes POST variants that
/// return `bytes::Bytes`, but dubbing audio/transcript downloads are GETs, so
/// we drive `reqwest` directly here. Uses the same `check_status` path by
/// mapping the response through the client's standard helpers via the public
/// `http` client and `url()` helpers.
pub(crate) async fn get_bytes(
    client: &ElevenLabsClient,
    path: &str,
) -> Result<bytes::Bytes, AppError> {
    let resp = client.http.get(client.url(path)).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        // Redact anything that looks like an `sk_…` key before the body
        // reaches the error envelope — the central `check_status` path
        // already does this, but this helper drives `reqwest` directly so
        // it must redact manually. A misbehaving upstream proxy that echoed
        // the `xi-api-key` header must not leak the key into our JSON
        // output.
        let message = if body.is_empty() {
            format!("HTTP {code}")
        } else {
            crate::client::redact_secrets(&body.chars().take(300).collect::<String>())
        };
        return Err(match code {
            401 | 403 => AppError::AuthFailed(message),
            429 => AppError::RateLimited(message),
            _ => AppError::Api {
                status: code,
                message,
            },
        });
    }
    Ok(resp.bytes().await?)
}
