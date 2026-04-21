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

    /// Generate multi-speaker dialogue with `eleven_v3`
    #[command(visible_alias = "dlg")]
    Dialogue(DialogueArgs),

    /// Forced alignment: align a known transcript to an audio recording
    Align(AlignArgs),

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

    // ── Domain: dubbing ─────────────────────────────────────────────────────
    /// Dub media into new languages (and edit dubs Studio-style)
    Dubbing {
        #[command(subcommand)]
        action: DubbingAction,
    },

    // ── Domain: pronunciation dictionaries ──────────────────────────────────
    /// Manage pronunciation dictionaries (IPA / alias lexicons)
    Dict {
        #[command(subcommand)]
        action: DictAction,
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
    #[command(after_long_help = crate::help::UPDATE_HELP)]
    Update {
        /// Check only, do not install
        #[arg(long)]
        check: bool,
    },

    /// Run environment + dependency diagnostics
    #[command(after_long_help = crate::help::DOCTOR_HELP)]
    Doctor(DoctorArgs),
}

// ── TTS ────────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
#[command(after_long_help = crate::help::TTS_HELP)]
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

    /// Route to the streaming endpoint (/v1/text-to-speech/{voice}/stream).
    /// Audio is written as it arrives — useful for low-latency playback.
    #[arg(long)]
    pub stream: bool,

    /// Return per-character timings alongside the audio
    /// (/v1/text-to-speech/{voice}/with-timestamps). Saves audio to `--output`
    /// and the alignment JSON to `--save-timestamps`.
    #[arg(long)]
    pub with_timestamps: bool,

    /// Path to save the alignment JSON when `--with-timestamps` is set.
    #[arg(long, value_name = "PATH")]
    pub save_timestamps: Option<String>,

    /// Sampling seed for reproducibility (0 to 4_294_967_295).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4_294_967_295))]
    pub seed: Option<u32>,

    /// Latency optimization level (0=none, 4=max; may affect quality).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4))]
    pub optimize_streaming_latency: Option<u32>,

    /// Zero-retention mode (enterprise only).
    #[arg(long)]
    pub no_logging: bool,

    /// Text that immediately preceded this request (for continuity across splits).
    #[arg(long)]
    pub previous_text: Option<String>,

    /// Text that immediately follows this request (for continuity across splits).
    #[arg(long)]
    pub next_text: Option<String>,

    /// Previous request_id(s) to stitch onto (repeatable, max 3).
    /// Useful to maintain speech continuity when regenerating clips.
    #[arg(long = "previous-request-id", value_name = "ID")]
    pub previous_request_ids: Vec<String>,

    /// Next request_id(s) to anchor onto (repeatable, max 3).
    #[arg(long = "next-request-id", value_name = "ID")]
    pub next_request_ids: Vec<String>,

    /// Text normalization mode: auto | on | off.
    #[arg(long, value_parser = ["auto", "on", "off"])]
    pub apply_text_normalization: Option<String>,

    /// Apply language-specific text normalization (currently Japanese only).
    #[arg(long)]
    pub apply_language_text_normalization: bool,

    /// Temporary PVC-latency workaround: generate using the IVC version of
    /// the voice instead of the PVC version.
    #[arg(long)]
    pub use_pvc_as_ivc: bool,
}

// ── STT ────────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
#[command(after_long_help = crate::help::STT_HELP)]
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
#[command(after_long_help = crate::help::SFX_HELP)]
pub struct SfxArgs {
    /// Text description of the sound effect
    pub text: String,

    /// Output file path. Defaults to ./sfx_<timestamp>.mp3
    #[arg(short, long)]
    pub output: Option<String>,

    /// Duration in seconds (0.5 to 30). Omit to let the model choose.
    #[arg(long)]
    pub duration: Option<f32>,

    /// Prompt influence 0.0-1.0 (default 0.3)
    #[arg(long)]
    pub prompt_influence: Option<f32>,

    /// Loop the generated sound (only supported by eleven_text_to_sound_v2).
    #[arg(long = "loop")]
    pub looping: bool,

    /// Output format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Model ID (e.g. eleven_text_to_sound_v2)
    #[arg(long)]
    pub model: Option<String>,
}

