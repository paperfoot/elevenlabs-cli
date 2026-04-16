//! Clap CLI definitions — every subcommand lives here so `cli.rs` is the
//! single source of truth for the command surface.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "elevenlabs",
    version,
    about = "Agent-friendly CLI for the ElevenLabs AI audio platform",
    long_about = "Text-to-speech, speech-to-text, sound effects, voice cloning, \
                  music generation, and conversational AI agents from your terminal. \
                  Use --json for machine-readable output (auto-enabled when piped)."
)]
pub struct Cli {
    /// Force JSON output (auto-enabled when piped)
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress informational output
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // ── Domain: speech ──────────────────────────────────────────────────────
    /// Convert text to speech
    #[command(visible_alias = "speak")]
    Tts(TtsArgs),

    /// Transcribe an audio file to text
    #[command(visible_alias = "transcribe")]
    Stt(Box<SttArgs>),

    /// Generate a sound effect from a text description
    #[command(visible_alias = "sound")]
    Sfx(SfxArgs),

    // ── Domain: voices ──────────────────────────────────────────────────────
    /// Manage and search voices
    #[command(visible_alias = "voice")]
    Voices {
        #[command(subcommand)]
        action: VoicesAction,
    },

    // ── Domain: models ──────────────────────────────────────────────────────
    /// List or inspect TTS/STT models
    #[command(visible_alias = "model")]
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },

    // ── Domain: audio transforms ────────────────────────────────────────────
    /// Audio transforms (isolation, voice-to-voice conversion)
    Audio {
        #[command(subcommand)]
        action: AudioAction,
    },

    // ── Domain: music ───────────────────────────────────────────────────────
    /// Compose music from text prompts
    Music {
        #[command(subcommand)]
        action: MusicAction,
    },

    // ── Domain: user / subscription ─────────────────────────────────────────
    /// User and subscription info
    User {
        #[command(subcommand)]
        action: UserAction,
    },

    // ── Domain: conversational AI agents ────────────────────────────────────
    /// Manage ElevenLabs conversational AI agents
    Agents {
        #[command(subcommand)]
        action: AgentsAction,
    },

    /// Browse agent conversations / transcripts
    #[command(visible_alias = "convs")]
    Conversations {
        #[command(subcommand)]
        action: ConversationsAction,
    },

    /// Manage phone numbers and outbound calls
    Phone {
        #[command(subcommand)]
        action: PhoneAction,
    },

    // ── Domain: history ─────────────────────────────────────────────────────
    /// Browse or delete generation history
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },

    // ── Framework commands ──────────────────────────────────────────────────
    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo,

    /// Manage skill file installation across AI agent platforms
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Manage configuration (api key, defaults)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Self-update from GitHub Releases
    Update {
        /// Check only, do not install
        #[arg(long)]
        check: bool,
    },
}

// ── TTS ────────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct TtsArgs {
    /// Text to synthesise (use - to read from stdin)
    pub text: String,

    /// Output file path. Defaults to ./tts_<timestamp>.mp3
    #[arg(short, long)]
    pub output: Option<String>,

    /// Voice ID (overrides config default)
    #[arg(long)]
    pub voice_id: Option<String>,

    /// Voice name (resolves to voice_id via search)
    #[arg(long)]
    pub voice: Option<String>,

    /// Model ID (eleven_v3, eleven_multilingual_v2, eleven_turbo_v2_5, eleven_flash_v2_5, ...)
    #[arg(long)]
    pub model: Option<String>,

    /// Output format (e.g. mp3_44100_128, pcm_44100, ulaw_8000)
    #[arg(long)]
    pub format: Option<String>,

    /// Stability 0.0-1.0 (default 0.5)
    #[arg(long)]
    pub stability: Option<f32>,

    /// Similarity boost 0.0-1.0 (default 0.75)
    #[arg(long)]
    pub similarity: Option<f32>,

    /// Style exaggeration 0.0-1.0 (default 0.0)
    #[arg(long)]
    pub style: Option<f32>,

    /// Speaker boost (default on)
    #[arg(long)]
    pub speaker_boost: Option<bool>,

    /// Speed 0.7-1.2 (default 1.0)
    #[arg(long)]
    pub speed: Option<f32>,

    /// ISO language code (for v3 and multilingual models)
    #[arg(long)]
    pub language: Option<String>,

    /// Write raw audio bytes to stdout instead of a file (implies --quiet)
    #[arg(long)]
    pub stdout: bool,
}

