//! Command modules. Framework commands (agent_info, skill, config, update)
//! + domain commands (tts, stt, voices, agents, …).

pub mod agent_info;
pub mod agents;
pub mod audio;
pub mod config;
pub mod conversations;
pub mod history;
pub mod models;
pub mod music;
pub mod phone;
pub mod sfx;
pub mod skill;
pub mod stt;
pub mod tts;
pub mod update;
pub mod user;
pub mod voices;

// ── Shared helpers ──────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Resolve an output path for a generated file. If `path` is given, return it
/// as-is; otherwise build a name from `kind` + timestamp + extension, placed
/// in the current working directory.
pub fn resolve_output_path(path: Option<String>, kind: &str, ext: &str) -> PathBuf {
    if let Some(p) = path {
        return PathBuf::from(p);
    }
    let ts = now_timestamp();
    PathBuf::from(format!("{kind}_{ts}.{ext}"))
}

pub fn now_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

/// Read a file into memory. Helpful for multipart uploads.
pub async fn read_file_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    tokio::fs::read(path).await.map_err(AppError::Io)
}

/// Infer MIME type from file extension (fallback to application/octet-stream).
pub fn mime_for_path(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string()
}