// ── Voices ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum VoicesAction {
    /// List voices in your library (v2)
    #[command(visible_alias = "ls")]
    List {
        /// Filter by search term
        #[arg(long)]
        search: Option<String>,

        /// Sort field (name|created_at_unix)
        #[arg(long, default_value = "name")]
        sort: String,

        /// Sort direction (asc|desc)
        #[arg(long, default_value = "asc")]
        direction: String,

        /// Max results per page (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Include legacy premade voices (/v1 compatibility)
        #[arg(long)]
        show_legacy: bool,

        /// Pagination cursor from a previous response
        #[arg(long, value_name = "TOKEN")]
        next_page_token: Option<String>,

        /// Voice type filter: personal|community|default|workspace|non-default|non-community|saved
        #[arg(long, value_parser = ["personal", "community", "default", "workspace", "non-default", "non-community", "saved"])]
        voice_type: Option<String>,

        /// Category filter: premade|cloned|generated|professional
        #[arg(long, value_parser = ["premade", "cloned", "generated", "professional"])]
        category: Option<String>,

        /// Fine-tuning state (professional voices only)
        #[arg(long, value_parser = ["draft", "not_verified", "not_started", "queued", "fine_tuning", "fine_tuned", "failed", "delayed"])]
        fine_tuning_state: Option<String>,

        /// Filter by collection ID
        #[arg(long)]
        collection_id: Option<String>,

        /// Include the total voice count in the response
        #[arg(long)]
        include_total_count: bool,

        /// Look up specific voice IDs (repeatable, max 100)
        #[arg(long = "voice-id", value_name = "ID")]
        voice_id: Vec<String>,
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
    #[command(after_long_help = crate::help::VOICES_LIBRARY_HELP)]
    Library {
        /// Search term
        #[arg(long)]
        search: Option<String>,

        /// Page number (1-indexed)
        #[arg(long, default_value = "1")]
        page: u32,

        /// Page size (1-100)
        #[arg(long, default_value = "20")]
        page_size: u32,

        /// Category: professional | high_quality | famous
        #[arg(long)]
        category: Option<String>,

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

        /// Locale filter (e.g. en-US)
        #[arg(long)]
        locale: Option<String>,

        /// Use case filter (narration|audiobook|...)
        #[arg(long)]
        use_case: Option<String>,

        /// Filter to featured voices only
        #[arg(long)]
        featured: bool,

        /// Filter voices by minimum notice period in days
        #[arg(long)]
        min_notice_days: Option<u32>,

        /// Include voices with custom rates
        #[arg(long)]
        include_custom_rates: bool,

        /// Include live-moderated voices
        #[arg(long)]
        include_live_moderated: bool,

        /// Filter voices enabled for the reader app
        #[arg(long)]
        reader_app_enabled: bool,

        /// Filter by public owner ID
        #[arg(long)]
        owner_id: Option<String>,

        /// Sort criteria (e.g. cloned_by_count, usage_character_count_1y)
        #[arg(long)]
        sort: Option<String>,
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
    #[command(after_long_help = crate::help::VOICES_DESIGN_HELP)]
    Design {
        /// Text description of the voice
        description: String,

        /// Optional text to read in the preview (auto-generated if omitted).
        /// Must be 100-1000 characters.
        #[arg(long)]
        text: Option<String>,

        /// Directory to save preview files
        #[arg(long)]
        output_dir: Option<String>,

        /// Voice design model: eleven_multilingual_ttv_v2 | eleven_ttv_v3
        #[arg(long, value_parser = ["eleven_multilingual_ttv_v2", "eleven_ttv_v3"])]
        model: Option<String>,

        /// Loudness -1.0 (quietest) to 1.0 (loudest); 0 ≈ -24 LUFS
        #[arg(long)]
        loudness: Option<f32>,

        /// Seed for reproducible generation
        #[arg(long)]
        seed: Option<u32>,

        /// Guidance scale — higher = stick closer to prompt (may sound robotic)
        #[arg(long)]
        guidance_scale: Option<f32>,

        /// Enhance the description with AI before generation
        #[arg(long)]
        enhance: bool,

        /// Return preview IDs only (stream audio via separate endpoint)
        #[arg(long)]
        stream_previews: bool,

        /// Higher quality = better voice but less variety
        #[arg(long)]
        quality: Option<f32>,
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

        /// Confirm deletion. Without this flag the command errors out instead
        /// of silently deleting — required because deletion is irreversible.
        #[arg(long)]
        yes: bool,
    },

    /// Add a shared voice (from the public library) to your collection
    AddShared {
        /// Public user ID that owns the shared voice
        public_user_id: String,

        /// Voice ID in the public library
        voice_id: String,

        /// Name to save the voice under in your library
        #[arg(long)]
        name: String,

        /// Mark the voice as bookmarked after adding
        #[arg(long)]
        bookmarked: Option<bool>,
    },

    /// Find shared voices similar to an audio sample
    Similar {
        /// Audio file to use as the reference sample
        audio_file: String,

        /// Similarity threshold 0.0-2.0 (lower = more similar)
        #[arg(long)]
        similarity_threshold: Option<f32>,

        /// Maximum voices to return (1-100)
        #[arg(long)]
        top_k: Option<u32>,

        /// Gender filter
        #[arg(long)]
        gender: Option<String>,

        /// Age filter
        #[arg(long)]
        age: Option<String>,

        /// Accent filter
        #[arg(long)]
        accent: Option<String>,

        /// Language filter
        #[arg(long)]
        language: Option<String>,

        /// Use case filter
        #[arg(long)]
        use_case: Option<String>,
    },

    /// Edit a voice — rename, re-describe, update labels, add/remove samples
    Edit {
        /// Voice ID to edit
        voice_id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Label pair (key=value). Repeatable.
        #[arg(long = "labels", value_name = "KEY=VALUE")]
        labels: Vec<String>,

        /// Additional sample file to upload. Repeatable.
        #[arg(long = "add-sample", value_name = "FILE")]
        add_sample: Vec<String>,

        /// Sample ID to remove. Repeatable.
        #[arg(long = "remove-sample", value_name = "SAMPLE_ID")]
        remove_sample: Vec<String>,

        /// Run added samples through the background-noise-removal model
        #[arg(long)]
        remove_background_noise: bool,
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

        /// Declare input as raw 16-bit PCM mono @ 16 kHz little-endian
        /// (lower latency than encoded formats).
        #[arg(long)]
        pcm_16k: bool,
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

        /// Output audio format (mp3_44100_128, pcm_44100, ulaw_8000, ...)
        #[arg(long)]
        format: Option<String>,

        /// Stability override 0.0-1.0
        #[arg(long)]
        stability: Option<f32>,

        /// Similarity boost override 0.0-1.0
        #[arg(long)]
        similarity: Option<f32>,

        /// Style exaggeration override 0.0-1.0
        #[arg(long)]
        style: Option<f32>,

        /// Speaker boost override
        #[arg(long)]
        speaker_boost: Option<bool>,

        /// Speed override 0.7-1.2
        #[arg(long)]
        speed: Option<f32>,

        /// Sampling seed for reproducibility (0 to 4_294_967_295)
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4_294_967_295))]
        seed: Option<u32>,

        /// Remove background noise from the input before conversion
        #[arg(long)]
        remove_background_noise: bool,

        /// Latency optimization level (0=none, 4=max)
        #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4))]
        optimize_streaming_latency: Option<u32>,

        /// Declare input as raw 16-bit PCM mono @ 16 kHz (lower latency)
        #[arg(long)]
        pcm_16k: bool,

        /// Zero-retention mode (enterprise only)
        #[arg(long)]
        no_logging: bool,
    },
}