// ── STT ────────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct SttArgs {
    /// Input audio/video file path. Omit when using --from-url or --source-url.
    pub file: Option<String>,

    /// HTTPS URL to audio/video (S3/GCS/R2/CDN/pre-signed). Mutually exclusive with <FILE>/--source-url.
    #[arg(long, value_name = "URL", conflicts_with_all = ["file", "source_url"])]
    pub from_url: Option<String>,

    /// Hosted video URL (YouTube, TikTok, etc). Mutually exclusive with <FILE>/--from-url.
    #[arg(long, value_name = "URL", conflicts_with_all = ["file", "from_url"])]
    pub source_url: Option<String>,

    /// Output text file for the transcript. When absent, the transcript is printed.
    #[arg(short, long)]
    pub output: Option<String>,

    /// Save the full JSON response (words, characters, entities, ...) to a file.
    #[arg(long, value_name = "PATH")]
    pub save_raw: Option<String>,

    /// Save just the word-timing array as JSON (for lyric/subtitle pipelines).
    #[arg(long, value_name = "PATH")]
    pub save_words: Option<String>,

    // ── Model / language ───────────────────────────────────────────────────
    /// Model ID. Default scribe_v2 (best accuracy); scribe_v1 for legacy.
    #[arg(long, default_value = "scribe_v2", value_parser = ["scribe_v2", "scribe_v1"])]
    pub model: String,

    /// ISO 639-1 or ISO 639-3 language code (auto-detect when omitted).
    #[arg(long)]
    pub language: Option<String>,

    // ── Timestamps ─────────────────────────────────────────────────────────
    /// Timestamp granularity. 'character' gives per-character start/end — ideal for
    /// karaoke/lyric sync. Values: none | word | character. Default word.
    #[arg(long, default_value = "word", value_parser = ["none", "word", "character"])]
    pub timestamps: String,

    // ── Diarization ────────────────────────────────────────────────────────
    /// Annotate which speaker is talking.
    #[arg(long)]
    pub diarize: bool,

    /// Max speakers expected (1-32). Helps diarization accuracy.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=32))]
    pub num_speakers: Option<u32>,

    /// Diarization threshold 0.0-1.0. Higher = fewer predicted speakers. Requires --diarize and
    /// conflicts with --num-speakers.
    #[arg(long, value_name = "FLOAT")]
    pub diarization_threshold: Option<f32>,

    /// Auto-label speakers as agent vs customer. Requires --diarize.
    #[arg(long)]
    pub detect_speaker_roles: bool,

    // ── Audio-event tagging / verbatim ─────────────────────────────────────
    /// Disable tagging of non-speech audio events (default: tagging is on).
    #[arg(long)]
    pub no_audio_events: bool,

    /// Explicitly enable audio-event tagging (on by default; flag is for clarity).
    #[arg(long, conflicts_with = "no_audio_events")]
    pub audio_events: bool,

    /// Remove filler words and non-speech sounds (scribe_v2 only).
    #[arg(long)]
    pub no_verbatim: bool,

    // ── Multi-channel / raw PCM ────────────────────────────────────────────
    /// Transcribe each channel independently (e.g. stereo call: agent L / customer R, max 5).
    #[arg(long)]
    pub multi_channel: bool,

    /// Declare input is raw 16-bit PCM mono @ 16 kHz little-endian (lower latency).
    #[arg(long)]
    pub pcm_16k: bool,

    // ── Determinism ────────────────────────────────────────────────────────
    /// Sampling temperature 0.0-2.0. Lower = more deterministic.
    #[arg(long, value_name = "FLOAT")]
    pub temperature: Option<f32>,

    /// Seed for deterministic sampling (0 to 2_147_483_647).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=2_147_483_647))]
    pub seed: Option<u32>,

    // ── Biasing / keyterms ─────────────────────────────────────────────────
    /// Keyterm to bias transcription toward. Repeatable; max 1000; <=50 chars each; ≤5 words
    /// after normalisation. Adds 20% cost surcharge.
    #[arg(long = "keyterm", value_name = "WORD")]
    pub keyterms: Vec<String>,

    // ── Entity detection / redaction (PII) ─────────────────────────────────
    /// Detect entities. Values: all | pii | phi | pci | other | offensive_language | <specific-type>.
    /// Repeatable. Adds 30% cost surcharge.
    #[arg(long = "detect-entities", value_name = "TYPE")]
    pub detect_entities: Vec<String>,

    /// Redact detected entities (must be a subset of --detect-entities). Repeatable. 30% surcharge.
    #[arg(long = "redact-entities", value_name = "TYPE")]
    pub redact_entities: Vec<String>,

    /// Redaction format for the text. Values: redacted | entity_type | enumerated_entity_type.
    #[arg(long, value_parser = ["redacted", "entity_type", "enumerated_entity_type"])]
    pub redaction_mode: Option<String>,

    // ── Additional export formats (SRT, segmented_json, ...) ───────────────
    /// Export format in addition to the JSON transcript. Repeatable.
    /// Values: srt | txt | segmented_json | docx | pdf | html.
    #[arg(long = "format", value_name = "FMT", value_parser = ["srt", "txt", "segmented_json", "docx", "pdf", "html"])]
    pub formats: Vec<String>,

    /// Include speaker labels in exported formats.
    #[arg(long)]
    pub format_include_speakers: bool,

    /// Include timestamps in exported formats.
    #[arg(long)]
    pub format_include_timestamps: bool,

    /// Segment exported text on silence longer than N seconds.
    #[arg(long, value_name = "SECONDS")]
    pub format_segment_on_silence: Option<f32>,

    /// Maximum segment duration in seconds for exported formats.
    #[arg(long, value_name = "SECONDS")]
    pub format_max_segment_duration: Option<f32>,

    /// Maximum characters per segment for exported formats.
    #[arg(long)]
    pub format_max_segment_chars: Option<u32>,

    /// Maximum characters per line (SRT/TXT only).
    #[arg(long)]
    pub format_max_chars_per_line: Option<u32>,

    /// Directory to save exported format files (defaults to CWD).
    #[arg(long, value_name = "DIR")]
    pub format_out_dir: Option<String>,

    // ── Privacy / ZRM ──────────────────────────────────────────────────────
    /// Zero-retention mode (enterprise only). Disables log/transcript storage.
    #[arg(long)]
    pub no_logging: bool,

    // ── Webhooks (async) ───────────────────────────────────────────────────
    /// Send result to configured webhooks asynchronously; command returns early.
    #[arg(long)]
    pub webhook: bool,

    /// Specific webhook ID to receive the result (requires --webhook).
    #[arg(long)]
    pub webhook_id: Option<String>,

    /// JSON metadata passed through to the webhook (max 16 KB, depth 2).
    #[arg(long, value_name = "JSON")]
    pub webhook_metadata: Option<String>,
}

