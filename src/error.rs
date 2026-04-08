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
                    format!("API key is missing a required permission. {msg}")
                } else {
                    "API key is invalid. Re-issue via https://elevenlabs.io/app/settings/api-keys \
                     and run: elevenlabs config init --api-key <sk_...>"
                        .into()
                }
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
