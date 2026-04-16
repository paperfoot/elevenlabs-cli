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
                "description": "Convert text to speech. Writes an audio file. Supports streaming \
                 (--stream routes to /stream endpoint) and per-character alignment JSON \
                 (--with-timestamps routes to /with-timestamps endpoint).",
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
                    "--stdout",
                    "--stream",
                    "--with-timestamps",
                    "--save-timestamps <path>",
                    "--seed <0-4294967295>",
                    "--optimize-streaming-latency <0-4>",
                    "--no-logging",
                    "--previous-text <str>",
                    "--next-text <str>",
                    "--previous-request-id <id> (repeatable, max 3)",
                    "--next-request-id <id> (repeatable, max 3)",
                    "--apply-text-normalization <auto|on|off>",
                    "--apply-language-text-normalization",
                    "--use-pvc-as-ivc"
                ]
            },
            "stt [file]": {
                "description": "Transcribe audio (scribe_v2 by default). Supports local files, HTTPS URLs, \
                 hosted videos (YouTube/TikTok), character-level timestamps, diarization, entity \
                 redaction, biasing keyterms, and SRT/TXT/DOCX/PDF/HTML export.",
                "aliases": ["transcribe"],
                "options": [
                    "--output <path>",
                    "--save-raw <path>",
                    "--save-words <path>",
                    "--from-url <url>",
                    "--source-url <url>",
                    "--model <scribe_v2|scribe_v1>",
                    "--language <iso>",
                    "--timestamps <none|word|character>",
                    "--diarize",
                    "--num-speakers <1-32>",
                    "--diarization-threshold <0-1>",
                    "--detect-speaker-roles",
                    "--audio-events",
                    "--no-audio-events",
                    "--no-verbatim",
                    "--multi-channel",
                    "--pcm-16k",
                    "--temperature <0-2>",
                    "--seed <n>",
                    "--keyterm <word> (repeatable)",
                    "--detect-entities <type> (repeatable)",
                    "--redact-entities <type> (repeatable)",
                    "--redaction-mode <redacted|entity_type|enumerated_entity_type>",
                    "--format <srt|txt|segmented_json|docx|pdf|html> (repeatable)",
                    "--format-include-speakers",
                    "--format-include-timestamps",
                    "--format-segment-on-silence <s>",
                    "--format-max-segment-duration <s>",
                    "--format-max-segment-chars <n>",
                    "--format-max-chars-per-line <n>",
                    "--format-out-dir <dir>",
                    "--no-logging",
                    "--webhook",
                    "--webhook-id <id>",
                    "--webhook-metadata <json>"
                ]
            },
            "sfx <text>": {
                "description": "Generate a sound effect from a text description (0.5-30s).",
                "aliases": ["sound"],
                "options": [
                    "--duration <0.5-30>",
                    "--prompt-influence <0-1>",
                    "--loop",
                    "--format <codec_rate_bitrate>",
                    "--model <id>",
                    "--output <path>"
                ]
            },
            "voices list": "List voices in your library. Supports --search, --sort, --direction, --limit, --show-legacy.",
            "voices show <voice_id>": "Get full details for a voice",
            "voices search <query>": "Search your voice library",
            "voices library": "Search the public shared voice library (1-indexed pagination). Filters: --search, --page, --page-size, --category, --gender, --age, --accent, --language, --locale, --use-case, --featured, --min-notice-days, --include-custom-rates, --include-live-moderated, --reader-app-enabled, --owner-id, --sort.",
            "voices clone <name> <files...>": "Instant voice clone (IVC) from samples",
            "voices design <description>": "Generate voice previews from a text description. Supports --model {eleven_multilingual_ttv_v2|eleven_ttv_v3}, --seed, --loudness, --guidance-scale, --enhance, --stream-previews, --quality, --text, --output-dir.",
            "voices save-preview <generated_voice_id> <name> <description>": "Save a designed voice to your library",
            "voices delete <voice_id> --yes": "Delete a voice (aliases: rm). --yes is required because deletion is irreversible.",
            "models list": "List available models",
            "audio isolate <file>": "Isolate speech from background. Supports --pcm-16k for raw 16-bit PCM input.",
            "audio convert <file>": "Voice-to-voice conversion (speech-to-speech). Supports --format, --voice/--voice-id, --model, --stability/--similarity/--style/--speaker-boost/--speed (voice settings), --seed, --remove-background-noise, --optimize-streaming-latency, --pcm-16k, --no-logging.",
            "music compose [prompt]": "Compose music from a text prompt. Length 3000-600000ms. Supports --composition-plan <file>, --force-instrumental, --seed, --model, --respect-sections-durations, --store-for-inpainting, --sign-with-c2pa.",
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
            "history list": "List generation history. Filters: --start-after <id>, --voice-id, --model-id, --before <unix>, --after <unix>, --sort-direction {asc|desc}, --search, --source {TTS|STS}.",
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
