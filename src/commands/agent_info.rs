//! Machine-readable capability manifest. Agents call this once to bootstrap.

pub fn run() {
    let info = serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "binary": "elevenlabs",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "homepage": env!("CARGO_PKG_HOMEPAGE"),
        "commands": {
            "tts <text>": {
                "description": "Convert text to speech. Writes an audio file.",
                "aliases": ["speak"],
                "options": [
                    "--voice-id <id>",
                    "--voice <name>",
                    "--model <model_id>",
                    "--output <path>",
                    "--format <codec_rate_bitrate>",
                    "--stability <0-1>",
                    "--similarity <0-1>",
                    "--style <0-1>",
                    "--speed <0.7-1.2>",
                    "--language <iso>",
                    "--stdout"
                ]
            },
            "stt <file>": {
                "description": "Transcribe an audio file (scribe_v1 by default).",
                "aliases": ["transcribe"],
                "options": ["--output <path>", "--language <iso>", "--diarize", "--timestamps", "--audio-events"]
            },
            "sfx <text>": {
                "description": "Generate a sound effect from a text description.",
                "aliases": ["sound"],
                "options": ["--duration <sec>", "--prompt-influence <0-1>", "--loop", "--output <path>"]
            },
            "voices list": "List voices in your library",
            "voices show <voice_id>": "Get full details for a voice",
            "voices search <query>": "Search your voice library",
            "voices library": "Search the public shared voice library",
            "voices clone <name> <files...>": "Instant voice clone (IVC) from samples",
            "voices design <description>": "Generate voice previews from a text description",
            "voices save-preview <generated_voice_id> <name> <description>": "Save a designed voice to your library",
            "voices delete <voice_id>": "Delete a voice (aliases: rm)",
            "models list": "List available models",
            "audio isolate <file>": "Isolate speech from background",
            "audio convert <file>": "Voice-to-voice conversion (speech-to-speech)",
            "music compose <prompt>": "Compose music from a text prompt",
            "music plan <prompt>": "Create a composition plan (free)",
            "user info": "Basic user info",
            "user subscription": "Subscription tier, usage, and remaining characters",
            "agents list": "List conversational AI agents",
            "agents show <agent_id>": "Get agent details",
            "agents create <name>": "Create a conversational AI agent",
            "agents delete <agent_id>": "Delete an agent",
            "agents add-knowledge <agent_id> <name>": "Add a knowledge base document to an agent",
            "conversations list": "List agent conversations",
            "conversations show <conversation_id>": "Get a conversation with transcript",
            "phone list": "List phone numbers",
            "phone call <agent_id>": "Place an outbound call via an agent",
            "history list": "List generation history",
            "history delete <id>": "Delete a history item",
            "agent-info": {
                "description": "This manifest",
                "aliases": ["info"]
            },
            "skill install": "Install skill file to Claude/Codex/Gemini directories",
            "skill status": "Check skill installation status",
            "config show": "Display effective merged config (secrets masked)",
            "config path": "Show config file path",
            "config set <key> <value>": "Set a config key",
            "config check": "Verify the configured API key works",
            "config init": "Interactive first-time init",
            "update": "Self-update from GitHub Releases",
            "update --check": "Check for updates without installing"
        },
        "global_flags": {
            "--json": "Force JSON output (auto-enabled when piped)",
            "--quiet": "Suppress informational output"
        },
        "exit_codes": {
            "0": "Success",
            "1": "Transient error (IO, network, API 5xx) — retry",
            "2": "Config/auth error — fix setup",
            "3": "Bad input — fix arguments",
            "4": "Rate limited — wait and retry"
        },
        "envelope": {
            "version": "1",
            "success": "{ version, status: 'success', data }",
            "error": "{ version, status: 'error', error: { code, message, suggestion } }"
        },
        "config": {
            "path": crate::config::config_path().display().to_string(),
            "env_vars": {
                "ELEVENLABS_API_KEY": "Your ElevenLabs API key (required) — wins over config file",
                "ELEVENLABS_API_BASE_URL": "Override API base URL (default https://api.elevenlabs.io)",
                "ELEVENLABS_CLI_CONFIG": "Full path override for config.toml (tests + power users)"
            }
        },
        "auto_json_when_piped": true,
        "requires_api_key": true,
        "auth_env_var": "ELEVENLABS_API_KEY",
        "api_docs": "https://elevenlabs.io/docs/api-reference"
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&info).unwrap_or_else(|_| "{}".to_string())
    );
}