// ── SFX ────────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct SfxArgs {
    /// Text description of the sound effect
    pub text: String,

    /// Output file path. Defaults to ./sfx_<timestamp>.mp3
    #[arg(short, long)]
    pub output: Option<String>,

    /// Duration in seconds (0.5 to 22)
    #[arg(long)]
    pub duration: Option<f32>,

    /// Prompt influence 0.0-1.0 (default 0.3)
    #[arg(long)]
    pub prompt_influence: Option<f32>,

    /// Loop the generated sound
    #[arg(long = "loop")]
    pub looping: bool,

    /// Output format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,
}

// ── Voices ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum VoicesAction {
    /// List voices in your library
    #[command(visible_alias = "ls")]
    List {
        /// Filter by search term
        #[arg(long)]
        search: Option<String>,

        /// Sort by field (name|created_at_unix)
        #[arg(long, default_value = "name")]
        sort: String,

        /// Sort direction (asc|desc)
        #[arg(long, default_value = "asc")]
        direction: String,

        /// Max results
        #[arg(long, default_value = "100")]
        limit: u32,
    },

    /// Get full details for a voice
    #[command(visible_alias = "get")]
    Show {
        /// Voice ID
        voice_id: String,
    },

    /// Search your voice library
    Search {
        /// Search term
        query: String,
    },

    /// Search the public (shared) voice library
    Library {
        /// Search term
        #[arg(long)]
        search: Option<String>,

        /// Page number (0-indexed)
        #[arg(long, default_value = "0")]
        page: u32,

        /// Page size (1-100)
        #[arg(long, default_value = "20")]
        page_size: u32,

        /// Gender filter (male|female|neutral)
        #[arg(long)]
        gender: Option<String>,

        /// Age filter (young|middle_aged|old)
        #[arg(long)]
        age: Option<String>,

        /// Accent filter
        #[arg(long)]
        accent: Option<String>,

        /// Language filter (ISO code)
        #[arg(long)]
        language: Option<String>,

        /// Use case filter (narration|audiobook|...)
        #[arg(long)]
        use_case: Option<String>,
    },

    /// Instant-clone a voice from sample audio files (IVC)
    Clone {
        /// Name for the cloned voice
        name: String,

        /// Audio sample files (mp3/wav/m4a)
        #[arg(required = true)]
        files: Vec<String>,

        /// Description for the voice
        #[arg(long)]
        description: Option<String>,
    },

    /// Generate voice previews from a text description (voice design)
    Design {
        /// Text description of the voice
        description: String,

        /// Optional text to read in the preview (auto-generated if omitted)
        #[arg(long)]
        text: Option<String>,

        /// Directory to save preview files
        #[arg(long)]
        output_dir: Option<String>,
    },

    /// Save a previously-designed voice preview to your library
    SavePreview {
        /// Generated voice ID (from `voices design`)
        generated_voice_id: String,

        /// Voice name
        name: String,

        /// Voice description
        description: String,
    },

    /// Delete a voice
    #[command(visible_alias = "rm")]
    Delete {
        /// Voice ID to delete
        voice_id: String,
    },
}

