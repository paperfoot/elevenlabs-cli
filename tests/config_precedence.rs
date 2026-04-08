//! Contract test: env var must win over the config file per the
//! documented precedence ladder (CLI flags > env > config > defaults).

use assert_cmd::Command;
use std::io::Write;
use std::path::PathBuf;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

/// Write a temp config.toml and return its path. The CLI respects the
/// ELEVENLABS_CLI_CONFIG env var as a full-path override, so these tests
/// work identically on Linux / macOS / Windows.
fn temp_config(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn extract_api_key(stdout: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(stdout).unwrap();
    v["data"]["api_key"].as_str().unwrap_or("").to_string()
}

#[test]
fn env_var_wins_over_config_file() {
    let (_dir, path) = temp_config("config_key_xxxxxxxxxxxx");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env("ELEVENLABS_API_KEY", "env_key_yyyyyyyyyyyyy")
        .args(["config", "show", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "config show should exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let masked = extract_api_key(&out.stdout);
    assert!(
        masked.starts_with("env_ke"),
        "expected env key to win, got masked={masked}"
    );
    assert!(
        !masked.contains("config"),
        "masked should not contain 'config_'"
    );
}

#[test]
fn config_file_wins_over_defaults_when_no_env() {
    let (_dir, path) = temp_config("config_key_aaaaaaaaaaaa");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .args(["config", "show", "--json"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "config show should exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let masked = extract_api_key(&out.stdout);
    assert!(
        masked.starts_with("config"),
        "expected config key, got masked={masked}"
    );
}

#[test]
fn config_override_path_is_respected() {
    let (_dir, path) = temp_config("override_key_zzzzzzz");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &path)
        .env_remove("ELEVENLABS_API_KEY")
        .args(["config", "path", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let reported = v["data"]["path"].as_str().unwrap_or("");
    assert_eq!(
        reported,
        path.to_string_lossy(),
        "override path must be respected"
    );
}
