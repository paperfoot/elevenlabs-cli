//! `doctor` — structured dependency + environment diagnostics.
//!
//! Prints a machine-readable list of checks covering config, auth,
//! env-var shadowing, API-key scope, network reachability, ffmpeg, disk
//! writeability, and the optional `default_output_dir` setting.
//!
//! Exit code:
//! - `0` if every check passes or only warns (non-blocking)
//! - `2` if any check fails (config / auth error — fix setup)
//!
//! The command never bubbles network errors as `AppError`: when an HTTP
//! probe fails we record it as a `fail` check so the diagnostic report
//! stays complete. The only `AppError` paths here are genuine programmer
//! or local-IO errors (e.g. cannot create a temp file in the cwd because
//! the filesystem itself is broken).

use std::collections::HashSet;
use std::time::Duration;

use serde::Serialize;

use crate::client::ElevenLabsClient;
use crate::config::{self, AppConfig, AuthSource};
use crate::error::AppError;
use crate::output::{self, Ctx};

/// Parsed CLI args. Kept local to this module so the clap `DoctorArgs`
/// in `cli.rs` is the single source of truth for the flag surface — this
/// struct is a private mirror used by `run()`.
#[derive(Debug, Clone)]
pub struct DoctorOptions {
    pub skip: Vec<String>,
    pub timeout_ms: u64,
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self {
            skip: Vec::new(),
            timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Summary {
    pub pass: u32,
    pub warn: u32,
    pub fail: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub checks: Vec<Check>,
    pub summary: Summary,
}

/// Canonical check names. Keep these stable — tests and agents key off them.
pub const CHECK_CONFIG_FILE: &str = "config_file";
pub const CHECK_API_KEY: &str = "api_key";
pub const CHECK_ENV_SHADOW: &str = "env_shadow";
pub const CHECK_API_KEY_SCOPE: &str = "api_key_scope";
pub const CHECK_NETWORK: &str = "network";
pub const CHECK_FFMPEG: &str = "ffmpeg";
pub const CHECK_DISK_WRITE: &str = "disk_write";
pub const CHECK_OUTPUT_DIR: &str = "output_dir";

#[allow(dead_code)]
pub const ALL_CHECKS: &[&str] = &[
    CHECK_CONFIG_FILE,
    CHECK_API_KEY,
    CHECK_ENV_SHADOW,
    CHECK_API_KEY_SCOPE,
    CHECK_NETWORK,
    CHECK_FFMPEG,
    CHECK_DISK_WRITE,
    CHECK_OUTPUT_DIR,
];

// ── Entry point ────────────────────────────────────────────────────────────

pub async fn run(ctx: Ctx, opts: DoctorOptions) -> Result<(), AppError> {
    let report = collect(&opts).await;
    let had_fail = report.summary.fail > 0;

    output::print_success_or(ctx, &report, |r| human_print(ctx, r))?;

    if had_fail {
        // `doctor` is the one place where we intentionally bypass the
        // `AppError` → `main.rs` envelope flow: the diagnostic report
        // IS the success envelope (pass/warn/fail rows with suggestions),
        // so wrapping the "some checks failed" outcome in a second
        // `AppError` envelope would print the same findings twice and
        // force agents to parse two shapes for one command.
        //
        // The clean alternative — a dedicated `AppError::DoctorFailed`
        // variant that `output::print_error` special-cases to print
        // nothing — requires changes to `error.rs` and `output.rs`,
        // which are owned by Fixer α in this phase. See
        // plans/cli-snippets/fixes/delta/NOTES.md for the follow-up
        // ticket. For now, the direct `exit(2)` is the pragmatic choice:
        // code 2 = config/auth error (which is what a failing check is),
        // and nothing is printed after the report.
        std::process::exit(2);
    }
    Ok(())
}

/// Gather every requested check. Order matches `ALL_CHECKS` so output
/// is deterministic for tests.
pub async fn collect(opts: &DoctorOptions) -> Report {
    let skip: HashSet<&str> = opts.skip.iter().map(String::as_str).collect();
    let cfg = config::load().ok();
    let mut checks: Vec<Check> = Vec::new();

    if !skip.contains(CHECK_CONFIG_FILE) {
        checks.push(check_config_file());
    }
    if !skip.contains(CHECK_API_KEY) {
        checks.push(check_api_key(cfg.as_ref()));
    }
    if !skip.contains(CHECK_ENV_SHADOW) {
        checks.push(check_env_shadow(cfg.as_ref()));
    }
    if !skip.contains(CHECK_API_KEY_SCOPE) {
        checks.push(check_api_key_scope(cfg.as_ref(), opts.timeout_ms).await);
    }
    if !skip.contains(CHECK_NETWORK) {
        checks.push(check_network(opts.timeout_ms).await);
    }
    if !skip.contains(CHECK_FFMPEG) {
        checks.push(check_ffmpeg());
    }
    if !skip.contains(CHECK_DISK_WRITE) {
        checks.push(check_disk_write());
    }
    if !skip.contains(CHECK_OUTPUT_DIR) {
        checks.push(check_output_dir(cfg.as_ref()));
    }

    let mut summary = Summary::default();
    for c in &checks {
        match c.status {
            CheckStatus::Pass => summary.pass += 1,
            CheckStatus::Warn => summary.warn += 1,
            CheckStatus::Fail => summary.fail += 1,
        }
    }

    Report { checks, summary }
}

// ── Individual checks ──────────────────────────────────────────────────────

fn check_config_file() -> Check {
    let path = config::config_path();
    let path_str = path.display().to_string();

    if !path.exists() {
        return Check {
            name: CHECK_CONFIG_FILE.into(),
            status: CheckStatus::Warn,
            detail: format!("config file not found at {path_str}"),
            suggestion: "Save your API key with: elevenlabs config init --api-key <sk_...>".into(),
        };
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<toml::Value>(&text) {
            Ok(_) => Check {
                name: CHECK_CONFIG_FILE.into(),
                status: CheckStatus::Pass,
                detail: format!("valid TOML at {path_str}"),
                suggestion: String::new(),
            },
            Err(e) => Check {
                name: CHECK_CONFIG_FILE.into(),
                status: CheckStatus::Fail,
                detail: format!("malformed TOML at {path_str}: {e}"),
                suggestion: format!(
                    "Fix or remove the config file: rm {path_str} && \
                     elevenlabs config init --api-key <sk_...>"
                ),
            },
        },
        Err(e) => Check {
            name: CHECK_CONFIG_FILE.into(),
            status: CheckStatus::Fail,
            detail: format!("cannot read config at {path_str}: {e}"),
            suggestion: format!("Check file permissions: ls -l {path_str}"),
        },
    }
}

fn check_api_key(cfg: Option<&AppConfig>) -> Check {
    let state = cfg
        .map(AppConfig::auth_key_state)
        .unwrap_or_else(|| config::AuthKeyState::snapshot(None));
    match state.effective_source() {
        AuthSource::File => Check {
            name: CHECK_API_KEY.into(),
            status: CheckStatus::Pass,
            detail: "API key present (source: config file)".into(),
            suggestion: String::new(),
        },
        AuthSource::Env => Check {
            name: CHECK_API_KEY.into(),
            status: CheckStatus::Pass,
            detail: "API key present (source: env)".into(),
            suggestion: String::new(),
        },
        AuthSource::None => Check {
            name: CHECK_API_KEY.into(),
            status: CheckStatus::Fail,
            detail: "no API key found (source: none)".into(),
            suggestion: "Set your API key: elevenlabs config init --api-key <sk_...>  \
                 or: export ELEVENLABS_API_KEY=sk_..."
                .into(),
        },
    }
}

fn check_env_shadow(cfg: Option<&AppConfig>) -> Check {
    let state = cfg
        .map(AppConfig::auth_key_state)
        .unwrap_or_else(|| config::AuthKeyState::snapshot(None));
    if state.env_ignored_by_file() {
        let env_masked = state.env_key.as_deref().map(config::mask_secret);
        let file_masked = state.file_key.as_deref().map(config::mask_secret);
        return Check {
            name: CHECK_ENV_SHADOW.into(),
            status: CheckStatus::Warn,
            detail: format!(
                "env overrides config? No — since v0.1.6 config wins; env is fallback only. \
                 Note the mismatch: file={} env={}",
                file_masked.unwrap_or_else(|| "(unset)".into()),
                env_masked.unwrap_or_else(|| "(unset)".into()),
            ),
            suggestion: "If the env value is the correct one, overwrite the saved file: \
                 elevenlabs config set api_key <value>. Otherwise unset the env: \
                 unset ELEVENLABS_API_KEY"
                .into(),
        };
    }
    // Informational pass — env and file either agree or only one is set.
    let detail = match (state.env_key.as_deref(), state.file_key.as_deref()) {
        (Some(_), Some(_)) => "env and config file match — no shadowing".into(),
        (Some(_), None) => {
            "only ELEVENLABS_API_KEY env var is set; no config file key to shadow".into()
        }
        (None, Some(_)) => "only config file key is set; env is not set — no shadowing risk".into(),
        (None, None) => "neither env nor config file has an api_key".into(),
    };
    Check {
        name: CHECK_ENV_SHADOW.into(),
        status: CheckStatus::Pass,
        detail,
        suggestion: String::new(),
    }
}

async fn check_api_key_scope(cfg: Option<&AppConfig>, timeout_ms: u64) -> Check {
    let Some(cfg) = cfg else {
        return Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: "cannot load config".into(),
            suggestion: "Fix config errors: elevenlabs config show".into(),
        };
    };
    if cfg.resolve_api_key().is_none() {
        return Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: "skipped: no API key configured".into(),
            suggestion: "Set your API key: elevenlabs config init --api-key <sk_...>".into(),
        };
    }
    let client = match ElevenLabsClient::from_config(cfg) {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: CHECK_API_KEY_SCOPE.into(),
                status: CheckStatus::Fail,
                detail: format!("cannot build client: {e}"),
                suggestion: "Fix config errors: elevenlabs config show".into(),
            };
        }
    };

    // Probe /v1/user and /v1/voices independently so we can detect
    // restricted keys (voices ok, user denied = scope missing).
    let timeout = Duration::from_millis(timeout_ms);
    let user_ok = probe_status(&client, "/v1/user", timeout).await;
    let voices_ok = probe_status(&client, "/v1/voices", timeout).await;

    match (user_ok, voices_ok) {
        (ProbeOutcome::Ok, ProbeOutcome::Ok) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Pass,
            detail: "both /v1/user and /v1/voices returned 2xx".into(),
            suggestion: String::new(),
        },
        (ProbeOutcome::Forbidden(code_u), ProbeOutcome::Ok) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Warn,
            detail: format!(
                "restricted key: /v1/user returned {code_u} but /v1/voices is ok. \
                 The user_read scope is not granted."
            ),
            suggestion: "If you need user info, re-issue a key at \
                 https://elevenlabs.io/app/settings/api-keys with user_read scope"
                .into(),
        },
        (_, ProbeOutcome::Forbidden(code_v)) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: format!(
                "/v1/voices returned {code_v} — key is invalid or lacks voices_read scope"
            ),
            suggestion:
                "Re-issue a working key at https://elevenlabs.io/app/settings/api-keys and run: \
                 elevenlabs config init --api-key <sk_...>"
                    .into(),
        },
        (ProbeOutcome::Error(e_u), ProbeOutcome::Error(e_v)) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: format!("both scope probes errored: user={e_u}; voices={e_v}"),
            suggestion: "Check network reachability to api.elevenlabs.io and retry: \
                 elevenlabs doctor"
                .into(),
        },
        (ProbeOutcome::Ok, ProbeOutcome::Error(e_v)) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: format!("/v1/voices errored: {e_v}"),
            suggestion: "Retry; check status.elevenlabs.io if it persists".into(),
        },
        (ProbeOutcome::Error(e_u), ProbeOutcome::Ok) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Warn,
            detail: format!(
                "/v1/user errored ({e_u}) but /v1/voices is ok. Likely a transient issue or a \
                 restricted scope — most commands will still work."
            ),
            suggestion: "Retry `elevenlabs doctor`; if /v1/user keeps failing, \
                        the key may lack user_read scope"
                .into(),
        },
        (ProbeOutcome::Forbidden(code_u), ProbeOutcome::Error(e_v)) => Check {
            name: CHECK_API_KEY_SCOPE.into(),
            status: CheckStatus::Fail,
            detail: format!("/v1/user denied ({code_u}) and /v1/voices errored: {e_v}"),
            suggestion: "Re-issue a working key at https://elevenlabs.io/app/settings/api-keys"
                .into(),
        },
    }
}

