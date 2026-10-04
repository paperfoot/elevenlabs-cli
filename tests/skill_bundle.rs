use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

const GUIDE: &str = include_str!("../docs/eleven-v4-prompting.md");
const TARGETS: [&str; 3] = [
    ".claude/skills/elevenlabs",
    ".codex/skills/elevenlabs",
    ".gemini/skills/elevenlabs",
];

fn bin(home: &Path) -> Command {
    let mut command = Command::cargo_bin("elevenlabs").unwrap();
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ELEVENLABS_CLI_CONFIG", home.join("config.toml"))
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY");
    command
}

fn run(home: &Path, action: &str) -> Vec<Value> {
    let output = bin(home)
        .args(["--json", "skill", action])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "skill {action} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["status"], "success");
    let data = envelope["data"].as_array().unwrap().clone();
    assert_eq!(data.len(), TARGETS.len());
    data
}

fn target_paths(home: &Path) -> Vec<PathBuf> {
    TARGETS.iter().map(|target| home.join(target)).collect()
}

fn assert_bundle(target: &Path) {
    let skill = std::fs::read_to_string(target.join("SKILL.md")).unwrap();
    assert!(skill.contains("references/eleven-v4-prompting.md"));
    assert_eq!(
        std::fs::read(target.join("references/eleven-v4-prompting.md")).unwrap(),
        GUIDE.as_bytes()
    );
}

#[test]
fn install_status_and_repair_cover_the_complete_skill_bundle() {
    let home = tempfile::tempdir().unwrap();
    let targets = target_paths(home.path());

    let installed = run(home.path(), "install");
    assert_eq!(installed.len(), targets.len());
    assert!(installed.iter().all(|item| item["status"] == "installed"));
    for target in &targets {
        assert_bundle(target);
    }

    let current = run(home.path(), "status");
    assert!(current.iter().all(|item| item["installed"] == true));
    assert!(current.iter().all(|item| item["current"] == true));

    let unchanged = run(home.path(), "install");
    assert!(
        unchanged
            .iter()
            .all(|item| item["status"] == "already_current")
    );

    std::fs::remove_file(targets[0].join("references/eleven-v4-prompting.md")).unwrap();
    std::fs::write(
        targets[1].join("references/eleven-v4-prompting.md"),
        "stale guide",
    )
    .unwrap();

    let stale = run(home.path(), "status");
    assert_eq!(stale[0]["current"], false);
    assert_eq!(stale[1]["current"], false);
    assert_eq!(stale[2]["current"], true);

    run(home.path(), "install");
    for target in &targets {
        assert_bundle(target);
    }
    assert!(
        run(home.path(), "status")
            .iter()
            .all(|item| item["current"] == true)
    );
}
