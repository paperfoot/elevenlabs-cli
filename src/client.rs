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
pub const USER_AGENT: &str = concat!(
    "elevenlabs-cli/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/paperfoot/elevenlabs-cli)"
);

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
        Self::with_redirect_policy(cfg, reqwest::redirect::Policy::limited(10))
    }

    pub fn for_api(cfg: &AppConfig) -> Result<Self, AppError> {
        // Generic operations can include redirect endpoints (e.g. /docs).
        // Surface their location without forwarding xi-api-key to another host.
        Self::with_redirect_policy(cfg, reqwest::redirect::Policy::none())
    }

    fn with_redirect_policy(
        cfg: &AppConfig,
        policy: reqwest::redirect::Policy,
    ) -> Result<Self, AppError> {
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
            .redirect(policy)
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

    /// Schema-selected operation, sharing authentication and status handling.
    pub async fn send_api(
        &self,
        method: &str,
        path: &str,
        query: &[(String, String)],
        headers: &[(String, String)],
        body: Option<&serde_json::Value>,
        form: Option<reqwest::multipart::Form>,
    ) -> Result<reqwest::Response, AppError> {
        let method = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
            .map_err(|_| AppError::bad_input("Unsupported HTTP method"))?;
        let mut request = self.http.request(method, self.url(path)).query(query);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        if let Some(form) = form {
            request = request.multipart(form);
        } else if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| AppError::Http(e.without_url().to_string()))?;
        if response.status().is_redirection() {
            return Ok(response);
        }
        check_status(response).await.map_err(|error| match error {
            AppError::Api { status, message } => {
                let message = redact_api_error(&message, &self.api_key, body);
                if (400..500).contains(&status) {
                    AppError::bad_input_with(
                        message,
                        "Inspect the request inputs: elevenlabs api call --help",
                    )
                } else {
                    AppError::Api { status, message }
                }
            }
            AppError::AuthFailed(message) => {
                AppError::AuthFailed(redact_api_error(&message, &self.api_key, body))
            }
            AppError::RateLimited(message) => {
                AppError::RateLimited(redact_api_error(&message, &self.api_key, body))
            }
            error => error,
        })
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

    /// GET a path and return the raw response body as bytes. Used by
    /// `conversations audio`, `dubbing get-audio`, etc. — anywhere the
    /// response is a binary payload (mp3/wav/mp4/zip) rather than JSON.
    /// Status + error body handling routes through the same `check_status`
    /// pipeline as every other method so error shape stays consistent.
    pub async fn get_bytes(&self, path: &str) -> Result<bytes::Bytes, AppError> {
        let resp = self.http.get(self.url(path)).send().await?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?)
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

    #[allow(dead_code)]
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

    /// JSON POST with query params, returning JSON. Used by
    /// `/v1/text-to-speech/{id}/with-timestamps` where the body is JSON, the
    /// response is JSON, and `output_format`/`enable_logging` are query params.
    pub async fn post_json_with_query<B: Serialize, Q: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        query: &Q,
        body: &B,
    ) -> Result<T, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .query(query)
            .json(body)
            .send()
            .await?;
        check_status(resp)
            .await?
            .json::<T>()
            .await
            .map_err(Into::into)
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

    /// Like `post_multipart_json` but also attaches query-string parameters.
    /// Needed for endpoints like `/v1/speech-to-text` where `enable_logging`
    /// is a query param even though the body is multipart.
    pub async fn post_multipart_json_with_query<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
        form: reqwest::multipart::Form,
    ) -> Result<T, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .query(query)
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

    /// Multipart POST with query-string parameters. Needed by endpoints like
    /// `/v1/speech-to-speech/{voice_id}` where `output_format`,
    /// `enable_logging`, and `optimize_streaming_latency` are query params
    /// even though the body is multipart.
    pub async fn post_multipart_bytes_with_query<Q: Serialize + ?Sized>(
        &self,
        path: &str,
        query: &Q,
        form: reqwest::multipart::Form,
    ) -> Result<bytes::Bytes, AppError> {
        let resp = self
            .http
            .post(self.url(path))
            .query(query)
            .multipart(form)
            .send()
            .await?;
        let resp = check_status(resp).await?;
        Ok(resp.bytes().await?)
    }

    // ── DELETE ──────────────────────────────────────────────────────────────

    pub async fn delete(&self, path: &str) -> Result<(), AppError> {
        let resp = self.http.delete(self.url(path)).send().await?;
        check_status(resp).await?;
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
/// error body surfaced in the message when present. Defensively strips
/// anything that looks like an `sk_*` key out of the body before surfacing
/// it, so a misbehaving upstream proxy that echoes the auth header can't
/// leak the key into our JSON envelope.
async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, AppError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let code = status.as_u16();
    let body = resp.text().await.unwrap_or_default();

    let raw = extract_api_message(&body).unwrap_or_else(|| {
        if body.is_empty() {
            format!("HTTP {code}")
        } else {
            body.chars().take(300).collect::<String>()
        }
    });
    let message = redact_secrets(&raw);

    Err(match code {
        401 | 403 => AppError::AuthFailed(message),
        429 => AppError::RateLimited(message),
        _ => AppError::Api {
            status: code,
            message,
        },
    })
}

