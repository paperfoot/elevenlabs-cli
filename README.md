<div align="center">

<img src="./assets/banner.svg" alt="elevenlabs-cli — agent-friendly Rust CLI for the ElevenLabs AI audio platform" width="100%"/>

# `elevenlabs-cli`

**One Rust binary. Every [ElevenLabs](https://elevenlabs.io) endpoint. No MCP server, no Python runtime, no drift.**

TTS • STT • Sound Effects • Voice Cloning • Voice Design • Conversational Agents • Music Generation • Phone Calls — all from your terminal, all machine-readable.

[![Star this repo](https://img.shields.io/github/stars/199-biotechnologies/elevenlabs-cli?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/199-biotechnologies/elevenlabs-cli/stargazers)
&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

[![crates.io](https://img.shields.io/crates/v/elevenlabs-cli?style=for-the-badge&logo=rust&color=orange)](https://crates.io/crates/elevenlabs-cli)
[![Downloads](https://img.shields.io/crates/d/elevenlabs-cli?style=for-the-badge&logo=rust&color=orange)](https://crates.io/crates/elevenlabs-cli)
[![CI](https://img.shields.io/github/actions/workflow/status/199-biotechnologies/elevenlabs-cli/ci.yml?style=for-the-badge&logo=github-actions&label=CI)](https://github.com/199-biotechnologies/elevenlabs-cli/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![MSRV 1.85+](https://img.shields.io/badge/MSRV-1.85%2B-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-Welcome-brightgreen?style=for-the-badge)](#contributing)

[Install](#install) • [Quick start](#quick-start) • [Commands](#commands) • [Why](#why-a-cli-instead-of-an-mcp-server) • [For agents](#for-ai-agents) • [Config](#configuration)

</div>

---

## What it does

A single ~5 MB Rust binary that exposes the entire ElevenLabs platform on the command line:

```bash
elevenlabs tts "Hello, world"                          # Text → speech
elevenlabs stt voicenote.m4a                           # Speech → text
elevenlabs sfx "rain on a tin roof" --duration 4       # Sound effects
elevenlabs voices clone "My Voice" sample1.wav sample2.wav
elevenlabs voices design "a gruff old lighthouse keeper"
elevenlabs audio isolate noisy.mp3                     # Extract clean speech
elevenlabs audio convert me.mp3 --voice "Rachel"       # Voice-to-voice
elevenlabs music compose "lofi chill for a rainy sunday"
elevenlabs agents create "triage-bot" --system-prompt "You are a friendly support agent..."
elevenlabs agents list
elevenlabs conversations show conv_abc123
elevenlabs phone call agent_xxx --from-id phnum_yyy --to +14155551234
```

Every command auto-switches between **coloured human output** (terminal) and **JSON envelopes** (piped / `--json`). Exit codes are semantic (`0=ok, 1=transient, 2=config, 3=bad input, 4=rate limited`). Errors carry a machine-readable `code` and an actionable `suggestion` an AI agent can follow literally.

---

## Install

```bash
# cargo (Linux, macOS, Windows)
cargo install elevenlabs-cli

# Homebrew (macOS, Linux)
brew tap 199-biotechnologies/tap
brew install elevenlabs-cli

# Prebuilt binaries — Linux, macOS (x86_64 + arm64), Windows
curl -L https://github.com/199-biotechnologies/elevenlabs-cli/releases/latest/download/elevenlabs-$(uname -s)-$(uname -m).tar.gz | tar xz
sudo mv elevenlabs /usr/local/bin/
```

Self-update once installed:

```bash
elevenlabs update --check
elevenlabs update
```

---

## Quick start

```bash
# 1. Auth
export ELEVENLABS_API_KEY=sk_...
# or persist it:
elevenlabs config init --api-key sk_...
elevenlabs config check        # verify the key works

# 2. Generate speech
elevenlabs tts "The quick brown fox" -o fox.mp3
elevenlabs tts "$(cat long-script.txt)" --voice "Rachel" --model eleven_multilingual_v2 -o narration.mp3

# 3. Transcribe
elevenlabs stt interview.m4a --diarize -o transcript.txt

# 4. Browse voices
elevenlabs voices list
elevenlabs voices library --gender female --accent british

# 5. Install the skill so Claude/Codex/Gemini know about the CLI
elevenlabs skill install

# 6. For AI agents — get the full capability manifest
elevenlabs agent-info
```

---

## Commands

<details>
<summary><b>Speech synthesis &amp; transcription</b></summary>

```bash
# Text-to-speech
elevenlabs tts <text> [-o path] [--voice name | --voice-id id] [--model id]
                     [--format mp3_44100_128] [--stability 0.5] [--similarity 0.75]
                     [--style 0.0] [--speed 1.0] [--language en] [--stdout]

# Speech-to-text (Scribe v1)
elevenlabs stt <file> [-o path] [--language iso] [--diarize] [--timestamps] [--audio-events]

# Sound effects
elevenlabs sfx <prompt> [-o path] [--duration 0.5..22] [--prompt-influence 0.3] [--loop]
```

</details>

<details>
<summary><b>Voices (library, search, clone, design)</b></summary>

```bash
elevenlabs voices list [--search TERM] [--limit N]
elevenlabs voices show <voice_id>
elevenlabs voices search <query>
elevenlabs voices library [--search TERM] [--gender female] [--accent british]
                          [--age young|middle_aged|old] [--language en] [--use-case narration]
elevenlabs voices clone <name> <files...> [--description "..."]
elevenlabs voices design <description> [--text "read this"] [--output-dir ./previews]
elevenlabs voices save-preview <generated_voice_id> <name> <description>
elevenlabs voices delete <voice_id>     # alias: rm
```

</details>

<details>
<summary><b>Audio transforms</b></summary>

```bash
elevenlabs audio isolate <file> [-o path]
elevenlabs audio convert <file> [--voice NAME | --voice-id ID] [--model ID] [-o path]
```

</details>

<details>
<summary><b>Music generation</b></summary>

```bash
elevenlabs music compose <prompt> [--length-ms 30000] [-o path]
elevenlabs music plan <prompt> [--length-ms 30000]
```

</details>

<details>
<summary><b>Conversational AI agents</b></summary>

```bash
elevenlabs agents list                  # alias: ls
elevenlabs agents show <agent_id>        # alias: get
elevenlabs agents create <name>
    --system-prompt "..."
    [--first-message "Hi, how can I help?"]
    [--voice-id ID] [--language en] [--llm gemini-2.0-flash-001]
    [--temperature 0.5] [--model-id eleven_turbo_v2]
elevenlabs agents delete <agent_id>      # alias: rm
elevenlabs agents add-knowledge <agent_id> <name> (--url URL | --file PATH | --text "...")

elevenlabs conversations list [--agent-id ID] [--page-size 30] [--cursor TOKEN]
elevenlabs conversations show <conversation_id>
```

</details>

<details>
<summary><b>Phone calls (outbound via Twilio/SIP)</b></summary>

```bash
elevenlabs phone list
elevenlabs phone call <agent_id> --from-id <phone_number_id> --to +14155551234
```

</details>

<details>
<summary><b>User, history, framework</b></summary>

```bash
elevenlabs user info
elevenlabs user subscription
elevenlabs history list [--page-size 20]
elevenlabs history delete <id>

elevenlabs config show | path | set <key> <value> | check | init --api-key sk_...
elevenlabs skill install | status
elevenlabs update [--check]
elevenlabs agent-info          # JSON capability manifest (alias: info)
```

</details>

---

## For AI agents

This CLI is built to be called by autonomous agents. It follows the [Agent CLI Framework](https://github.com/199-biotechnologies/agent-cli-framework) patterns:

- **`agent-info`** returns a complete capability manifest as JSON — no MCP server required, no schema file to fetch.
- **Dual output**: terminal users get colour + tables, piped/`--json` callers get a stable `{version, status, data|error}` envelope.
- **Semantic exit codes**: `0=ok, 1=transient, 2=config/auth, 3=bad input, 4=rate limited`. Agents use these to pick retry, fix-and-retry, or escalate.
- **Errors have suggestions**: every error envelope includes a `suggestion` field with a concrete next command, not vague advice.
- **No interactive prompts** — every flag has an environment or config fallback. Scripts never hang.
- **Installable skill**: `elevenlabs skill install` drops `SKILL.md` into `~/.claude/skills/`, `~/.codex/skills/`, and `~/.gemini/skills/` so agents discover the tool automatically.

Example agent usage:

```bash
# Bootstrap: what can this tool do?
elevenlabs agent-info | jq '.commands | keys'

# Call it, parse structured output
output=$(elevenlabs tts "status update" --json -o /tmp/out.mp3)
path=$(echo "$output" | jq -r '.data.output_path')

# Check exit code semantically
if [ $? -eq 4 ]; then sleep 30; else ...; fi
```

---

## Why a CLI instead of an MCP server?

The [official ElevenLabs MCP server](https://github.com/elevenlabs/elevenlabs-mcp) runs as a stdio subprocess per session, burns context on tool definitions, requires a Python runtime, and drifts from the API. This CLI does the opposite:

|                              | MCP server           | `elevenlabs-cli`              |
|------------------------------|----------------------|-------------------------------|
| Install                      | Python, `uvx`, MCP client | Single ~5 MB static binary    |
| Context cost per tool        | ~550-1400 tokens     | 0 (one shell exec)            |
| Context cost to bootstrap    | ~55k tokens (typical)| One `agent-info` call         |
| Scriptable                   | No                   | Yes (pipes, shell, make, CI)  |
| Works without MCP host       | No                   | Yes                           |
| Offline from first install   | No (needs runtime)   | Yes                           |
| Cold start                   | ~200 ms+             | **<10 ms**                    |
| Memory                       | ~50-80 MB            | **~7 MB**                     |

Benchmarks in the wild: [MCP vs CLI (Scalekit)](https://www.scalekit.com/blog/mcp-vs-cli-use) measured a 32× token overhead for MCP on 75 real tasks. [Speakeasy](https://www.speakeasy.com/blog/how-we-reduced-token-usage-by-100x-dynamic-toolsets-v2) reduced token usage by ~100× by moving agents off MCP onto CLIs. GitHub Copilot [dropped from 40 tools to 13](https://github.blog/ai-and-ml/github-copilot/how-were-making-github-copilot-smarter-with-fewer-tools/) and got better results.

LLMs already know how to drive CLIs — the grammar of `tool subcommand --flag value` is baked into their weights. Give them a tool, not a pamphlet about tools.

---

## Configuration

Config lives in a TOML file at:

| OS      | Path                                                           |
|---------|----------------------------------------------------------------|
| macOS   | `~/Library/Application Support/elevenlabs-cli/config.toml`      |
| Linux   | `~/.config/elevenlabs-cli/config.toml`                         |
| Windows | `%APPDATA%\elevenlabs-cli\config.toml`                         |

Example:

```toml
api_key = "sk_..."

[defaults]
voice_id = "oaGwHLz3csUaSnc2NBD4"
model_id = "eleven_multilingual_v2"
output_format = "mp3_44100_128"
output_dir = "~/Desktop"

[update]
enabled = true
owner = "199-biotechnologies"
repo = "elevenlabs-cli"
```

**Precedence** (highest wins): CLI flags → environment variables (`ELEVENLABS_API_KEY`, `ELEVENLABS_CLI_*`) → config file → defaults. Secrets are masked in `config show`. The config file is chmod `0600` on Unix.

---

## Output contract

**Success** (stdout, piped or `--json`):
```json
{
  "version": "1",
  "status": "success",
  "data": {
    "voice_id": "oaGwHLz3csUaSnc2NBD4",
    "output_path": "/tmp/hello.mp3",
    "bytes_written": 46437
  }
}
```

**Error** (stderr, piped or `--json`):
```json
{
  "version": "1",
  "status": "error",
  "error": {
    "code": "auth_missing",
    "message": "API key not configured",
    "suggestion": "Set your API key: elevenlabs config init --api-key <sk_...>  or: export ELEVENLABS_API_KEY=sk_..."
  }
}
```

---

## Building from source

```bash
git clone https://github.com/199-biotechnologies/elevenlabs-cli
cd elevenlabs-cli
cargo build --release
./target/release/elevenlabs --version
cargo test
```

Requires Rust 1.85+ (2024 edition).

---

## Contributing

PRs welcome. Keep commits focused, follow the existing patterns, and run `cargo fmt && cargo clippy && cargo test` before pushing.

---

## License

MIT © 2026 [199 Biotechnologies](https://github.com/199-biotechnologies). See [LICENSE](LICENSE).

---

<div align="center">

Built by [Boris Djordjevic](https://github.com/longevityboris) at [199 Biotechnologies](https://github.com/199-biotechnologies) using the [Agent CLI Framework](https://github.com/199-biotechnologies/agent-cli-framework).

**If this saves you context or setup time:**

[![Star this repo](https://img.shields.io/github/stars/199-biotechnologies/elevenlabs-cli?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/199-biotechnologies/elevenlabs-cli/stargazers)
&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

</div>
