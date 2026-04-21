//! Machine-readable capability manifest. Agents call this once to bootstrap.

use crate::commands::agents::agent_config::{AGENT_TTS_MODEL_IDS, GOTCHAS};

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
            "voices list": "List voices in your library (v2). Supports --search, --sort, --direction, --limit, --show-legacy, --next-page-token, --voice-type, --category, --fine-tuning-state, --collection-id, --include-total-count, --voice-id (repeatable).",
            "voices show <voice_id>": "Get full details for a voice",
            "voices search <query>": "Search your voice library",
            "voices library": "Search the public shared voice library (1-indexed pagination). Filters: --search, --page, --page-size, --category, --gender, --age, --accent, --language, --locale, --use-case, --featured, --min-notice-days, --include-custom-rates, --include-live-moderated, --reader-app-enabled, --owner-id, --sort.",
            "voices clone <name> <files...>": "Instant voice clone (IVC) from samples",
            "voices design <description>": "Generate voice previews from a text description. Supports --model {eleven_multilingual_ttv_v2|eleven_ttv_v3}, --seed, --loudness, --guidance-scale, --enhance, --stream-previews, --quality, --text, --output-dir.",
            "voices save-preview <generated_voice_id> <name> <description>": "Save a designed voice to your library",
            "voices delete <voice_id> --yes": "Delete a voice (aliases: rm). --yes is required because deletion is irreversible.",
            "voices add-shared <public_user_id> <voice_id> --name <new_name>": "Add a shared voice from the public library into your collection. Optional --bookmarked.",
            "voices similar <audio_file>": "Find shared voices similar to an audio sample. Filters: --similarity-threshold, --top-k, --gender, --age, --accent, --language, --use-case.",
            "voices edit <voice_id>": "Edit a voice — rename, re-describe, update labels, add/remove samples. Flags: --name, --description, --labels <k=v> (repeatable), --add-sample <file> (repeatable), --remove-sample <sample_id> (repeatable), --remove-background-noise.",
            "dialogue [triples... | path.json | -]": "Generate multi-speaker dialogue (eleven_v3). Accepts JSON file, colon-delimited `label:voice_id:text` positional triples, or `-` for stdin JSON. --stream/--with-timestamps route to the four variant endpoints. (aliases: dlg)",
            "align <audio> <transcript|path>": "Forced alignment: align a known transcript to an audio recording. Returns per-word and per-character start/end timings plus a loss score. Prefer --transcript-file for multi-line text.",
            "models list": "List available models",
            "audio isolate <file>": "Isolate speech from background. Supports --pcm-16k for raw 16-bit PCM input.",
            "audio convert <file>": "Voice-to-voice conversion (speech-to-speech). Supports --format, --voice/--voice-id, --model, --stability/--similarity/--style/--speaker-boost/--speed (voice settings), --seed, --remove-background-noise, --optimize-streaming-latency, --pcm-16k, --no-logging.",
            "music compose [prompt]": "Compose music from a text prompt. Length 3000-600000ms. Supports --composition-plan <file>, --force-instrumental, --seed, --model, --respect-sections-durations, --store-for-inpainting, --sign-with-c2pa.",
            "music plan <prompt>": "Create a composition plan (free, returns JSON with sections)",
            "music detailed [prompt]": "Generate music plus rich metadata (bpm, time_signature, sections). Same flags as compose plus --save-metadata. Audio and metadata land in separate files.",
            "music stream [prompt]": "Stream compose: writes audio to disk chunk-by-chunk as the response arrives.",
            "music upload <file>": "Upload an audio file so it can be referenced by song_id for inpainting. Flags: --name, --composition-plan.",
            "music stem-separation <song_id_or_file>": "Split a track into stems (aliases: stems). Flags: --output-dir, --stems <vocals|drums|bass|other> (repeatable).",
            "music video-to-music <video_file>": "Generate a score from video content (aliases: v2m). Flags: --description, --tag (repeatable), --model, --format, --output.",
            "dubbing create": "Create a dubbing job from --file or --source-url. Required: --target-lang. Optional: --source-lang, --num-speakers, --watermark, --start-time, --end-time, --highest-resolution, --drop-background-audio, --use-profanity-filter, --dubbing-studio, --disable-voice-cloning, --mode {automatic|manual}. (aliases: new)",
            "dubbing list": "List dubbing jobs. Filters: --dubbing-status, --filter-by-creator, --page-size.",
            "dubbing show <dubbing_id>": "Get a dubbing job's status (aliases: get).",
            "dubbing delete <dubbing_id> --yes": "Delete a dubbing job (aliases: rm).",
            "dubbing get-audio <dubbing_id> <language_code>": "Download the dubbed audio/video bytes. Flag: --output <path>.",
            "dubbing get-transcript <dubbing_id> <language_code> --format <srt|webvtt|json>": "Download the transcript in the chosen format (new endpoint).",
            "dubbing resource transcribe <dubbing_id>": "Re-run transcription on a Studio-mode dub. --patch <file.json> for overrides.",
            "dubbing resource translate <dubbing_id>": "Re-run translation on a Studio-mode dub. --patch <file.json>.",
            "dubbing resource dub <dubbing_id>": "Re-run the dub step. --patch <file.json>.",
            "dubbing resource render <dubbing_id> <language_code>": "Re-render the dubbed output for a target language. --patch <file.json>.",
            "dubbing resource migrate-segments <dubbing_id>": "Migrate legacy segment metadata. --patch <file.json>.",
            "dict list": "List pronunciation dictionaries. Filters: --cursor, --page-size, --search.",
            "dict add-file <name> <file>": "Upload a PLS XML / lexicon as a new dictionary. --description, --workspace-access.",
            "dict add-rules <name>": "Create a dictionary in-line via --rule WORD:PHONEME and/or --alias-rule WORD:ALIAS (repeatable). First ':' splits word from phoneme so IPA colons survive.",
            "dict show <id>": "Show full metadata for a pronunciation dictionary (aliases: get).",
            "dict update <id>": "PATCH dictionary metadata or archive it (reversible). Flags: --name, --description, --archive.",
            "dict download <id>": "Download the rendered PLS XML. --version <id> to pin, --output <path>.",
            "dict set-rules <id>": "Replace every rule (creates a new version). Plus --case-sensitive, --word-boundaries.",
            "dict add-rules-to <id>": "Append rules to an existing dictionary.",
            "dict remove-rules <id>": "Remove rules by their `string_to_replace` value. --word <WORD> (repeatable).",
            "user info": "Basic user info",
            "user subscription": "Subscription tier, usage, and remaining characters",
            "agents list": "List conversational AI agents",
            "agents show <agent_id>": "Get agent details",
            "agents llms": "List LLMs the Agents backend currently accepts for conversation_config.agent.prompt.llm. Hit this before `agents create --llm …` to avoid the 'accepted by clap, silently fails at conversation time' footgun.",
            "agents signed-url <agent_id>": "Issue a short-lived signed URL that can be embedded directly in a widget / web session without manual auth-token plumbing. (GET /v1/convai/conversation/get-signed-url)",
            "agents knowledge list": "List KB documents. Filters: --search, --page-size, --cursor. (aliases: ls)",
            "agents knowledge search <query>": "Chunk-level content search across the workspace KB. Useful for debugging RAG before your agent goes live. Flags: --document-id, --limit.",
            "agents knowledge refresh <document_id>": "Re-fetch a URL-backed KB document (e.g. after the source page changes). No-op for file/text-backed docs.",
            "agents create <name>": {
                "description": "Create a conversational AI agent. See `known_values.agent_tts_model_ids` and `gotchas.agents` before passing --model-id / --llm / --expressive-mode.",
                "aliases": ["new"],
                "defaults": {
                    "--llm": "gemini-3.1-flash-lite-preview",
                    "--model-id": "eleven_flash_v2_5",
                    "--temperature": 0.5,
                    "--language": "en",
                    "--max-duration-seconds": 300
                },
                "options": [
                    "--system-prompt <str> (required)",
                    "--first-message <str>",
                    "--voice-id <id>",
                    "--language <iso>",
                    "--llm <id>",
                    "--temperature <0.0-1.0>",
                    "--model-id <see known_values.agent_tts_model_ids>",
                    "--expressive-mode (implies model-id eleven_v3_conversational; requires Creator+ tier)",
                    "--max-duration-seconds <n>",
                    "--voicemail-detection (registers the voicemail_detection system tool; hangs up on answerphones)",
                    "--voicemail-message <text> (implies --voicemail-detection; leaves this message instead of hanging up)"
                ]
            },
            "agents update <agent_id> --patch <json_file>": {
                "description": "PATCH partial config into an existing agent. Body is forwarded verbatim to PATCH /v1/convai/agents/{id}. The CLI pre-validates conversation_config.tts.model_id and tts.expressive_mode against the known allowlist + the expressive-mode-silently-dropped footgun. See `known_values.agent_tts_model_ids` + `gotchas.agents`.",
                "common_patch_paths": {
                    "conversation_config.agent.prompt.prompt": "system prompt text",
                    "conversation_config.agent.prompt.llm": "LLM id (backend has its own allowlist)",
                    "conversation_config.agent.prompt.temperature": "0.0-1.0",
                    "conversation_config.agent.first_message": "what the agent says first",
                    "conversation_config.tts.voice_id": "bound voice id",
                    "conversation_config.tts.model_id": "one of known_values.agent_tts_model_ids",
                    "conversation_config.tts.expressive_mode": "true ONLY with model_id=eleven_v3_conversational",
                    "conversation_config.tts.stability": "0.0-1.0",
                    "conversation_config.tts.similarity_boost": "0.0-1.0",
                    "conversation_config.conversation.max_duration_seconds": "hard call limit in seconds (default 300)",
                    "conversation_config.turn.turn_model": "turn_v2 | turn_v3 (EMPIRICAL — enforced by the live API but absent from the current OpenAPI spec; treat as unofficial)",
                    "conversation_config.turn.turn_timeout": "silence seconds before the agent prompts (default 7, range 1-30)",
                    "conversation_config.turn.turn_eagerness": "patient | normal | eager (default normal)",
                    "conversation_config.turn.mode": "silence | turn (default turn)",
                    "conversation_config.turn.initial_wait_time": "seconds the agent waits before speaking at call-start",
                    "conversation_config.turn.silence_end_call_timeout": "seconds of silence after which the agent hangs up (-1 to disable)",
                    "conversation_config.turn.soft_timeout_config": "{timeout_seconds, message, use_llm_generated_message} — prompts the user after a short silence before giving up",
                    "conversation_config.agent.disable_first_message_interruptions": "true blocks user 'yes/uh-huh' from cutting off the greeting (lives on AGENT, not turn — pre-v0.3.0 docs had it wrong)",
                    "conversation_config.agent.max_conversation_duration_message": "message the agent says when the hard duration limit hits",
                    "conversation_config.agent.prompt.reasoning_effort": "none | minimal | low | medium | high | xhigh (only honoured by reasoning-capable LLMs)",
                    "conversation_config.agent.prompt.thinking_budget": "max tokens for <think> output on reasoning models",
                    "conversation_config.agent.prompt.max_tokens": "per-turn LLM output cap",
                    "conversation_config.agent.prompt.ignore_default_personality": "true strips ElevenLabs' default system preamble",
                    "conversation_config.agent.prompt.rag": "{enabled, embedding_model, max_documents_length, ...} — RAG over the attached knowledge_base",
                    "conversation_config.agent.prompt.tools[]": "append {type:'system', name:'voicemail_detection', description:''} to enable voicemail hang-up; add voicemail_message:'...' on that entry to leave a message instead",
                    "conversation_config.vad.background_voice_detection": "true filters speakerphone echo / room noise (lives on VAD, not turn — pre-v0.3.0 docs had it wrong)",
                    "platform_settings.evaluation.criteria[]": "post-call evaluation rubrics for automated QA",
                    "name": "TOP-LEVEL field, not inside conversation_config — agent display name"
                }
            },
            "agents duplicate <agent_id>": "Clone an agent's config. Supports --name <new_name>.",
            "agents delete <agent_id>": "Delete an agent",
            "agents add-knowledge <agent_id> <name>": "Create a knowledge base document AND attach it to the agent (POSTs /v1/convai/knowledge-base/{url|file|text} then PATCHes conversation_config.agent.prompt.knowledge_base). One of --url, --file, --text is required.",
            "agents tools list": "List workspace tools (aliases: ls)",
            "agents tools show <tool_id>": "Show full tool config (aliases: get)",
            "agents tools create --config <json_file>": "Create a tool from a JSON config file (aliases: new)",
            "agents tools update <tool_id> --patch <json_file>": "PATCH partial tool config",
            "agents tools delete <tool_id> --yes": "Delete a tool (irreversible; aliases: rm)",
            "agents tools deps <tool_id>": "List agents that depend on this tool (aliases: dependents)",
            "conversations list": "List agent conversations",
            "conversations show <conversation_id>": "Get a conversation with transcript",
            "conversations audio <conversation_id>": "Download the call audio recording. Default output: conv_<id>.mp3; override with -o. (GET /v1/convai/conversations/{id}/audio)",
            "phone list": "List phone numbers",
            "phone call <agent_id>": {
                "description": "Place an outbound call via an agent. Dispatches to Twilio or SIP-trunk based on the provider field of --from-id.",
                "options": [
                    "--from-id <phone_number_id> (required)",
                    "--to <E.164 number> (required, e.g. +14155551212)",
                    "--dynamic-variables <JSON object or @file.json> — per-call {{placeholders}}; merges into --client-data when both are passed",
                    "--client-data <JSON object or @file.json> — full conversation_initiation_client_data (conversation_config_override, user_id, source_info, branch_id, environment, starting_workflow_node_id, custom_llm_extra_body, dynamic_variables)",
                    "--record — enable call_recording_enabled on Twilio / SIP trunk",
                    "--ringing-timeout-secs <n> — cap the ring time before the call is abandoned"
                ]
            },
            "phone batch submit": "Submit a batch of outbound calls. Required: --agent, --phone-number, --recipients <path|->. CSV or JSON. Optional: --name, --scheduled-time-unix.",
            "phone batch list": "List batch calls in the workspace. Filters: --page-size, --cursor, --status, --agent-id.",
            "phone batch show <batch_id>": "Show a batch with per-call status (aliases: get).",
            "phone batch cancel <batch_id>": "Cancel a batch. Reversible via `phone batch retry`.",
            "phone batch retry <batch_id>": "Retry a batch (re-dials failed/pending recipients).",
            "phone batch delete <batch_id> --yes": "Delete a batch (aliases: rm).",
            "phone whatsapp call": "Place an outbound WhatsApp voice call. Required: --agent, --whatsapp-account, --recipient.",
            "phone whatsapp message": "Send a WhatsApp message. Supply exactly one of --text or --template.",
            "phone whatsapp accounts list": "List WhatsApp accounts (aliases: ls).",
            "phone whatsapp accounts show <id>": "Show WhatsApp account (aliases: get).",
            "phone whatsapp accounts update <id> --patch <json_file>": "PATCH a WhatsApp account with partial JSON.",
            "phone whatsapp accounts delete <id> --yes": "Delete a WhatsApp account (aliases: rm).",
            "doctor": "Run structured dependency + environment diagnostics (config, auth, env-var shadowing, API key scope, network reachability, ffmpeg, disk writeability, default_output_dir). JSON mode returns {checks, summary}. Exits 0 on pass/warn-only, 2 on any fail. Flags: --skip <name> (repeatable), --timeout-ms <ms>.",
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
                "ELEVENLABS_API_KEY": "Fallback API key used when config.toml has no api_key. Since v0.1.6 the saved config file wins; the env var is a fallback only.",
                "ELEVENLABS_API_BASE_URL": "Override API base URL (default https://api.elevenlabs.io)",
                "ELEVENLABS_CLI_CONFIG": "Full path override for config.toml (tests + power users)"
            }
        },
        "known_values": {
            "agent_tts_model_ids": AGENT_TTS_MODEL_IDS,
            "agent_turn_models": ["turn_v2", "turn_v3"],
            "agent_turn_eagerness": ["patient", "normal", "eager"]
        },
        "gotchas": {
            "agents": GOTCHAS,
            "turn_taking": [
                "turn_model lives at conversation_config.turn.turn_model. Valid values: 'turn_v2', 'turn_v3'. EMPIRICAL — the live API enforces these two values but the current OpenAPI spec doesn't reference the field; treat as unofficial. This CLI's `agents create` scaffolds turn_v2 because in real-world dialing (2026-04) turn_v3 was observed swallowing short turn-ends on some LLM configs — the agent heard the user but never took its turn.",
                "turn_eagerness: 'patient' | 'normal' | 'eager' (API default 'normal'). 'patient' empirically over-suppresses on speakerphone — the agent stops taking turns. 'normal' is the safe default.",
                "turn_timeout: seconds, range 1-30. Server default behaves like ~7s. Don't set above 15s without a test dial; users hang up thinking the line died.",
                "disable_first_message_interruptions lives on conversation_config.AGENT.disable_first_message_interruptions (spec schema AgentConfigAPIModel), NOT conversation_config.turn.*. Pre-v0.3.0 of this CLI wrote it under turn; the server silently dropped it. If you upgraded from 0.2.1/0.2.2 and rely on the greeting-not-interrupted behaviour, `agents update --patch` with the correct agent.* path or recreate the agent with v0.3.0+.",
                "background_voice_detection lives on conversation_config.VAD.background_voice_detection (spec schema VADConfig), NOT conversation_config.turn.*. Real-world testing showed true also mutes the user's own voice on speakerphones — leave it false unless a test dial proves otherwise.",
                "When a call connects but the agent never takes its next turn, always inspect `conversations show <conv_id>`: llm_usage.model_usage with 0 output_tokens on the expected LLM = the --llm was rejected by the backend and a fallback was tried that never completed. Swap the LLM first, not the turn settings."
            ],
            "outbound_calls": [
                "Voicemail detection is OFF by default on new agents. For outbound phone agents you almost always want it ON — otherwise the agent's opening greeting gets recorded onto the callee's answerphone. Pass --voicemail-detection on `agents create`, or PATCH an existing agent by appending {type:'system', name:'voicemail_detection', description:''} to conversation_config.agent.prompt.tools. Add voicemail_message:'...' to leave a message instead of hanging up.",
                "Dynamic variables are per-CALL, not per-agent. Set them on `phone call --dynamic-variables '{...}'`, not in conversation_config — that block is for schema placeholders only. Agent prompts reference them as {{name}}, {{order_id}}, etc."
            ]
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