async fn check_network(timeout_ms: u64) -> Check {
    // If the caller overrode the API base URL, honour that — otherwise
    // probe the canonical ElevenLabs host.
    //
    // We probe `/v1/models` rather than `/` because the root returns 404
    // pre-auth — a misleading signal to a user skimming the report. The
    // `/v1/models` endpoint returns 200 unauthenticated, so a success
    // here unambiguously proves DNS + TCP + TLS + HTTP routing all work
    // without needing an API key. That also keeps this check strictly
    // about network reachability — key validity is covered separately by
    // `check_api_key_scope`.
    let (url_base, overridden) = match std::env::var("ELEVENLABS_API_BASE_URL") {
        Ok(v) if !v.trim().is_empty() => (v.trim().to_string(), true),
        _ => (crate::client::DEFAULT_BASE_URL.to_string(), false),
    };
    let url = format!("{}/v1/models", url_base.trim_end_matches('/'));

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .connect_timeout(Duration::from_millis(timeout_ms))
        .user_agent(crate::client::USER_AGENT)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Check {
                name: CHECK_NETWORK.into(),
                status: CheckStatus::Fail,
                detail: format!("failed to build HTTP client: {e}"),
                suggestion: "Check TLS/system CA availability".into(),
            };
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            let note = if overridden {
                format!(" (overridden via ELEVENLABS_API_BASE_URL={url_base})")
            } else {
                String::new()
            };
            // Any response proves DNS + TCP + TLS work. 200 on /v1/models
            // is the happy path; 4xx/5xx still indicates reachability.
            Check {
                name: CHECK_NETWORK.into(),
                status: CheckStatus::Pass,
                detail: format!("GET {url} returned {code}{note}"),
                suggestion: String::new(),
            }
        }
        Err(e) => {
            let note = if overridden {
                format!(" (overridden via ELEVENLABS_API_BASE_URL={url_base})")
            } else {
                String::new()
            };
            Check {
                name: CHECK_NETWORK.into(),
                status: CheckStatus::Fail,
                detail: format!("cannot reach {url}: {e}{note}"),
                suggestion:
                    "Check your internet connection and any HTTP proxy (HTTPS_PROXY env var). \
                     If you're behind a corporate firewall, allow api.elevenlabs.io:443."
                        .into(),
            }
        }
    }
}

