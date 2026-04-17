//! Semantic error type. Every variant maps to an exit code (1-4), a
//! machine-readable error code, and a recovery suggestion.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API key not configured")]
    AuthMissing,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("{0}")]
    Transient(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Update failed: {0}")]
    Update(String),
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 3,
            Self::Config(_) | Self::AuthMissing | Self::AuthFailed(_) => 2,
            Self::RateLimited(_) => 4,
            Self::Api { status, .. } if *status == 429 => 4,
            Self::Api { status, .. } if *status == 401 || *status == 403 => 2,
            Self::Api { .. }
            | Self::Transient(_)
            | Self::Io(_)
            | Self::Http(_)
            | Self::Update(_) => 1,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Config(_) => "config_error",
            Self::AuthMissing => "auth_missing",
            Self::AuthFailed(_) => "auth_failed",
            Self::RateLimited(_) => "rate_limited",
            Self::Api { status, .. } if *status == 429 => "rate_limited",
            Self::Api { status, .. } if *status == 401 || *status == 403 => "auth_failed",
            Self::Api { .. } => "api_error",
            Self::Transient(_) => "transient_error",
            Self::Io(_) => "io_error",
            Self::Http(_) => "http_error",
            Self::Update(_) => "update_error",
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            Self::InvalidInput(_) => "Check arguments with: elevenlabs --help".into(),
            Self::Config(_) => "Check config with: elevenlabs config show".into(),
            Self::AuthMissing => "Set your API key: elevenlabs config init --api-key <sk_...>  \
                 or: export ELEVENLABS_API_KEY=sk_..."
                .into(),
            Self::AuthFailed(msg) => {
                // Surface missing-scope errors verbatim so agents know what to grant.
                if msg.contains("permission") || msg.contains("missing") {
                    return format!("API key is missing a required permission. {msg}");
                }
                // The #1 way auth silently fails: ELEVENLABS_API_KEY is set in
                // the environment to a value that differs from the saved
                // config.toml. The env always wins, so `elevenlabs config init`
                // appears to do nothing. Detect and explain.
                if let Some(hint) = env_shadow_hint() {
                    return hint;
                }
                "API key is invalid. Re-issue via https://elevenlabs.io/app/settings/api-keys \
                 and run: elevenlabs config init --api-key <sk_...>"
                    .into()
            }
            Self::RateLimited(_) => "Wait a moment and retry the command".into(),
            Self::Api { status, .. } if *status == 429 => {
                "Rate limited — wait a moment and retry".into()
            }
            Self::Api { status, .. } if *status == 401 || *status == 403 => {
                "API key is invalid or lacks permission for this endpoint".into()
            }
            Self::Api { .. } => {
                "Retry the command; check status.elevenlabs.io if it persists".into()
            }
            Self::Transient(_) | Self::Io(_) | Self::Http(_) => "Retry the command".into(),
            Self::Update(_) => "Retry later, or run: cargo install elevenlabs".into(),
        }
    }
}

/// If `ELEVENLABS_API_KEY` is set AND the saved config.toml has a different
/// `api_key`, return a tailored remediation. Env wins over file, so this
/// situation looks like "the CLI saved my key but auth still fails" unless we
/// surface it. Mask both values when showing them.
fn env_shadow_hint() -> Option<String> {
    let env_key = std::env::var("ELEVENLABS_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;
    let file_key = read_file_api_key()?;
    if env_key == file_key {
        return None;
    }
    Some(format!(
        "ELEVENLABS_API_KEY ({}) is set in your environment and overrides the saved \
         config.toml ({}). The env var value is what was sent and it's invalid. \
         Fix by either: (1) unset ELEVENLABS_API_KEY (the saved file value will take over), \
         or (2) re-issue a new key and run: elevenlabs config init --api-key <sk_...>",
        crate::config::mask_secret(&env_key),
        crate::config::mask_secret(&file_key),
    ))
}

/// Read `api_key` straight from the on-disk TOML file without the env-var
/// overlay that `config::load()` applies — we need the raw file value to
/// compare against the env value and detect shadowing.
fn read_file_api_key() -> Option<String> {
    let path = crate::config::config_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    parsed
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// Allow `?` on reqwest errors in async paths.
impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if let Some(status) = e.status() {
            AppError::Api {
                status: status.as_u16(),
                message: e.to_string(),
            }
        } else if e.is_timeout() {
            AppError::Transient(format!("request timed out: {e}"))
        } else if e.is_connect() {
            AppError::Transient(format!("connection failed: {e}"))
        } else {
            AppError::Http(e.to_string())
        }
    }
}
