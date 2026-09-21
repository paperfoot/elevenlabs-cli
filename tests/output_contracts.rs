//! Verify JSON envelope shape and stdout/stderr separation.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn assert_single_json_line(bytes: &[u8], label: &str) -> serde_json::Value {
    let text = std::str::from_utf8(bytes).unwrap();
    assert_eq!(
        text.trim_end().lines().count(),
        1,
        "{label} must emit one compact JSON line, got: {text:?}"
    );
    serde_json::from_slice(bytes).unwrap_or_else(|_| panic!("{label} must emit valid JSON"))
}

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

#[test]
fn per_command_invalid_input_suggestion_is_not_the_generic_default() {
    // `agents add-knowledge` rejects calls that don't supply at least one
    // of --url / --file / --text and attaches a command-specific
    // suggestion. If we ever regressed that into the generic
    // "Check arguments with: elevenlabs --help" fallback, this test fails.
    //
    // The framework-audit review (Claude) specifically called this out as
    // a P1: every InvalidInput should carry actionable recovery steps.
    // Using `agents add-knowledge` avoids needing a valid API key — the
    // validator fires before the client is ever built.

    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join("config.toml");
    std::fs::write(&cfg, b"api_key = \"sk_test_keyyyyyyyyy\"").unwrap();

    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        // Point at a bogus URL so any accidental network call fails fast.
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .args(["--json", "agents", "add-knowledge", "fake_agent", "fake_kb"])
        .output()
        .unwrap();

    assert_eq!(
        out.status.code(),
        Some(3),
        "missing source flags must fail with exit 3 (InvalidInput); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let env: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("error envelope must be valid JSON on stderr");
    assert_eq!(env["error"]["code"], "invalid_input");

    let suggestion = env["error"]["suggestion"].as_str().unwrap_or("");
    assert_ne!(
        suggestion, "Check arguments with: elevenlabs --help",
        "agents add-knowledge must attach a per-command suggestion, not the generic default"
    );
    assert!(
        suggestion.contains("--file")
            || suggestion.contains("--url")
            || suggestion.contains("--text"),
        "per-command suggestion must reference the missing flags concretely; got: {suggestion}"
    );
}

#[test]
fn machine_outputs_are_compact_single_line_json() {
    let tmp = tempfile::tempdir().unwrap();
    let config = tmp.path().join("config.toml");

    for (label, args) in [
        ("agent-info", vec!["agent-info"]),
        ("config path", vec!["config", "path"]),
        ("root help", vec!["--help"]),
        ("version", vec!["--version"]),
    ] {
        let output = bin()
            .env("ELEVENLABS_CLI_CONFIG", &config)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{label} must succeed");
        assert!(output.stderr.is_empty(), "{label} wrote to stderr");
        assert_single_json_line(&output.stdout, label);
    }

    let parse_error = bin().arg("--definitely-not-a-real-flag").output().unwrap();
    assert_eq!(parse_error.status.code(), Some(3));
    assert!(parse_error.stdout.is_empty());
    let error = assert_single_json_line(&parse_error.stderr, "parse error");
    assert_eq!(error["status"], "error");
}

#[cfg(unix)]
#[test]
fn closed_stdout_pipe_exits_one_with_json_error_instead_of_panicking() {
    use std::io::Read;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::process::Stdio;

    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader); // Ensure the child inherits a writer with no reader from process start.

    let writer_fd = OwnedFd::from(writer);
    let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin!("elevenlabs"));
    command
        .arg("agent-info")
        .stdout(Stdio::from(writer_fd))
        .stderr(Stdio::piped());

    let mut child = command.spawn().unwrap();
    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .expect("stderr must be piped")
        .read_to_end(&mut stderr)
        .unwrap();
    let status = child.wait().unwrap();

    assert_eq!(status.code(), Some(1));
    let text = String::from_utf8_lossy(&stderr);
    assert!(
        !text.contains("panicked"),
        "broken pipe caused panic: {text}"
    );
    let error = assert_single_json_line(&stderr, "closed stdout error");
    assert_eq!(error["status"], "error");
    assert!(error["error"]["code"].is_string());
    assert!(error["error"]["message"].is_string());
    assert!(error["error"]["suggestion"].is_string());
}
