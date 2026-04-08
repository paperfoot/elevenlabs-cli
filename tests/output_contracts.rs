//! Verify JSON envelope shape and stdout/stderr separation.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

#[test]
fn agent_info_is_raw_json_not_enveloped() {
    // agent-info is the schema definition itself — it's intentionally NOT
    // wrapped in the success envelope. Tests must verify that.
    let out = bin().arg("agent-info").output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        json.get("name").is_some(),
        "agent-info should have top-level name"
    );
    assert!(json.get("commands").is_some());
    // It should NOT be wrapped in a "data" field.
    let doubly_wrapped = json.get("data").and_then(|d| d.get("commands")).is_some();
    assert!(!doubly_wrapped, "agent-info must not be double-enveloped");
}

#[test]
fn success_envelope_shape_for_config_path() {
    let out = bin().args(["config", "path"]).output().unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "success");
    assert!(json["data"].is_object());
}

#[test]
fn error_envelope_shape_on_bad_input() {
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
        .args(["tts", "hello"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    // Error must go to stderr, not stdout.
    assert!(out.stdout.is_empty(), "errors must not leak to stdout");
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["status"], "error");
    assert!(json["error"]["code"].is_string());
    assert!(json["error"]["message"].is_string());
    assert!(json["error"]["suggestion"].is_string());
}

#[test]
fn json_flag_forces_json_on_tty() {
    // Even in a non-piped setup, --json forces JSON output.
    let out = bin().args(["--json", "agent-info"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("\"name\""));
}

#[test]
fn help_does_not_contain_error_envelope() {
    bin()
        .arg("--help")
        .assert()
        .code(0)
        .stdout(contains("Usage:").or(contains("usage")));
}