// ── Music ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum MusicAction {
    /// Compose music from a text prompt
    #[command(after_long_help = crate::help::MUSIC_COMPOSE_HELP)]
    Compose(ComposeArgs),

    /// Create a composition plan (free, subject to rate limits)
    Plan {
        /// Text prompt
        prompt: String,

        /// Target length in milliseconds (3000-600000)
        #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
        length_ms: Option<u32>,

        /// Model ID (default music_v1)
        #[arg(long)]
        model: Option<String>,
    },

    /// Generate music with rich metadata (bpm, time_signature, sections)
    Detailed(DetailedArgs),

    /// Stream the generated audio to disk as bytes arrive
    Stream(StreamArgs),

    /// Upload an audio file so it can be referenced by song_id for inpainting
    Upload(UploadArgs),

    /// Split a track into stems (vocals/drums/bass/other)
    #[command(name = "stem-separation", visible_alias = "stems")]
    StemSeparation(StemSeparationArgs),

    /// Generate a score from video content (Apr 2026)
    #[command(name = "video-to-music", visible_alias = "v2m")]
    VideoToMusic(VideoToMusicArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct ComposeArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds. Must be 3000-600000. Only used with --prompt.
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to a composition-plan JSON file (mutually exclusive with PROMPT).
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force the output to be instrumental. Only valid with --prompt.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility (cannot be combined with --prompt)
    #[arg(long)]
    pub seed: Option<u32>,

    /// Strictly enforce per-section durations from the composition plan.
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store the generated song for inpainting (enterprise only)
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign the output mp3 with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct DetailedArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds (3000-600000). Only used with --prompt.
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output audio file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to save the metadata JSON (defaults to <output>.metadata.json)
    #[arg(long, value_name = "PATH")]
    pub save_metadata: Option<String>,

    /// Path to a composition-plan JSON file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force instrumental output.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility
    #[arg(long)]
    pub seed: Option<u32>,

    /// Respect per-section durations
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store song for inpainting
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign output with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct StreamArgs {
    /// Text prompt. Mutually exclusive with --composition-plan.
    #[arg(required_unless_present = "composition_plan")]
    pub prompt: Option<String>,

    /// Target length in milliseconds (3000-600000)
    #[arg(long, value_parser = clap::value_parser!(u32).range(3000..=600000))]
    pub length_ms: Option<u32>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Path to a composition-plan JSON file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["prompt", "length_ms", "force_instrumental"])]
    pub composition_plan: Option<String>,

    /// Model ID (default music_v1)
    #[arg(long)]
    pub model: Option<String>,

    /// Force instrumental output.
    #[arg(long)]
    pub force_instrumental: bool,

    /// Seed for reproducibility
    #[arg(long)]
    pub seed: Option<u32>,

    /// Respect per-section durations
    #[arg(long)]
    pub respect_sections_durations: bool,

    /// Store song for inpainting
    #[arg(long)]
    pub store_for_inpainting: bool,

    /// Sign output with C2PA
    #[arg(long)]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct UploadArgs {
    /// Local audio file to upload
    pub file: String,

    /// Generate and return the composition plan for the uploaded song.
    /// Increases latency; the returned plan can be piped into
    /// `music compose --composition-plan <file>`.
    #[arg(long = "extract-composition-plan")]
    pub extract_composition_plan: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct StemSeparationArgs {
    /// Local audio file to split into stems.
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Directory to write the stem files into. Defaults to ./stems_<timestamp>.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<String>,

    /// Output audio format (codec_samplerate_bitrate, e.g. mp3_44100_128).
    #[arg(long = "output-format", value_name = "FORMAT")]
    pub output_format: Option<String>,

    /// Server-side stem variation id (opaque; see ElevenLabs docs).
    #[arg(long = "stem-variation-id", value_name = "ID")]
    pub stem_variation_id: Option<String>,

    /// Sign each generated mp3 with C2PA.
    #[arg(long = "sign-with-c2pa")]
    pub sign_with_c2pa: bool,
}

#[derive(clap::Args, Debug, Clone)]
pub struct VideoToMusicArgs {
    /// Input video file
    pub file: String,

