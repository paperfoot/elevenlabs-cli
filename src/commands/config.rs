//! config show / path / set / check / init.

use serde::Serialize;

use crate::client::ElevenLabsClient;
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

// ── show ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MaskedConfig<'a> {
    api_key: String,
    defaults: &'a config::Defaults,
    update: &'a config::UpdateConfig,
    path: String,
}

pub fn show(ctx: Ctx, cfg: &AppConfig) -> Result<(), AppError> {
    let masked = MaskedConfig {
        api_key: cfg
            .api_key
            .as_deref()
            .map(config::mask_secret)
            .unwrap_or_else(|| "(not set)".into()),
        defaults: &cfg.defaults,
        update: &cfg.update,
        path: config::config_path().display().to_string(),
    };

    output::print_success_or(ctx, &masked, |m| {
        use owo_colors::OwoColorize;
        println!("{}", "ElevenLabs CLI config".bold());
        println!("  {} {}", "path:".dimmed(), m.path);
        println!("  {} {}", "api_key:".dimmed(), m.api_key);
        if let Some(v) = &m.defaults.voice_id {
            println!("  {} {}", "voice_id:".dimmed(), v);
        }
        if let Some(v) = &m.defaults.model_id {
            println!("  {} {}", "model_id:".dimmed(), v);
        }
        if let Some(v) = &m.defaults.output_format {
            println!("  {} {}", "format:".dimmed(), v);
        }
        println!(
            "  {} {}/{} (enabled={})",
            "update:".dimmed(),
            m.update.owner,
            m.update.repo,
            m.update.enabled
        );
    });

    Ok(())
}

// ── path ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ConfigPath {
    path: String,
    exists: bool,
}

pub fn path(ctx: Ctx) -> Result<(), AppError> {
    let p = config::config_path();
    let result = ConfigPath {
        path: p.display().to_string(),
        exists: p.exists(),
    };
    output::print_success_or(ctx, &result, |r| {
        println!("{}", r.path);
        if !r.exists {
            use owo_colors::OwoColorize;
            println!("  {}", "(file does not exist, using defaults)".dimmed());
        }
    });
    Ok(())
}

// ── set ────────────────────────────────────────────────────────────────────

pub fn set(ctx: Ctx, key: &str, value: &str) -> Result<(), AppError> {
    let mut cfg = config::load().unwrap_or_default();

    match key {
        "api_key" => cfg.api_key = Some(value.to_string()),
        "defaults.voice_id" | "voice_id" => cfg.defaults.voice_id = Some(value.to_string()),
        "defaults.model_id" | "model_id" => cfg.defaults.model_id = Some(value.to_string()),
        "defaults.output_format" | "output_format" | "format" => {
            cfg.defaults.output_format = Some(value.to_string());
        }
        "defaults.output_dir" | "output_dir" => cfg.defaults.output_dir = Some(value.to_string()),
        "update.enabled" => {
            cfg.update.enabled = value
                .parse()
                .map_err(|_| AppError::InvalidInput(format!("expected bool, got '{value}'")))?;
        }
        "update.owner" => cfg.update.owner = value.to_string(),
        "update.repo" => cfg.update.repo = value.to_string(),
        other => {
            return Err(AppError::InvalidInput(format!(
                "unknown config key: '{other}'. Known: api_key, defaults.voice_id, \
                 defaults.model_id, defaults.output_format, defaults.output_dir, \
                 update.enabled, update.owner, update.repo"
            )));
        }
    }

    let path = config::save(&cfg)?;
    let result = serde_json::json!({
        "key": key,
        "saved": true,
        "path": path.display().to_string(),
    });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} {} saved to {}", "+".green(), key, path.display());
    });
    Ok(())
}

// ── check ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CheckResult {
    api_key_present: bool,
    api_key_valid: bool,
    endpoint: String,
    voices_available: usize,
}

pub async fn check(ctx: Ctx, cfg: &AppConfig) -> Result<(), AppError> {
    if cfg.resolve_api_key().is_none() {
        return Err(AppError::AuthMissing);
    }

    // /v1/voices is the broadest canary — any key that can do TTS can list
    // voices. /v1/user and /v1/models need extra scopes on restricted keys.
    let client = ElevenLabsClient::from_config(cfg)?;
    let resp: serde_json::Value = client.get_json("/v1/voices").await?;
    let count = resp
        .get("voices")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let result = CheckResult {
        api_key_present: true,
        api_key_valid: true,
        endpoint: "/v1/voices".into(),
        voices_available: count,
    };

    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!("{} API key is valid", "✓".green());
        println!(
            "  {} {} voices available",
            "ok:".dimmed(),
            r.voices_available
        );
    });
    Ok(())
}

// ── init ───────────────────────────────────────────────────────────────────

pub fn init(ctx: Ctx, api_key: Option<String>) -> Result<(), AppError> {
    let api_key = api_key.ok_or_else(|| {
        AppError::InvalidInput(
            "pass --api-key <sk_...> (this CLI is non-interactive; agents \
             and scripts should always provide the key as a flag)"
                .into(),
        )
    })?;

    let mut cfg = config::load().unwrap_or_default();
    cfg.api_key = Some(api_key.trim().to_string());
    let path = config::save(&cfg)?;

    let result = serde_json::json!({
        "saved": true,
        "path": path.display().to_string(),
        "api_key": config::mask_secret(cfg.api_key.as_deref().unwrap_or("")),
    });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} wrote config to {}", "+".green(), path.display());
        println!(
            "  run {} to verify the key works",
            "elevenlabs config check".bold()
        );
    });
    Ok(())
}
