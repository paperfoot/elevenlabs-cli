//! Verify `agent-info` is machine-readable and advertises the features
//! the CLI actually ships.

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

fn agent_info() -> serde_json::Value {
    let out = bin().arg("agent-info").output().unwrap();
    assert!(out.status.success(), "agent-info must exit 0");
    serde_json::from_slice(&out.stdout).expect("agent-info must be valid JSON")
}

#[test]
fn has_required_top_level_fields() {
    let info = agent_info();
    assert!(info["name"].is_string());
    assert!(info["binary"].is_string());
    assert!(info["version"].is_string());
    assert!(info["description"].is_string());
    assert!(info["commands"].is_object());
    assert!(info["exit_codes"].is_object());
    assert!(info["envelope"].is_object());
    assert!(info["auto_json_when_piped"].as_bool().unwrap_or(false));
    assert!(info["requires_api_key"].as_bool().unwrap_or(false));
}

#[test]
fn name_matches_binary() {
    let info = agent_info();
    assert_eq!(info["binary"], "elevenlabs");
}

#[test]
fn all_five_exit_codes_documented() {
    let info = agent_info();
    let codes = &info["exit_codes"];
    for code in ["0", "1", "2", "3", "4"] {
        assert!(
            codes[code].is_string(),
            "exit code {code} must be documented"
        );
    }
}

#[test]
fn advertises_core_tts_commands() {
    let info = agent_info();
    let commands = info["commands"].as_object().unwrap();
    assert!(commands.contains_key("tts <text>"), "missing tts");
    assert!(commands.contains_key("stt [file]"), "missing stt");
    assert!(commands.contains_key("sfx <text>"), "missing sfx");
}

#[test]
fn advertises_voices_subcommands() {
    let info = agent_info();
    let commands = info["commands"].as_object().unwrap();
    assert!(commands.contains_key("voices list"));
    assert!(commands.contains_key("voices show <voice_id>"));
}

#[test]
fn advertises_agents_subcommands() {
    let info = agent_info();
    let commands = info["commands"].as_object().unwrap();
    assert!(commands.contains_key("agents list"));
    assert!(commands.contains_key("agents create <name>"));
}

#[test]
fn advertises_framework_commands() {
    let info = agent_info();
    let commands = info["commands"].as_object().unwrap();
    assert!(commands.contains_key("config show"));
    assert!(commands.contains_key("config path"));
    assert!(commands.contains_key("skill install"));
    assert!(commands.contains_key("update"));
}

// ── Routability: every command listed must actually route ─────────────────

#[test]
fn agent_info_is_routable() {
    bin().arg("agent-info").assert().code(0);
}

#[test]
fn info_alias_is_routable() {
    bin().arg("info").assert().code(0);
}

#[test]
fn skill_status_is_routable() {
    let tmp = tempfile::tempdir().unwrap();
    bin()
        .env("HOME", tmp.path())
        .args(["skill", "status"])
        .assert()
        .code(0);
}

#[test]
fn config_path_is_routable() {
    bin().args(["config", "path"]).assert().code(0);
}
