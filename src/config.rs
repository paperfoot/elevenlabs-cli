//! Config loading with 3-tier precedence:
//!   1. Compiled defaults
//!   2. TOML file (~/.config/elevenlabs-cli/config.toml)
//!   3. Environment variables (ELEVENLABS_* or ELEVENLABS_CLI_*)
//!
//! The API key resolves from any of: `--api-key` flag (per-command where
//! supported), `ELEVENLABS_API_KEY` env var, or `api_key` in config.toml.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// ElevenLabs API key (falls back to ELEVENLABS_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Per-command defaults
    #[serde(default)]
    pub defaults: Defaults,

    /// Self-update settings
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    /// Default voice ID for TTS
    #[serde(default)]
    pub voice_id: Option<String>,

    /// Default model ID for TTS
    #[serde(default)]
    pub model_id: Option<String>,

    /// Default output format
    #[serde(default)]
    pub output_format: Option<String>,

    /// Default output directory for generated files
    #[serde(default)]
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub owner: String,
    pub repo: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            owner: "199-biotechnologies".into(),
            repo: "elevenlabs-cli".into(),
        }
    }
}

impl AppConfig {
    /// Resolve the API key with the documented precedence: environment
    /// variables win over the config file, which wins over (nothing).
    /// CLI flags at the per-command level would win over both but we
    /// don't currently expose a global `--api-key` flag.
    ///
    /// This matches the README's precedence ladder:
    ///   CLI flags → env vars → config file → defaults
    ///
    /// Earlier versions had config > env by accident. Codex caught it.
    pub fn resolve_api_key(&self) -> Option<String> {
        // 1. Env var wins.
        if let Ok(k) = std::env::var("ELEVENLABS_API_KEY") {
            let trimmed = k.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        // 2. Config file.
        if let Some(k) = self.api_key.as_ref() {
            let trimmed = k.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    /// Voice ID to use if none specified. Falls back to a built-in default.
    pub fn default_voice_id(&self) -> String {
        self.defaults
            .voice_id
            .clone()
            .unwrap_or_else(|| "cgSgspJ2msm6clMCkdW9".to_string())
    }

    /// Model to use for TTS if none specified.
    pub fn default_model_id(&self) -> String {
        self.defaults
            .model_id
            .clone()
            .unwrap_or_else(|| "eleven_multilingual_v2".to_string())
    }

    pub fn default_output_format(&self) -> String {
        self.defaults
            .output_format
            .clone()
            .unwrap_or_else(|| "mp3_44100_128".to_string())
    }
}

pub fn config_path() -> PathBuf {
    // Allow a full-path override for tests and power users. This is the
    // exact path to the config.toml file, not a directory.
    if let Ok(p) = std::env::var("ELEVENLABS_CLI_CONFIG") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    directories::ProjectDirs::from("", "", "elevenlabs-cli")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

pub fn load() -> Result<AppConfig, AppError> {
    use figment::Figment;
    use figment::providers::{Env, Format as _, Serialized, Toml};

    // Two env prefixes are accepted — `ELEVENLABS_CLI_` is canonical per
    // figment split convention, but `ELEVENLABS_API_KEY` is the well-known
    // name the rest of the ecosystem uses. We merge both, letting the latter
    // win.
    let base = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path()))
        .merge(Env::prefixed("ELEVENLABS_CLI_").split("_"));

    let mut cfg: AppConfig = base
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))?;

    // Documented precedence (README): env vars > config file > defaults.
    // ELEVENLABS_API_KEY is the well-known name used by the whole
    // ElevenLabs ecosystem — it must override anything from the TOML file
    // so `config show` reflects what the client will actually send, and
    // so setting the env var never quietly falls back to a stale key in
    // ~/.config/.../config.toml.
    if let Ok(k) = std::env::var("ELEVENLABS_API_KEY") {
        let trimmed = k.trim().to_string();
        if !trimmed.is_empty() {
            cfg.api_key = Some(trimmed);
        }
    }

    Ok(cfg)
}

/// Write the given config to disk, creating the parent directory if needed.
/// Sets 0600 permissions on Unix so the file isn't world-readable.
pub fn save(cfg: &AppConfig) -> Result<PathBuf, AppError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml = toml::to_string_pretty(cfg)
        .map_err(|e| AppError::Config(format!("serialising config: {e}")))?;
    std::fs::write(&path, toml)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(path)
}

/// Mask a secret for display: shows prefix and suffix only.
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        let prefix: String = chars[..2.min(chars.len())].iter().collect();
        format!("{prefix}***")
    } else {
        let prefix: String = chars[..6].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