/// Scrub anything that looks like an ElevenLabs API key (`sk_` + hex/base62)
/// out of arbitrary text. We match `sk_` followed by 10+ word chars and
/// replace with `sk_***`. Belt-and-braces: the reqwest client already
/// marks the auth header as sensitive, but we never want to rely on an
/// upstream to behave.
///
/// Exposed to the rest of the crate so ad-hoc error paths (dubbing,
/// dict/download, music/stream) can reuse the same redaction before they
/// surface raw response bodies in error envelopes — the central
/// `check_status` path already applies this, but a few endpoints drive
/// `reqwest` directly and must redact manually.
pub(crate) fn redact_secrets(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        // Look for literal "sk_" start.
        if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"sk_" {
            // Find end of the alphanumeric/underscore run.
            let mut j = i + 3;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            // Only redact if the run looks secret-shaped (>= 10 chars after sk_).
            if j - (i + 3) >= 10 {
                out.push_str("sk_***");
                i = j;
                continue;
            }
        }
        // Copy one UTF-8 scalar at a time so we don't split codepoints.
        let ch_len = utf8_char_len(bytes[i]);
        out.push_str(&text[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    // Leading-byte pattern determines codepoint length. Continuation
    // bytes (0x80..=0xbf) are treated as single-byte too; they shouldn't
    // appear as the start byte if we walk the string correctly.
    if b < 0xc0 {
        1
    } else if b < 0xe0 {
        2
    } else if b < 0xf0 {
        3
    } else {
        4
    }
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

pub(crate) fn credential_field(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    matches!(
        key.as_str(),
        "authorization"
            | "password"
            | "secret"
            | "secret_value"
            | "token"
            | "access_token"
            | "refresh_token"
            | "api_key"
            | "xi_api_key"
    ) || key.ends_with("_secret")
        || key.ends_with("_api_key")
        || matches!(key.as_str(), "secret_key" | "secrets" | "private_key")
        || (key.ends_with("_token") && !matches!(key.as_str(), "next_page_token" | "page_token"))
}

fn redact_api_error(message: &str, api_key: &str, body: Option<&serde_json::Value>) -> String {
    fn scrub(message: &mut String, value: &serde_json::Value, secret: bool) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    scrub(message, value, secret || credential_field(key));
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    scrub(message, value, secret);
                }
            }
            serde_json::Value::String(value) if secret && !value.is_empty() => {
                *message = message.replace(value, "[REDACTED]");
            }
            _ => {}
        }
    }
    let mut message = message.replace(api_key, "[REDACTED]");
    if let Some(body) = body {
        scrub(&mut message, body, false);
    }
    message
}