fn check_ffmpeg() -> Check {
    // Cross-platform `which` via spawning `ffmpeg -version`. Cheaper than
    // probing $PATH entries by hand and matches whatever the user's shell
    // would resolve.
    let result = std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();
    match result {
        Ok(out) if out.status.success() => {
            // First line is usually `ffmpeg version X …`. Include it so
            // the report shows the installed version.
            let first = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            Check {
                name: CHECK_FFMPEG.into(),
                status: CheckStatus::Pass,
                detail: if first.is_empty() {
                    "ffmpeg is installed".into()
                } else {
                    first
                },
                suggestion: String::new(),
            }
        }
        _ => Check {
            name: CHECK_FFMPEG.into(),
            status: CheckStatus::Warn,
            detail: "ffmpeg not found on PATH — required for STT/audio conversions on some inputs"
                .into(),
            suggestion: "Install ffmpeg: brew install ffmpeg (macOS) / \
                        apt install ffmpeg (Debian) / winget install ffmpeg (Windows)"
                .into(),
        },
    }
}

fn check_disk_write() -> Check {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return Check {
                name: CHECK_DISK_WRITE.into(),
                status: CheckStatus::Fail,
                detail: format!("cannot read cwd: {e}"),
                suggestion: "Change to a writable directory and retry".into(),
            };
        }
    };
    write_probe(&cwd, "cwd")
}

