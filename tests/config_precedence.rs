//! Contract test: env var must win over the config file per the
//! documented precedence ladder (CLI flags > env > config > defaults).

use assert_cmd::Command;
use std::io::Write;

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

/// Build a fake environment that redirects the config lookup to a temp
/// directory on every platform. Returns (tempdir, env_pairs_to_set).
/// We write config.toml to *every* candidate location and set the
/// matching env var so the `directories` crate resolves to our tempdir
/// regardless of OS.
fn fake_env_with_config(
    api_key: &str,
) -> (tempfile::TempDir, Vec<(&'static str, std::path::PathBuf)>) {
    let root = tempfile::tempdir().unwrap();
    let p = root.path();

    // macOS: ~/Library/Application Support/<app>/config.toml
    let mac_dir = p.join("Library/Application Support/elevenlabs-cli");
    // Linux: $XDG_CONFIG_HOME/<app>/config.toml or $HOME/.config/<app>/config.toml
    let linux_dir = p.join(".config/elevenlabs-cli");
    // Windows: %APPDATA%/<app>/config.toml
    let win_dir = p.join("AppData/Roaming/elevenlabs-cli");

    for dir in [&mac_dir, &linux_dir, &win_dir] {
        std::fs::create_dir_all(dir).unwrap();
        let mut f = std::fs::File::create(dir.join("config.toml")).unwrap();
        writeln!(f, "api_key = \"{api_key}\"").unwrap();
    }

    let envs: Vec<(&'static str, std::path::PathBuf)> = vec![
        ("HOME", p.to_path_buf()),
        ("XDG_CONFIG_HOME", linux_dir.parent().unwrap().to_path_buf()),
        ("APPDATA", p.join("AppData/Roaming")),
        ("LOCALAPPDATA", p.join("AppData/Local")),
        ("USERPROFILE", p.to_path_buf()),
    ];
    (root, envs)
}

fn run_config_show(
    envs: &[(&'static str, std::path::PathBuf)],
    extra_env: Option<(&str, &str)>,
    clear_api_key: bool,
) -> std::process::Output {
    let mut cmd = bin();
    for (k, v) in envs {
        cmd.env(k, v);
    }
    if clear_api_key {
        cmd.env_remove("ELEVENLABS_API_KEY")
            .env_remove("ELEVENLABS_CLI_API_KEY");
    }
    if let Some((k, v)) = extra_env {
        cmd.env(k, v);
    }
    cmd.args(["config", "show", "--json"]).output().unwrap()
}

fn extract_api_key(stdout: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(stdout).unwrap();
    v["data"]["api_key"].as_str().unwrap_or("").to_string()
}

#[test]
fn env_var_wins_over_config_file() {
    let (_root, envs) = fake_env_with_config("config_key_xxxxxxxxxxxx");
    let out = run_config_show(
        &envs,
        Some(("ELEVENLABS_API_KEY", "env_key_yyyyyyyyyyyyy")),
        false,
    );
    assert!(out.status.success(), "config show should exit 0");
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
    let (_root, envs) = fake_env_with_config("config_key_aaaaaaaaaaaa");
    let out = run_config_show(&envs, None, true);
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