    /// Optional text description to steer the score
    #[arg(long)]
    pub description: Option<String>,

    /// Style / mood tags (repeatable)
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,

    /// Output audio format (default mp3_44100_128)
    #[arg(long)]
    pub format: Option<String>,

    /// Output audio file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Sign the output with C2PA.
    #[arg(long = "sign-with-c2pa")]
    pub sign_with_c2pa: bool,
}

// ── User ───────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum UserAction {
    /// Get basic user info
    Info,
    /// Get subscription and usage info
    #[command(after_long_help = crate::help::USER_SUBSCRIPTION_HELP)]
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

    /// List the LLMs the Agents backend supports. Hit this before
    /// `agents create --llm …` to avoid the "accepted by clap, rejected
    /// at conversation time" footgun.
    Llms,

    /// Issue a pre-authenticated signed URL for a widget / web session
    /// with the given agent. The URL is short-lived and can be embedded
    /// directly in a browser session.
    #[command(name = "signed-url")]
    SignedUrl {
        /// Agent ID
        agent_id: String,
    },

    /// Create a new agent
    #[command(visible_alias = "new", after_long_help = crate::help::AGENTS_CREATE_HELP)]
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

        /// LLM id. Default gemini-3.1-flash-lite-preview. The Agents backend
        /// enforces its own allowlist — if conversations show 0 output tokens
        /// the LLM fell off the list; switch to a safer default.
        #[arg(long, default_value = "gemini-3.1-flash-lite-preview")]
        llm: String,

        /// Temperature 0.0-1.0
        #[arg(long, default_value = "0.5")]
        temperature: f32,

        /// TTS model id. One of: eleven_turbo_v2, eleven_turbo_v2_5,
        /// eleven_flash_v2, eleven_flash_v2_5, eleven_multilingual_v2,
        /// eleven_v3_conversational. Note: `eleven_v3` (the dialogue/ttv
        /// model) is rejected by the Agents API — use eleven_v3_conversational
        /// for the v3 realtime model instead.
        #[arg(long, default_value = "eleven_flash_v2_5")]
        model_id: String,

        /// Enable expressive v3 prosody. Only honoured with
        /// model_id=eleven_v3_conversational; the server silently drops it on
        /// every other model. Setting this flag without --model-id auto-
        /// upgrades to eleven_v3_conversational. Requires a tier that has
        /// access to expressive TTS (Creator+ at the time of writing).
        #[arg(long)]
        expressive_mode: bool,

        /// Max call duration in seconds. Default 300 (5 min). Bump to 1800
        /// (30 min) or higher for long-form interviews / coaching — calls hang
        /// up hard at this limit regardless of transcript state.
        #[arg(long, default_value = "300")]
        max_duration_seconds: u32,

        /// Register the `voicemail_detection` system tool. Almost always
        /// what you want for outbound phone agents: if the callee's line
        /// goes to voicemail, the agent hangs up cleanly rather than
        /// recording its opening over their answerphone. Default OFF so
        /// inbound/web-widget agents aren't affected.
        #[arg(long)]
        voicemail_detection: bool,

        /// If set, leave this message on voicemail instead of hanging up.
        /// Implies --voicemail-detection. Supports {{placeholder}}
        /// interpolation from --dynamic-variables on the call.
        #[arg(long, value_name = "TEXT")]
        voicemail_message: Option<String>,
    },

    /// Update (PATCH) an agent's config from a JSON file
    #[command(after_long_help = crate::help::AGENTS_UPDATE_HELP)]
    Update {
        /// Agent ID
        agent_id: String,
        /// Path to a JSON file whose contents are the PATCH body.
        /// Pass-through — lets you edit system_prompt, voice_id, tools,
        /// knowledge_base, etc. without recreating the agent.
        #[arg(long, value_name = "PATH")]
        patch: String,
    },

    /// Duplicate an agent (clone config to a new agent_id)
    Duplicate {
        /// Agent ID to duplicate
        agent_id: String,
        /// Optional name override for the new agent
        #[arg(long)]
        name: Option<String>,
    },

    /// Delete an agent (irreversible — cascades to conversations, attached KB
    /// entries, and tool-dep edges)
    #[command(visible_alias = "rm")]
    Delete {
        /// Agent ID
        agent_id: String,

        /// Confirm deletion. Required because agent deletion is irreversible
        /// server-side.
        #[arg(long)]
        yes: bool,
    },

    /// Add a knowledge base document and attach it to an agent
    #[command(after_long_help = crate::help::AGENTS_ADD_KNOWLEDGE_HELP)]
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

    /// Manage workspace-level tools
    Tools {
        #[command(subcommand)]
        action: AgentsToolsAction,
    },

    /// Browse / search / refresh the workspace knowledge base. Use
    /// `agents add-knowledge` to add + attach; these subcommands inspect
    /// what's already in the workspace.
    Knowledge {
        #[command(subcommand)]
        action: AgentsKnowledgeAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsKnowledgeAction {
    /// List knowledge-base documents in the workspace
    #[command(visible_alias = "ls")]
    List {
        /// Free-text search across doc names/content
        #[arg(long)]
        search: Option<String>,
        /// Page size (1-100)
        #[arg(long, default_value = "30")]
        page_size: u32,
        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,
    },

    /// Search knowledge-base content (chunk-level, used for RAG debug)
    Search {
        /// Query string
        query: String,
        /// Restrict to a specific document ID
        #[arg(long)]
        document_id: Option<String>,
        /// Max results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Refresh a URL-backed document (re-fetches the source page)
    Refresh {
        /// Knowledge-base document ID
        document_id: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum AgentsToolsAction {
    /// List tools
    #[command(visible_alias = "ls")]
    List,

    /// Show full tool config
    #[command(visible_alias = "get")]
    Show {
        /// Tool ID
        tool_id: String,
    },

    /// Create a tool from a JSON config file
    #[command(visible_alias = "new")]
    Create {
        /// Path to a JSON file that becomes the POST body verbatim.
        /// The tools API surface is wide (system/client/webhook/mcp tool
        /// types, many fields each) — pass the JSON directly instead of
        /// modelling every field as a flag.
        #[arg(long, value_name = "PATH")]
        config: String,
    },

    /// Update (PATCH) a tool from a JSON file
    Update {
        /// Tool ID
        tool_id: String,
        /// Path to a JSON file whose contents are the PATCH body.
        #[arg(long, value_name = "PATH")]
        patch: String,
    },

    /// Delete a tool (requires --yes)
    #[command(visible_alias = "rm")]
    Delete {
        /// Tool ID
        tool_id: String,
        /// Confirm deletion. Without --yes the command errors out.
        #[arg(long)]
        yes: bool,
    },

    /// List agents that depend on this tool
    #[command(visible_alias = "dependents")]
    Deps {
        /// Tool ID
        tool_id: String,
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

    /// Download the audio recording for a conversation. Writes to
    /// ./conv_<id>.mp3 by default; override with --output.
    Audio {
        /// Conversation ID
        conversation_id: String,
        /// Output path (default: conv_<id>.mp3)
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
}

// ── Phone ──────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneAction {
    /// List phone numbers
    #[command(visible_alias = "ls")]
    List,

    /// Make an outbound call with an agent
    #[command(after_long_help = crate::help::PHONE_CALL_HELP)]
    Call {
        /// Agent ID to handle the call
        agent_id: String,

        /// Phone number ID to call from
        #[arg(long)]
        from_id: String,

        /// E.164 number to call (+1...)
        #[arg(long)]
        to: String,

        /// Per-call dynamic variables as JSON (e.g. '{"name":"Alex"}').
        /// Agent prompts can interpolate these as {{name}}. Prefix with `@`
        /// to load from a file: '@vars.json'. Keep under 4KB.
        #[arg(long = "dynamic-variables", value_name = "JSON_OR_@FILE")]
        dynamic_variables: Option<String>,

        /// Full `conversation_initiation_client_data` object as JSON (or
        /// @file.json). Spec-backed siblings supported: `dynamic_variables`,
        /// `conversation_config_override` (override agent.first_message,
        /// tts.voice_id/stability/speed, agent.prompt.prompt/llm, etc.
        /// per-call), `custom_llm_extra_body`, `user_id`, `source_info`,
        /// `branch_id`, `environment`, `starting_workflow_node_id`. When
        /// both this and --dynamic-variables are passed, the latter is
        /// merged into the client-data object.
        #[arg(long = "client-data", value_name = "JSON_OR_@FILE")]
        client_data: Option<String>,

        /// Record the call (Twilio / SIP trunk). Maps to the outbound-call
        /// body's `call_recording_enabled`. Defaults to the server's own
        /// default when omitted.
        #[arg(long)]
        record: bool,

        /// Max seconds to ring the callee before giving up. Maps to
        /// `telephony_call_config.ringing_timeout_secs` on the
        /// outbound-call body.
        #[arg(long, value_name = "SECS")]
        ringing_timeout_secs: Option<u32>,
    },

    /// Batch outbound calls (CSV or JSON recipients)
    Batch {
        #[command(subcommand)]
        action: PhoneBatchAction,
    },

    /// WhatsApp channel: outbound calls, messages, and accounts
    Whatsapp {
        #[command(subcommand)]
        action: PhoneWhatsappAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneBatchAction {
    /// Submit a batch of outbound calls
    Submit {
        /// Agent ID that will handle the calls
        #[arg(long = "agent")]
        agent_id: String,

        /// Phone number ID to dial from
        #[arg(long = "phone-number")]
        phone_number_id: String,

        /// Path to CSV or JSON recipients file (use `-` for stdin)
        #[arg(long)]
        recipients: String,

        /// Optional human-readable batch name
        #[arg(long)]
        name: Option<String>,

        /// Optional scheduled start time as a Unix timestamp
        #[arg(long, value_name = "UNIX")]
        scheduled_time_unix: Option<i64>,
    },

    /// List batch calls in the current workspace
    #[command(visible_alias = "ls")]
    List {
        /// Page size (1-100)
        #[arg(long)]
        page_size: Option<u32>,

        /// Pagination cursor
        #[arg(long)]
        cursor: Option<String>,

        /// Filter by batch status
        #[arg(long)]
        status: Option<String>,

        /// Filter by agent ID
        #[arg(long)]
        agent_id: Option<String>,
    },

    /// Show detail for a batch (includes per-call status)
    #[command(visible_alias = "get")]
    Show {
        /// Batch ID
        batch_id: String,
    },

    /// Cancel a batch (reversible via `phone batch retry`)
    Cancel {
        /// Batch ID
        batch_id: String,
    },

    /// Retry a batch (re-dials failed/pending recipients)
    Retry {
        /// Batch ID
        batch_id: String,
    },

    /// Delete a batch
    #[command(visible_alias = "rm")]
    Delete {
        /// Batch ID
        batch_id: String,

        /// Confirm deletion. Required because it is irreversible.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneWhatsappAction {
    /// Place an outbound WhatsApp voice call.
    ///
    /// WhatsApp requires a pre-approved call-permission-request template.
    Call {
        /// Agent ID to handle the call
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp phone number ID to call from (sending business number)
        #[arg(long = "whatsapp-phone-number")]
        whatsapp_phone_number_id: String,

        /// WhatsApp user ID of the recipient
        #[arg(long = "whatsapp-user")]
        whatsapp_user_id: String,

        /// Name of the pre-approved WhatsApp call-permission-request template
        #[arg(long = "permission-template")]
        permission_template_name: String,

        /// Language code for the permission template (e.g. en_US)
        #[arg(long = "permission-template-language")]
        permission_template_language_code: String,
    },

    /// Send an outbound WhatsApp message via a pre-approved template.
    ///
    /// WhatsApp rejects free-form text — every outbound message must use
    /// a pre-approved template. Repeat --template-param to fill body vars.
    Message {
        /// Agent ID associated with the message
        #[arg(long = "agent")]
        agent_id: String,

        /// WhatsApp phone number ID to send from (sending business number)
        #[arg(long = "whatsapp-phone-number")]
        whatsapp_phone_number_id: String,

        /// WhatsApp user ID of the recipient
        #[arg(long = "whatsapp-user")]
        whatsapp_user_id: String,

        /// Pre-approved WhatsApp template name
        #[arg(long = "template")]
        template_name: String,

        /// Language code for the template (e.g. en_US)
        #[arg(long = "template-language")]
        template_language_code: String,

        /// Template body parameter as key=value. Repeatable.
        #[arg(long = "template-param", value_name = "KEY=VALUE")]
        template_params: Vec<String>,

        /// Path to a JSON file whose contents become
        /// `conversation_initiation_client_data`.
        #[arg(long = "client-data")]
        client_data: Option<String>,
    },

    /// Manage WhatsApp accounts
    Accounts {
        #[command(subcommand)]
        action: PhoneWhatsappAccountsAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PhoneWhatsappAccountsAction {
    /// List WhatsApp accounts
    #[command(visible_alias = "ls")]
    List,

    /// Show details for a WhatsApp account
    #[command(visible_alias = "get")]
    Show {
        /// WhatsApp account ID
        account_id: String,
    },

    /// PATCH a WhatsApp account with partial JSON from a file
    Update {
        /// WhatsApp account ID
        account_id: String,

        /// Path to a JSON file whose contents become the PATCH body
        #[arg(long)]
        patch: String,
    },

    /// Delete a WhatsApp account
    #[command(visible_alias = "rm")]
    Delete {
        /// WhatsApp account ID
        account_id: String,

        /// Confirm deletion. Required because it is irreversible.
        #[arg(long)]
        yes: bool,
    },
}

// ── History ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum HistoryAction {
    /// List recent generation history
    #[command(visible_alias = "ls", after_long_help = crate::help::HISTORY_LIST_HELP)]
    List {
        /// Page size (1-1000)
        #[arg(long, default_value = "20")]
        page_size: u32,

        /// Paginate — start after this history_item_id (returned by previous page)
        #[arg(long, value_name = "ID")]
        start_after: Option<String>,

        /// Filter by voice ID
        #[arg(long)]
        voice_id: Option<String>,

        /// Filter by model ID
        #[arg(long)]
        model_id: Option<String>,

        /// Only include items before this Unix timestamp
        #[arg(long, value_name = "UNIX")]
        before: Option<i64>,

        /// Only include items after this Unix timestamp
        #[arg(long, value_name = "UNIX")]
        after: Option<i64>,

        /// Sort direction: asc | desc (default: desc)
        #[arg(long, value_parser = ["asc", "desc"])]
        sort_direction: Option<String>,

        /// Search text within history items
        #[arg(long)]
        search: Option<String>,

        /// Filter by source: TTS | STS
        #[arg(long, value_parser = ["TTS", "STS"])]
        source: Option<String>,
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
    #[command(after_long_help = crate::help::CONFIG_INIT_HELP)]
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
    #[command(after_long_help = crate::help::SKILL_INSTALL_HELP)]
    Install,
    /// Check which platforms have the skill installed
    Status,
}

// ── Dialogue ───────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
#[command(after_long_help = crate::help::DIALOGUE_HELP)]
pub struct DialogueArgs {
    /// Dialogue inputs as positional triples `label:voice_id:text`, e.g.
    /// `"Alice:v_1234:Hello there"`. Alternatively, pass a single JSON file
    /// path and the CLI will detect it via the `.json` extension. Pass `-`
    /// to read JSON from stdin.
    #[arg(value_name = "LINE")]
    pub positional: Vec<String>,

    /// Path to a JSON file containing an array of `{text, voice_id}`
    /// entries. Mutually exclusive with positional triples. Use `-` for stdin.
    #[arg(long, value_name = "PATH", conflicts_with = "positional")]
    pub input: Option<String>,

    /// Output file path. Defaults to ./dialogue_<timestamp>.<ext>.
    #[arg(short, long)]
    pub output: Option<String>,

    /// Model ID (default eleven_v3).
    #[arg(long)]
    pub model: Option<String>,

    /// Output format (e.g. mp3_44100_128, pcm_44100, ulaw_8000).
    #[arg(long)]
    pub format: Option<String>,

    /// Route to the streaming endpoint (/v1/text-to-dialogue/stream).
    #[arg(long)]
    pub stream: bool,

    /// Return per-character alignment alongside the audio.
    #[arg(long)]
    pub with_timestamps: bool,

    /// Path to save alignment JSON when --with-timestamps is set. Defaults
    /// to <audio>.timings.json.
    #[arg(long, value_name = "PATH")]
    pub save_timestamps: Option<String>,

    /// Write raw audio bytes to stdout instead of a file (implies --quiet).
    /// Only supported on the non-timestamp variants.
    #[arg(long)]
    pub stdout: bool,

    /// Sampling seed for reproducibility (0..=4_294_967_295).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4_294_967_295))]
    pub seed: Option<u32>,

    /// Stability 0.0-1.0.
    #[arg(long)]
    pub stability: Option<f32>,

    /// Similarity boost 0.0-1.0.
    #[arg(long)]
    pub similarity: Option<f32>,

    /// Style exaggeration 0.0-1.0.
    #[arg(long)]
    pub style: Option<f32>,

    /// Speaker boost (default on).
    #[arg(long)]
    pub speaker_boost: Option<bool>,

    /// ISO language code (mostly for v3; optional).
    #[arg(long)]
    pub language: Option<String>,

    /// Text normalization mode: auto | on | off.
    #[arg(long, value_parser = ["auto", "on", "off"])]
    pub apply_text_normalization: Option<String>,

    /// Latency optimization level (0=none, 4=max).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=4))]
    pub optimize_streaming_latency: Option<u32>,

    /// Zero-retention mode (enterprise only).
    #[arg(long)]
    pub no_logging: bool,
}

// ── Align ──────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
#[command(after_long_help = crate::help::ALIGN_HELP)]
pub struct AlignArgs {
    /// Input audio file (the recording to align against).
    pub audio: String,

    /// Inline transcript text, OR a path to a transcript file (detected when
    /// the value is a short single-line path that exists on disk). For
    /// anything non-trivial prefer --transcript-file.
    #[arg(conflicts_with = "transcript_file")]
    pub transcript: Option<String>,

    /// Path to a transcript file. Use this for transcripts with newlines or
    /// paths containing colons.
    #[arg(long, value_name = "PATH")]
    pub transcript_file: Option<String>,

    /// Send the audio as a spooled file — required when the file is very
    /// large (>~50MB).
    #[arg(long)]
    pub enabled_spooled_file: bool,

    /// Save the full JSON response (characters + words) to a file.
    #[arg(short, long)]
    pub output: Option<String>,
}

// ── Dubbing ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum DubbingAction {
    /// Create a new dubbing job from a local file or URL
    #[command(visible_alias = "new", after_long_help = crate::help::DUBBING_CREATE_HELP)]
    Create(DubbingCreateArgs),

    /// List your dubbing jobs
    #[command(visible_alias = "ls")]
    List {
        /// Filter by status (dubbing | dubbed | failed | …)
        #[arg(long)]
        dubbing_status: Option<String>,

        /// Filter by creator: only_me | admin | workspace
        #[arg(long)]
        filter_by_creator: Option<String>,

        /// Page size (max 100)
        #[arg(long)]
        page_size: Option<u32>,
    },

    /// Get a dubbing job's full status
    #[command(visible_alias = "get")]
    Show {
        /// Dubbing job ID
        dubbing_id: String,
    },

    /// Delete a dubbing job
    #[command(visible_alias = "rm")]
    Delete {
        /// Dubbing job ID
        dubbing_id: String,

        /// Confirm deletion (required — deletion is irreversible).
        #[arg(long)]
        yes: bool,
    },

    /// Download the dubbed audio/video for a language
    GetAudio {
        /// Dubbing job ID
        dubbing_id: String,

        /// ISO language code (es, fr, de, …)
        language_code: String,

        /// Output file path (default: dub_<id>_<lang>.mp4)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Download the transcript for a dubbed language in the requested format
    GetTranscript {
        /// Dubbing job ID
        dubbing_id: String,

        /// ISO language code
        language_code: String,

        /// Transcript format
        #[arg(long, value_parser = ["srt", "webvtt", "json"])]
        format: String,

        /// Output file path (default: dub_<id>_<lang>.<ext>)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Editable-dub (Studio) operations — require `--dubbing-studio=true` at create
    Resource {
        #[command(subcommand)]
        action: DubbingResourceAction,
    },
}

#[derive(clap::Args, Debug, Clone)]
pub struct DubbingCreateArgs {
    /// Target language ISO code (required)
    #[arg(long)]
    pub target_lang: String,

    /// Source media file (mutually exclusive with --source-url)
    #[arg(long, conflicts_with = "source_url")]
    pub file: Option<String>,

    /// Publicly reachable URL to source media (mutually exclusive with --file)
    #[arg(long, conflicts_with = "file")]
    pub source_url: Option<String>,

    /// Source language ISO code (auto-detect when omitted)
    #[arg(long)]
    pub source_lang: Option<String>,

    /// Number of speakers in the source media (1-32)
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=32))]
    pub num_speakers: Option<u32>,

    /// Embed the ElevenLabs watermark in the output
    #[arg(long)]
    pub watermark: Option<bool>,

    /// Start time of the clip to dub, in seconds
    #[arg(long)]
    pub start_time: Option<u32>,

    /// End time of the clip to dub, in seconds
    #[arg(long)]
    pub end_time: Option<u32>,

    /// Use the highest available video resolution for rendering
    #[arg(long)]
    pub highest_resolution: Option<bool>,

    /// Drop background audio (music/SFX) from the dubbed output
    #[arg(long)]
    pub drop_background_audio: Option<bool>,

    /// Run the profanity filter before dubbing
    #[arg(long)]
    pub use_profanity_filter: Option<bool>,

    /// Return a Studio-editable dub instead of a one-shot render
    #[arg(long)]
    pub dubbing_studio: Option<bool>,

    /// Disable voice cloning — use the default voice per speaker instead
    #[arg(long)]
    pub disable_voice_cloning: Option<bool>,

    /// Dubbing mode: automatic | manual
    #[arg(long, value_parser = ["automatic", "manual"])]
    pub mode: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DubbingResourceAction {
    /// Re-run transcription on the source media
    Transcribe {
        /// Dubbing job ID
        dubbing_id: String,

        /// JSON file with request-body overrides
        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-run translation from the transcript
    Translate {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-run the dub step
    Dub {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Re-render the dubbed output for a target language
    Render {
        dubbing_id: String,

        /// Target language ISO code
        language_code: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },

    /// Migrate legacy segment metadata to the current schema
    MigrateSegments {
        dubbing_id: String,

        #[arg(long, value_name = "PATH")]
        patch: Option<String>,
    },
}

// ── Dict (pronunciation dictionaries) ──────────────────────────────────────

#[derive(Subcommand, Debug, Clone)]
pub enum DictAction {
    /// List pronunciation dictionaries
    #[command(visible_alias = "ls")]
    List {
        /// Pagination cursor from a previous page
        #[arg(long)]
        cursor: Option<String>,

        /// Page size (max 100)
        #[arg(long)]
        page_size: Option<u32>,

        /// Filter by substring match on name
        #[arg(long)]
        search: Option<String>,
    },

    /// Upload a PLS/lexicon file as a new pronunciation dictionary
    AddFile {
        /// Dictionary name (shown in the library)
        name: String,

        /// Path to a PLS XML / lexicon file
        file: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Workspace access mode: admin | editor | viewer
        #[arg(long)]
        workspace_access: Option<String>,
    },

    /// Create a new dictionary from `--rule` / `--alias-rule` flags (no file)
    #[command(after_long_help = crate::help::DICT_ADD_RULES_HELP)]
    AddRules {
        /// Dictionary name
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Workspace access mode: admin | editor | viewer
        #[arg(long)]
        workspace_access: Option<String>,

        /// Phoneme rule WORD:PHONEME (IPA). Repeatable.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS (spoken as alias). Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,
    },

    /// Show a dictionary's metadata
    #[command(visible_alias = "get")]
    Show {
        /// Dictionary ID
        id: String,
    },

    /// Update dictionary metadata or archive it
    Update {
        /// Dictionary ID
        id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Archive the dictionary (reversible server-side)
        #[arg(long)]
        archive: bool,
    },

    /// Download the rendered PLS XML for a dictionary version
    Download {
        /// Dictionary ID
        id: String,

        /// Version ID. Omit to use the latest version.
        #[arg(long)]
        version: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Replace every rule in the dictionary (creates a new version)
    SetRules {
        /// Dictionary ID
        id: String,

        /// Phoneme rule WORD:PHONEME (IPA). Repeatable.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS. Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,

        /// Case-sensitive matching when applying rules.
        #[arg(long)]
        case_sensitive: Option<bool>,

        /// Only match on whole-word boundaries.
        #[arg(long)]
        word_boundaries: Option<bool>,
    },

    /// Append new rules to an existing dictionary (creates a new version)
    AddRulesTo {
        /// Dictionary ID
        id: String,

        /// Phoneme rule WORD:PHONEME. Repeatable.
        #[arg(long = "rule", value_name = "WORD:PHONEME")]
        rule: Vec<String>,

        /// Alias rule WORD:ALIAS. Repeatable.
        #[arg(long = "alias-rule", value_name = "WORD:ALIAS")]
        alias_rule: Vec<String>,
    },

    /// Remove rules by their `string_to_replace` value
    RemoveRules {
        /// Dictionary ID
        id: String,

        /// The WORD whose rule should be dropped. Repeatable.
        #[arg(long, value_name = "WORD")]
        word: Vec<String>,
    },
}

// ── Doctor ─────────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Skip a named check (repeatable). Known names:
    /// config_file, api_key, env_shadow, api_key_scope, network,
    /// ffmpeg, disk_write, output_dir.
    #[arg(long = "skip", value_name = "NAME")]
    pub skip: Vec<String>,

    /// Timeout in milliseconds for the network reachability probe
    /// and API-scope probes (default 5000).
    #[arg(long = "timeout-ms", value_name = "MS", default_value_t = 5000)]
    pub timeout_ms: u64,
}
