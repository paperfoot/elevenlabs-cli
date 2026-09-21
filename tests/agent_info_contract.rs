//! Verify `agent-info` is machine-readable and advertises the features
//! the CLI actually ships.

use assert_cmd::Command;
use serde_json::{Map, Value};

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

fn agent_info() -> serde_json::Value {
    let out = bin().arg("agent-info").output().unwrap();
    assert!(out.status.success(), "agent-info must exit 0");
    serde_json::from_slice(&out.stdout).expect("agent-info must be valid JSON")
}

fn scoped_agent_info(command: &str) -> std::process::Output {
    bin()
        .args(["agent-info", "--command", command])
        .output()
        .unwrap()
}

fn command_map(info: &Value) -> &Map<String, Value> {
    info["commands"]
        .as_object()
        .expect("agent-info commands must be an object")
}

fn without_commands(mut info: Value) -> Value {
    info.as_object_mut()
        .expect("agent-info must be an object")
        .remove("commands");
    info
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

#[test]
fn advertises_current_stt_and_music_model_contracts() {
    let info = agent_info();
    let commands = info["commands"].as_object().unwrap();

    let stt = &commands["stt [file]"];
    assert!(
        stt["description"]
            .as_str()
            .unwrap_or("")
            .contains("scribe_v2_medical")
    );
    assert!(
        stt["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option == "--model <id>"),
        "STT model IDs must remain open for forwards compatibility"
    );

    for command in ["music compose [prompt]", "music plan <prompt>"] {
        assert!(
            commands[command]
                .as_str()
                .unwrap_or("")
                .contains("music_v2_5"),
            "{command} must advertise the current default model"
        );
    }

    let upload = commands["music upload <file>"].as_str().unwrap_or("");
    assert!(upload.contains("--extract-composition-plan"));
    assert!(upload.contains("--model"));
    assert!(upload.contains("music_v2_5"));
}

#[test]
fn current_model_flags_are_routable_in_help() {
    let stt = bin().args(["stt", "--help"]).output().unwrap();
    assert!(stt.status.success());
    let stt_help: serde_json::Value = serde_json::from_slice(&stt.stdout).unwrap();
    let stt_usage = stt_help["data"]["usage"].as_str().unwrap_or("");
    assert!(stt_usage.contains("scribe_v2"));
    assert!(stt_usage.contains("scribe_v2_medical"));

    for action in ["compose", "plan", "detailed", "stream"] {
        let output = bin().args(["music", action, "--help"]).output().unwrap();
        assert!(output.status.success(), "music {action} --help must route");
        let help: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            help["data"]["usage"]
                .as_str()
                .unwrap_or("")
                .contains("music_v2_5"),
            "music {action} help must name the current default model"
        );
    }

    let upload = bin().args(["music", "upload", "--help"]).output().unwrap();
    assert!(upload.status.success());
    let help: serde_json::Value = serde_json::from_slice(&upload.stdout).unwrap();
    let usage = help["data"]["usage"].as_str().unwrap_or("");
    assert!(usage.contains("--extract-composition-plan"));
    assert!(usage.contains("--model"));
    assert!(usage.contains("music_v2_5"));
}

#[test]
fn scoped_leaf_preserves_legacy_key_and_top_level_metadata() {
    let full = agent_info();
    let output = scoped_agent_info("tts");
    assert!(output.status.success());
    let scoped: Value = serde_json::from_slice(&output.stdout).unwrap();

    let commands = command_map(&scoped);
    assert_eq!(commands.len(), 1);
    assert!(commands.contains_key("tts <text>"));
    assert_eq!(without_commands(scoped), without_commands(full));
}

#[test]
fn scoped_group_returns_only_canonical_descendants() {
    let full = agent_info();
    let output = scoped_agent_info("music");
    assert!(output.status.success());
    let scoped: Value = serde_json::from_slice(&output.stdout).unwrap();
    let commands = command_map(&scoped);

    assert!(!commands.is_empty());
    assert!(commands.contains_key("music compose [prompt]"));
    assert!(commands.contains_key("music upload <file>"));
    assert!(commands.keys().all(|key| key.starts_with("music ")));

    let expected = command_map(&full)
        .iter()
        .filter(|(key, _)| key.starts_with("music "))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Map<_, _>>();
    assert_eq!(commands, &expected);
}

#[test]
fn scoped_nested_leaf_returns_one_command() {
    let output = scoped_agent_info("dubbing resource transcribe");
    assert!(output.status.success());
    let scoped: Value = serde_json::from_slice(&output.stdout).unwrap();
    let commands = command_map(&scoped);
    assert_eq!(commands.len(), 1);
    assert!(commands.contains_key("dubbing resource transcribe <dubbing_id>"));
}

#[test]
fn scoped_update_returns_one_command_despite_legacy_check_entry() {
    let output = scoped_agent_info("update");
    assert!(output.status.success());
    let scoped: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(command_map(&scoped).len(), 1);
    assert!(command_map(&scoped).contains_key("update"));
}

#[test]
fn scoped_discovery_works_through_info_alias_with_global_flags() {
    let output = bin()
        .args(["info", "--command", "tts", "--json", "--quiet"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let scoped: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(command_map(&scoped).len(), 1);
    assert!(command_map(&scoped).contains_key("tts <text>"));
}

#[test]
fn scoped_discovery_rejects_empty_unknown_alias_partial_and_extra_tokens() {
    for invalid in ["", "does-not-exist", "speak", "mus", "music compose extra"] {
        let output = scoped_agent_info(invalid);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{invalid:?} must be rejected: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "invalid filter {invalid:?} leaked to stdout"
        );
        let error: Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|_| panic!("invalid filter {invalid:?} must emit JSON on stderr"));
        assert_eq!(error["status"], "error");
        assert_eq!(error["error"]["code"], "invalid_input");
        assert!(error["error"]["suggestion"].is_string());
    }
}

#[test]
fn scoped_discovery_does_not_load_config_or_contact_the_api() {
    let tmp = tempfile::tempdir().unwrap();
    let malformed = tmp.path().join("config.toml");
    std::fs::write(&malformed, b"this is not = valid = toml [").unwrap();

    let output = bin()
        .env("ELEVENLABS_CLI_CONFIG", &malformed)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .args(["agent-info", "--command", "tts"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "discovery must be offline and config-independent: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(command_map(&info).contains_key("tts <text>"));
}

#[test]
fn every_manifest_command_key_resolves_to_real_canonical_help() {
    let info = agent_info();
    for manifest_key in command_map(&info).keys() {
        let path = manifest_key
            .split_whitespace()
            .take_while(|token| {
                !token.starts_with('<') && !token.starts_with('[') && !token.starts_with("--")
            })
            .collect::<Vec<_>>();
        assert!(!path.is_empty(), "empty command path for {manifest_key}");

        let tmp = tempfile::tempdir().unwrap();
        let malformed = tmp.path().join("config.toml");
        std::fs::write(&malformed, b"not valid toml = [").unwrap();
        let output = bin()
            .env("ELEVENLABS_CLI_CONFIG", &malformed)
            .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
            .env_remove("ELEVENLABS_API_KEY")
            .env_remove("ELEVENLABS_CLI_API_KEY")
            .args(&path)
            .arg("--help")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "manifest key {manifest_key:?} does not route through canonical path {path:?}: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