// ── Models ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum ModelsAction {
    /// List available models
    #[command(visible_alias = "ls")]
    List,
}

// ── Audio transforms ────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum AudioAction {
    /// Isolate speech from background noise / music
    Isolate {
        /// Input audio file
        file: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Voice-to-voice conversion (speech-to-speech)
    Convert {
        /// Input audio file
        file: String,

        /// Target voice ID
        #[arg(long)]
        voice_id: Option<String>,

        /// Target voice name (resolves to voice_id)
        #[arg(long)]
        voice: Option<String>,

        /// Model ID (default eleven_multilingual_sts_v2)
        #[arg(long)]
        model: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

// ── Music ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum MusicAction {
    /// Compose music from a text prompt
    Compose {
        /// Text prompt
        prompt: String,

        /// Target length in milliseconds (10000-300000)
        #[arg(long)]
        length_ms: Option<u32>,

        /// Output audio format (default mp3_44100_128)
        #[arg(long)]
        format: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Create a composition plan (free, subject to rate limits)
    Plan {
        /// Text prompt
        prompt: String,

        /// Target length in milliseconds
        #[arg(long)]
        length_ms: Option<u32>,
    },
}

// ── User ───────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum UserAction {
    /// Get basic user info
    Info,
    /// Get subscription and usage info
    Subscription,
}

// ── Agents (Conversational AI) ─────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsAction {
    /// List agents
    #[command(visible_alias = "ls")]
    List,

    /// Get agent details
    #[command(visible_alias = "get")]
    Show {
        /// Agent ID
        agent_id: String,
    },

    /// Create a new agent
    #[command(visible_alias = "new")]
    Create {
        /// Agent name
        name: String,

        /// System prompt
        #[arg(long)]
        system_prompt: String,

        /// First message the agent says
        #[arg(long)]
        first_message: Option<String>,

        /// Voice ID for the agent
        #[arg(long)]
        voice_id: Option<String>,

        /// Language ISO 639-1 code (default en)
        #[arg(long, default_value = "en")]
        language: String,

        /// LLM to use (default gemini-2.0-flash-001)
        #[arg(long, default_value = "gemini-2.0-flash-001")]
        llm: String,

        /// Temperature 0.0-1.0
        #[arg(long, default_value = "0.5")]
        temperature: f32,

        /// TTS model ID (default eleven_turbo_v2)
        #[arg(long, default_value = "eleven_turbo_v2")]
        model_id: String,
    },

    /// Delete an agent
    #[command(visible_alias = "rm")]
    Delete {
        /// Agent ID
        agent_id: String,
    },

    /// Add a knowledge base document to an agent
    AddKnowledge {
        /// Agent ID
        agent_id: String,

        /// Document name
        name: String,

        /// Source URL (one of: --url, --file, --text)
        #[arg(long)]
        url: Option<String>,

        /// Source file path
        #[arg(long)]
        file: Option<String>,

        /// Source text
        #[arg(long)]
        text: Option<String>,
    },
}

// ── Conversations ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum ConversationsAction {
    /// List conversations
    #[command(visible_alias = "ls")]
    List {
        /// Filter by agent ID
        #[arg(long)]
        agent_id: Option<String>,

        /// Page size (1-100)
        #[arg(long, default_value = "30")]
        page_size: u32,

        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Get a conversation (with transcript)
    #[command(visible_alias = "get")]
    Show {
        /// Conversation ID
        conversation_id: String,
    },
}

// ── Phone ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneAction {
    /// List phone numbers
    #[command(visible_alias = "ls")]
    List,

    /// Make an outbound call with an agent
    Call {
        /// Agent ID to handle the call
        agent_id: String,

        /// Phone number ID to call from
        #[arg(long)]
        from_id: String,

        /// E.164 number to call (+1...)
        #[arg(long)]
        to: String,
    },
}

// ── History ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum HistoryAction {
    /// List recent generation history
    #[command(visible_alias = "ls")]
    List {
        /// Page size (1-1000)
        #[arg(long, default_value = "20")]
        page_size: u32,
    },
    /// Delete a history item
    #[command(visible_alias = "rm")]
    Delete {
        /// History item ID
        history_item_id: String,
    },
}

// ── Config ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Display effective merged configuration (masks secrets)
    Show,
    /// Print configuration file path
    Path,
    /// Set a configuration key
    Set {
        /// Dotted key (e.g. `api_key`, `defaults.voice_id`)
        key: String,
        /// Value
        value: String,
    },
    /// Verify the configured API key works
    Check,
    /// Interactive first-time init (writes config.toml)
    Init {
        /// API key (omit to be prompted — non-interactive envs should pass it)
        #[arg(long)]
        api_key: Option<String>,
    },
}

// ── Skill ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum SkillAction {
    /// Install skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}
