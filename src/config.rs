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

/// Where the effective API key came from. Surfaced in `config show` and in
/// auth-error suggestions so users know which source to edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    /// ELEVENLABS_API_KEY environment variable.
    Env,
    /// `api_key` in config.toml.
    File,
    /// No key available anywhere.
    None,
}

impl AuthSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Env => "ELEVENLABS_API_KEY env var",
            Self::File => "config file",
            Self::None => "(unset)",
        }
    }
}

/// Snapshot of every API-key source, for diagnostic output. The CLI consults
/// this when the user runs `config show`, when `config init` saves a new key,
/// and when an auth call fails — so the "env var silently shadows the file"
/// case gets surfaced instead of producing a generic "invalid key" error.
#[derive(Debug, Clone)]
pub struct AuthKeyState {
    /// Raw value of `ELEVENLABS_API_KEY` (trimmed), if set and non-empty.
    pub env_key: Option<String>,
    /// Raw `api_key` from config.toml (trimmed), if present and non-empty.
    pub file_key: Option<String>,
}

impl AuthKeyState {
    /// Read the current state from the environment and the given loaded
    /// config. The config loader pre-populates `api_key` with the env value
    /// when set, so we re-read the env directly to distinguish the two.
    pub fn snapshot(file_key_from_config: Option<&str>) -> Self {
        let env_key = std::env::var("ELEVENLABS_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        // `config::load()` overwrites `cfg.api_key` with the env value when the
        // env is set; re-read the raw TOML file directly so we can report the
        // two sources independently.
        let file_key = read_file_api_key().or_else(|| file_key_from_config.map(String::from));
        Self {
            env_key,
            file_key: file_key.filter(|s| !s.trim().is_empty()),
        }
    }

    /// Which source is being used for auth right now.
    pub fn effective_source(&self) -> AuthSource {
        if self.env_key.is_some() {
            AuthSource::Env
        } else if self.file_key.is_some() {
            AuthSource::File
        } else {
            AuthSource::None
        }
    }

    /// The value that actually ships on the wire (env wins over file).
    pub fn effective_key(&self) -> Option<&str> {
        self.env_key.as_deref().or(self.file_key.as_deref())
    }

    /// True iff the env var and file hold different non-empty values.
    /// This is the case that silently breaks auth: the env is stale/invalid
    /// and the user's saved-config key is ignored.
    pub fn env_shadows_file(&self) -> bool {
        matches!(
            (&self.env_key, &self.file_key),
            (Some(e), Some(f)) if e.trim() != f.trim()
        )
    }
}

/// Parse just the `api_key` field from the on-disk TOML config. Unlike
/// `load()` this does NOT fold in the env var — we need the file value
/// independently to detect the env-shadow case.
fn read_file_api_key() -> Option<String> {
    let path = config_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    parsed
        .get("api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

    /// Build a full snapshot of where keys are and which one wins.
    pub fn auth_key_state(&self) -> AuthKeyState {
        AuthKeyState::snapshot(self.api_key.as_deref())
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

/// Atomically write the given config to disk, setting 0600 permissions on
/// Unix before the final rename. The write goes through a sibling temp
/// file so concurrent readers never observe a partially-written config,
/// and the temp file has 0600 the whole time so the secret is never
/// briefly world-readable.
pub fn save(cfg: &AppConfig) -> Result<PathBuf, AppError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let toml = toml::to_string_pretty(cfg)
        .map_err(|e| AppError::Config(format!("serialising config: {e}")))?;

    // Write to a sibling temp file, then rename.
    let tmp_path = path.with_extension("toml.tmp");

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&tmp_path)?;
        f.write_all(toml.as_bytes())?;
        f.sync_all().ok();
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&tmp_path, toml.as_bytes())?;
    }

    std::fs::rename(&tmp_path, &path).map_err(|e| {
        // On rename failure, try to clean up the temp file so we don't
        // leave a world-readable-ish file with a secret in it.
        let _ = std::fs::remove_file(&tmp_path);
        AppError::Io(e)
    })?;

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
