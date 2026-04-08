//! Thin async HTTP client wrapper around `reqwest`. Every API module uses
//! the same `ElevenLabsClient` so auth headers, base URL, timeouts, and
//! error mapping are handled in one place.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::AppConfig;
use crate::error::AppError;

pub const DEFAULT_BASE_URL: &str = "https://api.elevenlabs.io";
pub const USER_AGENT: &str = concat!("elevenlabs-cli/", env!("CARGO_PKG_VERSION"), " (+https://github.com/199-biotechnologies/elevenlabs-cli)");

#[derive(Clone)]
pub struct ElevenLabsClient {
    pub http: reqwest::Client,
    pub base_url: String,
    #[allow(dead_code)]
    pub api_key: String,
}

impl ElevenLabsClient {
    /// Build a client from loaded config. Errors with `AuthMissing` if no
    /// API key is configured anywhere.
    pub fn from_config(cfg: &AppConfig) -> Result<Self, AppError> {
        let api_key = cfg.resolve_api_key().ok_or(AppError::AuthMissing)?;

        let mut headers = HeaderMap::new();
        // xi-api-key is the ElevenLabs auth header.
        let mut val = HeaderValue::from_str(&api_key)
            .map_err(|_| AppError::Config("api_key contains invalid characters".into()))?;
        val.set_sensitive(true);
        headers.insert("xi-api-key", val);
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(USER_AGENT),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()
            .map_err(|e| AppError::Http(format!("building http client: {e}")))?;

        let base_url = std::env::var("ELEVENLABS_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    /// Build an absolute URL for the given path. Path should start with `/`.
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    // ── GET → JSON ─────────────────────────────────────────────────────────

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let resp = self.http.get(self.url(path)).send().await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
    }

    pub async fn get_json_with_query<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<T, AppError> {
        let resp = self.http.get(self.url(path)).query(query).send().await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
    }

    // ── POST JSON → JSON ───────────────────────────────────────────────────

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        let resp = self.http.post(self.url(path)).json(body).send().await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
    }

    // ── POST JSON → raw bytes (for audio endpoints) ────────────────────────

    pub async fn post_json_bytes<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<bytes::Bytes, AppError> {
        let resp = self.http.post(self.url(path)).json(body).send().await?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?)
    }

    pub async fn post_json_bytes_with_query<B: Serialize, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
        body: &B,
    ) -> Result<bytes::Bytes, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .query(query)
            .json(body)
            .send()
            .await?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?)
    }

    // ── POST multipart (file uploads) ──────────────────────────────────────

    pub async fn post_multipart_json<T: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<T, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .multipart(form)
            .send()
            .await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
    }

    pub async fn post_multipart_bytes(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<bytes::Bytes, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .multipart(form)
            .send()
            .await?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?)
    }

    // ── DELETE ──────────────────────────────────────────────────────────────

    pub async fn delete(&self, path: &str) -> Result<(), AppError> {
        let resp = self.http.delete(self.url(path)).send().await?;
        let _ = check_status(resp).await?;
        Ok(())
    }

    // ── PATCH JSON ──────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        let resp = self.http.patch(self.url(path)).json(body).send().await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
    }
}

/// Convert non-2xx responses into semantic `AppError`s with the ElevenLabs
/// error body surfaced in the message when present.
async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, AppError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let code = status.as_u16();
    // Body may be JSON `{ "detail": { "status": ..., "message": ... } }`
    // or plain text — surface whichever we get.
    let body = resp.text().await.unwrap_or_default();

    let message = extract_api_message(&body).unwrap_or_else(|| {
        if body.is_empty() {
            format!("HTTP {code}")
        } else {
            body.chars().take(300).collect::<String>()
        }
    });

    Err(match code {
        401 | 403 => AppError::AuthFailed(message),
        429 => AppError::RateLimited(message),
        _ => AppError::Api {
            status: code,
            message,
        },
    })
}

fn extract_api_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(detail) = v.get("detail") {
        if let Some(msg) = detail.get("message").and_then(|m| m.as_str()) {
            return Some(msg.to_string());
        }
        if let Some(s) = detail.as_str() {
            return Some(s.to_string());
        }
        return Some(detail.to_string());
    }
    v.get("message")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}