fn check_output_dir(cfg: Option<&AppConfig>) -> Check {
    let dir = cfg.and_then(|c| c.defaults.output_dir.clone());
    let Some(dir) = dir else {
        return Check {
            name: CHECK_OUTPUT_DIR.into(),
            status: CheckStatus::Pass,
            detail: "default_output_dir not set — generated files go to cwd".into(),
            suggestion: String::new(),
        };
    };
    let path = std::path::PathBuf::from(&dir);
    if !path.exists() {
        return Check {
            name: CHECK_OUTPUT_DIR.into(),
            status: CheckStatus::Fail,
            detail: format!("default_output_dir does not exist: {dir}"),
            suggestion: format!("Create the directory: mkdir -p {dir}"),
        };
    }
    if !path.is_dir() {
        return Check {
            name: CHECK_OUTPUT_DIR.into(),
            status: CheckStatus::Fail,
            detail: format!("default_output_dir is not a directory: {dir}"),
            suggestion: "Point defaults.output_dir at a real directory: \
                 elevenlabs config set defaults.output_dir <path>"
                .into(),
        };
    }
    write_probe(&path, &format!("default_output_dir ({dir})"))
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn write_probe(dir: &std::path::Path, label: &str) -> Check {
    let probe = dir.join(format!(".elevenlabs-doctor-{}.tmp", std::process::id()));
    match std::fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Check {
                name: if label.starts_with("default_output_dir") {
                    CHECK_OUTPUT_DIR.into()
                } else {
                    CHECK_DISK_WRITE.into()
                },
                status: CheckStatus::Pass,
                detail: format!("{label} is writable ({})", dir.display()),
                suggestion: String::new(),
            }
        }
        Err(e) => Check {
            name: if label.starts_with("default_output_dir") {
                CHECK_OUTPUT_DIR.into()
            } else {
                CHECK_DISK_WRITE.into()
            },
            status: CheckStatus::Fail,
            detail: format!("cannot write to {label} ({}): {e}", dir.display()),
            suggestion: format!(
                "Check permissions on {}: ls -ld {}",
                dir.display(),
                dir.display(),
            ),
        },
    }
}

