//! Skill self-installation: write a minimal SKILL.md to Claude / Codex /
//! Gemini agent directories so they know the CLI exists.

use serde::Serialize;
use std::path::PathBuf;

use crate::error::AppError;
use crate::output::{self, Ctx};

fn skill_content() -> String {
    r#"---
name: elevenlabs
description: >
  Use when the user asks to generate speech (TTS), transcribe audio (STT),
  make sound effects, clone or design voices, browse the ElevenLabs voice
  library, or manage conversational AI agents and phone calls via the
  ElevenLabs API. Run `elevenlabs agent-info` for the full capability
  manifest and every flag/exit-code the CLI supports.
---

# ElevenLabs CLI

Agent-friendly CLI wrapping the ElevenLabs AI audio platform. One binary
replaces the MCP server, covers the whole API surface, and auto-switches
between human-readable and JSON output.

## Quick start

```bash
# One-time auth
export ELEVENLABS_API_KEY=sk_...
elevenlabs config check

# Core commands
elevenlabs tts "Hello, world" -o hello.mp3
elevenlabs stt voicenote.m4a
elevenlabs sfx "waves crashing on a beach" --duration 5
elevenlabs voices list
elevenlabs agents list
elevenlabs user subscription
```

## Discovery

```bash
elevenlabs agent-info      # full machine-readable capability manifest
elevenlabs --help          # human help
```

All commands accept `--json` (auto-enabled when piped) and emit a consistent
envelope `{ version, status, data|error }`. Exit codes are semantic:
`0=ok, 1=transient, 2=config/auth, 3=bad input, 4=rate limited`.
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
    });

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
    });

    Ok(())
}
