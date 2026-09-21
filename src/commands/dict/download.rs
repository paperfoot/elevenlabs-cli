//! `dict download ID [--version VER] [--output PATH]` — fetch the rendered
//! PLS XML for a dictionary version and write it to disk.
//!
//! GET /v1/pronunciation-dictionaries/{id}/{version}/download
//!
//! The response body is PLS XML (text/xml), not JSON, so we bypass the JSON
//! helpers and stream bytes straight to the output path. If `--version` is
//! omitted, we first fetch the dictionary detail via `dict show` to learn the
//! `latest_version_id` — saves the caller one round-trip.

use std::path::PathBuf;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    id: String,
    version: Option<String>,
    output: Option<String>,
) -> Result<(), AppError> {
    let version_id = match version {
        Some(v) => v,
        None => resolve_latest_version(client, &id).await?,
    };

    let api_path = format!("/v1/pronunciation-dictionaries/{id}/{version_id}/download");
    let url = client.url(&api_path);

    let resp = client.http.get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        // Route response bodies through `redact_secrets` before they reach
        // the error envelope — a misbehaving upstream proxy that echoes the
        // `xi-api-key` header (or any other `sk_…` token) would otherwise
        // leak the key into our stdout/stderr JSON. The central
        // `check_status` path already applies this; this downloader drives
        // `reqwest` directly so it must redact manually.
        let snippet: String =
            crate::client::redact_secrets(&body.chars().take(300).collect::<String>());
        return Err(match code {
            401 | 403 => AppError::AuthFailed(snippet),
            429 => AppError::RateLimited(snippet),
            _ => AppError::Api {
                status: code,
                message: if snippet.is_empty() {
                    format!("HTTP {code}")
                } else {
                    snippet
                },
            },
        });
    }
    let bytes = resp.bytes().await?;

    let out_path: PathBuf = match output {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(format!("pronunciation_dictionary_{id}_{version_id}.pls")),
    };
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(AppError::Io)?;
        }
    }
    tokio::fs::write(&out_path, &bytes)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "id": id,
        "version_id": version_id,
        "file": out_path.display().to_string(),
        "bytes": bytes.len(),
    });
    output::print_success_or(ctx, &result, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} wrote {} bytes to {}",
            "+".green(),
            v["bytes"],
            v["file"].as_str().unwrap_or("").bold()
        );
    })?;
    Ok(())
}

async fn resolve_latest_version(client: &ElevenLabsClient, id: &str) -> Result<String, AppError> {
    let path = format!("/v1/pronunciation-dictionaries/{id}");
    let detail: serde_json::Value = client.get_json(&path).await?;
    detail
        .get("latest_version_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::InvalidInput {
            msg: format!("dictionary '{id}' has no latest_version_id — pass --version explicitly"),
            suggestion: None,
        })
}