#[derive(Debug, Clone)]
enum ProbeOutcome {
    Ok,
    /// 401 / 403 — auth succeeded on the wire but the scope is insufficient
    /// (or the key is wholly rejected).
    Forbidden(u16),
    /// Transport error, timeout, 5xx, or an unexpected shape.
    Error(String),
}

async fn probe_status(client: &ElevenLabsClient, path: &str, timeout: Duration) -> ProbeOutcome {
    match tokio::time::timeout(timeout, client.http.get(client.url(path)).send()).await {
        Err(_) => ProbeOutcome::Error(format!("timed out after {}ms", timeout.as_millis())),
        Ok(Err(e)) => ProbeOutcome::Error(e.to_string()),
        Ok(Ok(resp)) => {
            let code = resp.status().as_u16();
            if resp.status().is_success() {
                ProbeOutcome::Ok
            } else if code == 401 || code == 403 {
                ProbeOutcome::Forbidden(code)
            } else {
                ProbeOutcome::Error(format!("HTTP {code}"))
            }
        }
    }
}

// ── Human output ───────────────────────────────────────────────────────────

fn human_print(ctx: Ctx, report: &Report) {
    if ctx.quiet {
        return;
    }
    use owo_colors::OwoColorize;
    let mut t = comfy_table::Table::new();
    t.set_header(vec!["", "Check", "Detail"]);
    for c in &report.checks {
        let (icon, icon_colored) = match c.status {
            CheckStatus::Pass => ("\u{2713}", "\u{2713}".green().to_string()),
            CheckStatus::Warn => ("\u{26A0}", "\u{26A0}".yellow().to_string()),
            CheckStatus::Fail => ("\u{2717}", "\u{2717}".red().to_string()),
        };
        let _ = icon; // silence unused warning when tty lacks color
        t.add_row(vec![icon_colored, c.name.clone(), c.detail.clone()]);
    }
    println!("{t}");
    println!(
        "\n{} {} pass, {} warn, {} fail",
        "Summary:".bold(),
        report.summary.pass.to_string().green(),
        report.summary.warn.to_string().yellow(),
        report.summary.fail.to_string().red(),
    );
    // Per-fail suggestion, each on its own line so the output is grep-able.
    for c in &report.checks {
        if matches!(c.status, CheckStatus::Fail) {
            println!("  {} {}: {}", "fix:".red().bold(), c.name, c.suggestion);
        }
    }
}
