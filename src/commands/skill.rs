//! Skill self-installation: write a minimal SKILL.md to Claude / Codex /
//! Gemini agent directories so they know the CLI exists.

use serde::Serialize;
use std::path::PathBuf;

use crate::error::AppError;
use crate::output::{self, Ctx};

fn skill_content() -> String {
    r#"---
name: elevenlabs
description: Generate speech, transcribe audio, make music or sound effects, manage voices and ElevenLabs agents or phone calls using the elevenlabs CLI.
---

# ElevenLabs CLI

Start with `elevenlabs --help` for the command index. Inspect only the command
or group needed for the task:

```bash
elevenlabs agent-info --command tts
elevenlabs agent-info --command "music compose"
elevenlabs voices --help
elevenlabs tts "Hello, world" -o hello.mp3
```

`agent-info` returns raw JSON. Other commands return compact JSON envelopes
when piped or with `--json`: `{version,status,data|error}`. Errors go to stderr.
Exit codes: 0 success, 1 runtime, 2 config/auth, 3 bad input, 4 rate limited.
Use `elevenlabs config check` to verify authentication. Full discovery remains
available with `elevenlabs agent-info`; it is usually unnecessary for one task.
"#
    .to_string()
}

struct SkillTarget {
    name: &'static str,
    path: PathBuf,
}

fn home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn skill_targets() -> Vec<SkillTarget> {
    let h = home();
    vec![
        SkillTarget {
            name: "Claude Code",
            path: h.join(".claude/skills/elevenlabs"),
        },
        SkillTarget {
            name: "Codex CLI",
            path: h.join(".codex/skills/elevenlabs"),
        },
        SkillTarget {
            name: "Gemini CLI",
            path: h.join(".gemini/skills/elevenlabs"),
        },
    ]
}

#[derive(Serialize)]
struct InstallResult {
    platform: String,
    path: String,
    status: String,
}

pub fn install(ctx: Ctx) -> Result<(), AppError> {
    let content = skill_content();
    let mut results: Vec<InstallResult> = Vec::new();

    for target in &skill_targets() {
        let skill_path = target.path.join("SKILL.md");

        if skill_path.exists() && std::fs::read_to_string(&skill_path).is_ok_and(|c| c == content) {
            results.push(InstallResult {
                platform: target.name.into(),
                path: skill_path.display().to_string(),
                status: "already_current".into(),
            });
            continue;
        }

        std::fs::create_dir_all(&target.path)?;
        std::fs::write(&skill_path, &content)?;
        results.push(InstallResult {
            platform: target.name.into(),
            path: skill_path.display().to_string(),
            status: "installed".into(),
        });
    }

    output::print_success_or(ctx, &results, |r| {
        use owo_colors::OwoColorize;
        for item in r {
            let marker = if item.status == "installed" {
                "+".green().to_string()
            } else {
                "=".dimmed().to_string()
            };
            println!(
                " {marker} {} -> {}",
                item.platform.bold(),
                item.path.dimmed()
            );
        }
    })?;

    Ok(())
}

#[derive(Serialize)]
struct SkillStatus {
    platform: String,
    installed: bool,
    current: bool,
    path: String,
}

pub fn status(ctx: Ctx) -> Result<(), AppError> {
    let content = skill_content();
    let mut results: Vec<SkillStatus> = Vec::new();

    for target in &skill_targets() {
        let skill_path = target.path.join("SKILL.md");
        let (installed, current) = if skill_path.exists() {
            let current = std::fs::read_to_string(&skill_path).is_ok_and(|c| c == content);
            (true, current)
        } else {
            (false, false)
        };
        results.push(SkillStatus {
            platform: target.name.into(),
            installed,
            current,
            path: skill_path.display().to_string(),
        });
    }

    output::print_success_or(ctx, &results, |r| {
        use owo_colors::OwoColorize;
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Platform", "Installed", "Current", "Path"]);
        for item in r {
            table.add_row(vec![
                item.platform.clone(),
                if item.installed {
                    "Yes".green().to_string()
                } else {
                    "No".red().to_string()
                },
                if item.current {
                    "Yes".green().to_string()
                } else {
                    "No".dimmed().to_string()
                },
                item.path.clone(),
            ]);
        }
        println!("{table}");
    })?;

    Ok(())
}
