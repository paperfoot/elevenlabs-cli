//! Verify semantic exit codes (0, 2, 3). Codes 1 and 4 require mocking
//! the network so we only verify them indirectly via agent-info.

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

#[test]
fn help_exits_0() {
    bin().arg("--help").assert().code(0);
}

#[test]
fn version_exits_0() {
    bin().arg("--version").assert().code(0);
}

#[test]
fn agent_info_exits_0() {
    bin().arg("agent-info").assert().code(0);
}

#[test]
fn agent_info_alias_exits_0() {
    bin().arg("info").assert().code(0);
}

#[test]
fn unknown_command_exits_3() {
    bin().arg("definitely-not-a-subcommand").assert().code(3);
}

#[test]
fn missing_required_arg_exits_3() {
    bin().arg("tts").assert().code(3);
}

#[test]
fn invalid_flag_value_exits_3() {
    bin()
        .args(["tts", "hello", "--stability", "not-a-number"])
        .assert()
        .code(3);
}

#[test]
fn missing_api_key_exits_2() {
    // Point HOME at a temp dir so no config file is found and env is unset.
    let tmp = tempfile::tempdir().unwrap();
    bin()
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join(".config"))
        .args(["tts", "hello"])
        .assert()
        .code(2);
}
