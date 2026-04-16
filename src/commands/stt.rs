//! speech-to-text: POST /v1/speech-to-text (multipart + optional query params).
//!
//! Grounded against the official ElevenLabs REST spec (mirrored by the
//! Fern-generated `@elevenlabs/elevenlabs-js` v2.43 SDK):
//!   - model_id: scribe_v2 (default) | scribe_v1
//!   - timestamps_granularity: none | word | character
//!   - keyterms / entity_detection / entity_redaction: repeated form fields
//!   - additional_formats: single JSON-stringified array form field
//!   - webhook_metadata: single JSON-stringified form field
//!   - enable_logging: query parameter (not form field)
//!   - file vs cloud_storage_url vs source_url: mutually exclusive inputs

use base64::Engine as _;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::cli::SttArgs;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

// ── Output shape ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CharTiming {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<f64>,
}

#[derive(Serialize)]
struct WordTiming {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<f64>,
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    speaker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprob: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    characters: Option<Vec<CharTiming>>,
}

#[derive(Serialize)]
struct Entity {
    text: String,
    entity_type: String,
    start_char: u64,
    end_char: u64,
}

#[derive(Serialize)]
struct ExportedFormat {
    requested_format: String,
    file_extension: String,
    content_type: String,
    saved_path: Option<String>,
}

#[derive(Serialize, Default)]
struct SavedFiles {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    words: Option<String>,
}

#[derive(Serialize)]
struct ChannelTranscript {
    channel_index: Option<u64>,
    text: String,
    words: Vec<WordTiming>,
}

#[derive(Serialize)]
struct SttResult {
    input: String,
    model: String,
    timestamps_granularity: String,
    diarized: bool,
    multi_channel: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_probability: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transcription_id: Option<String>,
    text: String,
    words: Vec<WordTiming>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    channels: Vec<ChannelTranscript>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    entities: Vec<Entity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    exported_formats: Vec<ExportedFormat>,
    #[serde(skip_serializing_if = "is_default_saved")]
    saved: SavedFiles,
    webhook_pending: bool,
}

fn is_default_saved(s: &SavedFiles) -> bool {
    s.text.is_none() && s.raw.is_none() && s.words.is_none()
}

// ── Entry ──────────────────────────────────────────────────────────────────

pub async fn run(ctx: Ctx, args: SttArgs) -> Result<(), AppError> {
    validate_args(&args)?;

    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    let form = build_form(&args).await?;
    let query = enable_logging_query(&args);

    let resp: serde_json::Value = if let Some(q) = &query {
        client
            .post_multipart_json_with_query("/v1/speech-to-text", q, form)
            .await?
    } else {
        client
            .post_multipart_json("/v1/speech-to-text", form)
            .await?
    };

    // Webhook responses don't carry the full transcript — return a stub.
    if args.webhook {
        return emit_webhook_pending(ctx, &args, &resp).await;
    }

    let input_label = input_label(&args);
    let mut result = build_result(&args, &input_label, &resp);

    // Save transcript text if requested.
    if let Some(out) = &args.output {
        tokio::fs::write(out, &result.text)
            .await
            .map_err(AppError::Io)?;
        result.saved.text = Some(out.clone());
    }

    // Save raw JSON response.
    if let Some(out) = &args.save_raw {
        let raw = serde_json::to_vec_pretty(&resp)
            .map_err(|e| AppError::Http(format!("serialize raw response: {e}")))?;
        tokio::fs::write(out, raw).await.map_err(AppError::Io)?;
        result.saved.raw = Some(out.clone());
    }

    // Save words-with-timings array (for lyric/subtitle pipelines).
    if let Some(out) = &args.save_words {
        let words_json = serde_json::to_vec_pretty(&result.words)
            .map_err(|e| AppError::Http(format!("serialize words: {e}")))?;
        tokio::fs::write(out, words_json)
            .await
            .map_err(AppError::Io)?;
        result.saved.words = Some(out.clone());
    }

    // Save additional_formats content (SRT, DOCX, ...) to disk.
    write_exported_formats(&args, &resp, &mut result).await?;

    output::print_success_or(ctx, &result, print_human);

    Ok(())
}

// ── Validation ─────────────────────────────────────────────────────────────

fn validate_args(args: &SttArgs) -> Result<(), AppError> {
    let source_count = [
        args.file.is_some(),
        args.from_url.is_some(),
        args.source_url.is_some(),
    ]
    .iter()
    .filter(|x| **x)
    .count();

    if source_count == 0 {
        return Err(AppError::InvalidInput(
            "provide a file path, --from-url <https://...>, or --source-url <url>".into(),
        ));
    }
    if source_count > 1 {
        return Err(AppError::InvalidInput(
            "pass only one of <FILE>, --from-url, --source-url".into(),
        ));
    }
    if let Some(p) = &args.file {
        let path = Path::new(p);
        if !path.exists() {
            return Err(AppError::InvalidInput(format!(
                "file does not exist: {}",
                path.display()
            )));
        }
    }

    if args.diarization_threshold.is_some() && !args.diarize {
        return Err(AppError::InvalidInput(
            "--diarization-threshold requires --diarize".into(),
        ));
    }
    if args.diarization_threshold.is_some() && args.num_speakers.is_some() {
        return Err(AppError::InvalidInput(
            "--diarization-threshold cannot be combined with --num-speakers".into(),
        ));
    }
    if let Some(t) = args.diarization_threshold {
        if !(0.0..=1.0).contains(&t) {
            return Err(AppError::InvalidInput(
                "--diarization-threshold must be between 0.0 and 1.0".into(),
            ));
        }
    }
    if let Some(t) = args.temperature {
        if !(0.0..=2.0).contains(&t) {
            return Err(AppError::InvalidInput(
                "--temperature must be between 0.0 and 2.0".into(),
            ));
        }
    }
    if args.detect_speaker_roles && !args.diarize {
        return Err(AppError::InvalidInput(
            "--detect-speaker-roles requires --diarize".into(),
        ));
    }
    if args.no_verbatim && args.model != "scribe_v2" {
        return Err(AppError::InvalidInput(
            "--no-verbatim is only supported by scribe_v2".into(),
        ));
    }
    if args.keyterms.len() > 1000 {
        return Err(AppError::InvalidInput(
            "--keyterm may be specified at most 1000 times".into(),
        ));
    }
    for k in &args.keyterms {
        if k.len() > 50 {
            return Err(AppError::InvalidInput(format!(
                "--keyterm '{k}' exceeds 50 characters"
            )));
        }
    }
    if args.webhook_id.is_some() && !args.webhook {
        return Err(AppError::InvalidInput(
            "--webhook-id requires --webhook".into(),
        ));
    }
    if let Some(md) = &args.webhook_metadata {
        serde_json::from_str::<serde_json::Value>(md).map_err(|e| {
            AppError::InvalidInput(format!("--webhook-metadata is not valid JSON: {e}"))
        })?;
    }

    Ok(())
}

// ── Form / query construction ──────────────────────────────────────────────

async fn build_form(args: &SttArgs) -> Result<reqwest::multipart::Form, AppError> {
    let mut form = reqwest::multipart::Form::new().text("model_id", args.model.clone());

    // Source: file vs cloud_storage_url vs source_url. Validation already ran.
    if let Some(fp) = &args.file {
        let path = Path::new(fp);
        let bytes = crate::commands::read_file_bytes(path).await?;
        let mime = crate::commands::mime_for_path(path);
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "audio".to_string());
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        form = form.part("file", part);
    } else if let Some(url) = &args.from_url {
        form = form.text("cloud_storage_url", url.clone());
    } else if let Some(url) = &args.source_url {
        form = form.text("source_url", url.clone());
    }

    if let Some(lang) = &args.language {
        form = form.text("language_code", lang.clone());
    }

    // audio_events: API default is true. Only send the field when user overrides.
    if args.no_audio_events {
        form = form.text("tag_audio_events", "false");
    } else if args.audio_events {
        form = form.text("tag_audio_events", "true");
    }

    // timestamps_granularity: always send (clap default is "word").
    form = form.text("timestamps_granularity", args.timestamps.clone());

    if args.diarize {
        form = form.text("diarize", "true");
    }
    if let Some(n) = args.num_speakers {
        form = form.text("num_speakers", n.to_string());
    }
    if let Some(t) = args.diarization_threshold {
        form = form.text("diarization_threshold", t.to_string());
    }
    if args.detect_speaker_roles {
        form = form.text("detect_speaker_roles", "true");
    }
    if args.no_verbatim {
        form = form.text("no_verbatim", "true");
    }
    if args.multi_channel {
        form = form.text("use_multi_channel", "true");
    }
    if args.pcm_16k {
        form = form.text("file_format", "pcm_s16le_16");
    }
    if let Some(t) = args.temperature {
        form = form.text("temperature", t.to_string());
    }
    if let Some(seed) = args.seed {
        form = form.text("seed", seed.to_string());
    }

    // Keyterms: repeated form field with same name (per Fern client).
    for k in &args.keyterms {
        form = form.text("keyterms", k.clone());
    }

    // Entity detection/redaction: repeated form fields.
    for e in &args.detect_entities {
        form = form.text("entity_detection", e.clone());
    }
    for e in &args.redact_entities {
        form = form.text("entity_redaction", e.clone());
    }
    if let Some(mode) = &args.redaction_mode {
        form = form.text("entity_redaction_mode", mode.clone());
    }

    // additional_formats: single JSON-stringified field.
    if let Some(af_json) = build_additional_formats(args)? {
        form = form.text("additional_formats", af_json);
    }

    if args.webhook {
        form = form.text("webhook", "true");
    }
    if let Some(id) = &args.webhook_id {
        form = form.text("webhook_id", id.clone());
    }
    if let Some(md) = &args.webhook_metadata {
        form = form.text("webhook_metadata", md.clone());
    }

    Ok(form)
}

fn enable_logging_query(args: &SttArgs) -> Option<Vec<(&'static str, String)>> {
    if args.no_logging {
        Some(vec![("enable_logging", "false".to_string())])
    } else {
        None
    }
}

fn build_additional_formats(args: &SttArgs) -> Result<Option<String>, AppError> {
    if args.formats.is_empty() {
        return Ok(None);
    }
    let mut arr: Vec<serde_json::Value> = Vec::with_capacity(args.formats.len());
    for fmt in &args.formats {
        let mut obj = serde_json::Map::new();
        obj.insert("format".into(), serde_json::Value::String(fmt.clone()));
        if args.format_include_speakers {
            obj.insert("include_speakers".into(), true.into());
        }
        if args.format_include_timestamps {
            obj.insert("include_timestamps".into(), true.into());
        }
        if let Some(s) = args.format_segment_on_silence {
            obj.insert(
                "segment_on_silence_longer_than_s".into(),
                serde_json::Value::from(s as f64),
            );
        }
        if let Some(d) = args.format_max_segment_duration {
            obj.insert(
                "max_segment_duration_s".into(),
                serde_json::Value::from(d as f64),
            );
        }
        if let Some(c) = args.format_max_segment_chars {
            obj.insert("max_segment_chars".into(), serde_json::Value::from(c));
        }
        // max_characters_per_line only applies to srt/txt per the SDK schema.
        if matches!(fmt.as_str(), "srt" | "txt") {
            if let Some(c) = args.format_max_chars_per_line {
                obj.insert("max_characters_per_line".into(), serde_json::Value::from(c));
            }
        }
        arr.push(serde_json::Value::Object(obj));
    }
    serde_json::to_string(&arr)
        .map(Some)
        .map_err(|e| AppError::Http(format!("serialize additional_formats: {e}")))
}

// ── Response parsing ───────────────────────────────────────────────────────

fn input_label(args: &SttArgs) -> String {
    if let Some(f) = &args.file {
        return Path::new(f).display().to_string();
    }
    if let Some(u) = &args.from_url {
        return u.clone();
    }
    if let Some(u) = &args.source_url {
        return u.clone();
    }
    String::new()
}

fn build_result(args: &SttArgs, input: &str, resp: &serde_json::Value) -> SttResult {
    // Multichannel response has a top-level "transcripts" array; otherwise the
    // response IS the chunk.
    let (channels, primary, audio_duration, transcription_id) =
        if let Some(transcripts) = resp.get("transcripts").and_then(|t| t.as_array()) {
            let mut chans: Vec<ChannelTranscript> = Vec::with_capacity(transcripts.len());
            let mut merged_text: Vec<String> = Vec::new();
            let mut merged_words: Vec<WordTiming> = Vec::new();
            let mut any_lang: Option<String> = None;
            let mut any_lp: Option<f64> = None;
            for t in transcripts {
                let words = parse_words(t.get("words"));
                let text = t
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let channel_index = t.get("channel_index").and_then(|v| v.as_u64());
                if any_lang.is_none() {
                    any_lang = t
                        .get("language_code")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                }
                if any_lp.is_none() {
                    any_lp = t.get("language_probability").and_then(|v| v.as_f64());
                }
                merged_text.push(format!(
                    "[channel {}]\n{}",
                    channel_index.map(|i| i.to_string()).unwrap_or_default(),
                    text.trim()
                ));
                merged_words.extend(words.iter().map(clone_word));
                chans.push(ChannelTranscript {
                    channel_index,
                    text,
                    words,
                });
            }
            let primary = PrimaryParsed {
                text: merged_text.join("\n\n"),
                words: merged_words,
                language_code: any_lang,
                language_probability: any_lp,
                entities: Vec::new(),
            };
            let duration = resp.get("audio_duration_secs").and_then(|v| v.as_f64());
            let tid = resp
                .get("transcription_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            (chans, primary, duration, tid)
        } else {
            let words = parse_words(resp.get("words"));
            let text = if args.diarize {
                format_diarized(&words)
            } else {
                resp.get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            let primary = PrimaryParsed {
                text,
                words,
                language_code: resp
                    .get("language_code")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                language_probability: resp.get("language_probability").and_then(|v| v.as_f64()),
                entities: parse_entities(resp.get("entities")),
            };
            let duration = resp.get("audio_duration_secs").and_then(|v| v.as_f64());
            let tid = resp
                .get("transcription_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            (Vec::new(), primary, duration, tid)
        };

    SttResult {
        input: input.to_string(),
        model: args.model.clone(),
        timestamps_granularity: args.timestamps.clone(),
        diarized: args.diarize,
        multi_channel: args.multi_channel,
        language_code: primary.language_code,
        language_probability: primary.language_probability,
        audio_duration_seconds: audio_duration,
        transcription_id,
        text: primary.text,
        words: primary.words,
        channels,
        entities: primary.entities,
        exported_formats: Vec::new(),
        saved: SavedFiles::default(),
        webhook_pending: false,
    }
}

struct PrimaryParsed {
    text: String,
    words: Vec<WordTiming>,
    language_code: Option<String>,
    language_probability: Option<f64>,
    entities: Vec<Entity>,
}

fn parse_words(val: Option<&serde_json::Value>) -> Vec<WordTiming> {
    let Some(arr) = val.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .map(|w| WordTiming {
            text: w
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            start: w.get("start").and_then(|v| v.as_f64()),
            end: w.get("end").and_then(|v| v.as_f64()),
            kind: w
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("word")
                .to_string(),
            speaker_id: w
                .get("speaker_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            channel_index: w.get("channel_index").and_then(|v| v.as_u64()),
            logprob: w.get("logprob").and_then(|v| v.as_f64()),
            characters: w.get("characters").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .map(|c| CharTiming {
                        text: c
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        start: c.get("start").and_then(|v| v.as_f64()),
                        end: c.get("end").and_then(|v| v.as_f64()),
                    })
                    .collect()
            }),
        })
        .collect()
}

fn clone_word(w: &WordTiming) -> WordTiming {
    WordTiming {
        text: w.text.clone(),
        start: w.start,
        end: w.end,
        kind: w.kind.clone(),
        speaker_id: w.speaker_id.clone(),
        channel_index: w.channel_index,
        logprob: w.logprob,
        characters: w.characters.as_ref().map(|cs| {
            cs.iter()
                .map(|c| CharTiming {
                    text: c.text.clone(),
                    start: c.start,
                    end: c.end,
                })
                .collect()
        }),
    }
}

fn parse_entities(val: Option<&serde_json::Value>) -> Vec<Entity> {
    let Some(arr) = val.and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|e| {
            Some(Entity {
                text: e.get("text")?.as_str()?.to_string(),
                entity_type: e.get("entity_type")?.as_str()?.to_string(),
                start_char: e.get("start_char")?.as_u64()?,
                end_char: e.get("end_char")?.as_u64()?,
            })
        })
        .collect()
}

fn format_diarized(words: &[WordTiming]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current_speaker: Option<String> = None;
    let mut current_text: Vec<String> = Vec::new();

    for w in words {
        if w.kind == "spacing" {
            continue;
        }
        let speaker = w.speaker_id.clone().unwrap_or_else(|| "speaker_0".into());
        if w.text.trim().is_empty() {
            continue;
        }
        match &current_speaker {
            Some(s) if *s == speaker => current_text.push(w.text.trim().to_string()),
            _ => {
                if let Some(prev) = &current_speaker {
                    lines.push(format!(
                        "{}: {}",
                        prev.to_uppercase().replace('_', " "),
                        current_text.join(" ")
                    ));
                    current_text.clear();
                }
                current_speaker = Some(speaker);
                current_text.push(w.text.trim().to_string());
            }
        }
    }
    if let Some(prev) = &current_speaker {
        lines.push(format!(
            "{}: {}",
            prev.to_uppercase().replace('_', " "),
            current_text.join(" ")
        ));
    }
    lines.join("\n\n")
}

// ── Exported formats → disk ────────────────────────────────────────────────

async fn write_exported_formats(
    args: &SttArgs,
    resp: &serde_json::Value,
    result: &mut SttResult,
) -> Result<(), AppError> {
    // Collect additional_formats from the top-level OR each channel (multi-channel).
    let mut collected: Vec<&serde_json::Value> = Vec::new();
    if let Some(arr) = resp.get("additional_formats").and_then(|v| v.as_array()) {
        collected.extend(arr.iter());
    }
    if let Some(transcripts) = resp.get("transcripts").and_then(|v| v.as_array()) {
        for t in transcripts {
            if let Some(arr) = t.get("additional_formats").and_then(|v| v.as_array()) {
                collected.extend(arr.iter());
            }
        }
    }
    if collected.is_empty() {
        return Ok(());
    }

    let dir = args
        .format_out_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(AppError::Io)?;
    }

    let basename = args
        .file
        .as_deref()
        .map(|p| {
            Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("transcript")
                .to_string()
        })
        .unwrap_or_else(|| format!("transcript_{}", crate::commands::now_timestamp()));

    for (i, fmt) in collected.iter().enumerate() {
        let requested = fmt
            .get("requested_format")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let ext = fmt
            .get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("txt")
            .to_string();
        let content_type = fmt
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();
        let content = fmt.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let is_b64 = fmt
            .get("is_base64_encoded")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let bytes: Vec<u8> = if is_b64 {
            base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|e| AppError::Http(format!("decode base64 {requested}: {e}")))?
        } else {
            content.as_bytes().to_vec()
        };

        // Use an index suffix if multiple entries of the same format exist (multi-channel).
        let suffix = if collected.len() > 1 {
            format!("_{i}")
        } else {
            String::new()
        };
        let ext = ext.trim_start_matches('.');
        let out_path = dir.join(format!("{basename}{suffix}.{ext}"));
        tokio::fs::write(&out_path, bytes)
            .await
            .map_err(AppError::Io)?;

        result.exported_formats.push(ExportedFormat {
            requested_format: requested,
            file_extension: ext.to_string(),
            content_type,
            saved_path: Some(out_path.display().to_string()),
        });
    }

    Ok(())
}

// ── Webhook-pending stub ───────────────────────────────────────────────────

async fn emit_webhook_pending(
    ctx: Ctx,
    args: &SttArgs,
    resp: &serde_json::Value,
) -> Result<(), AppError> {
    let result = SttResult {
        input: input_label(args),
        model: args.model.clone(),
        timestamps_granularity: args.timestamps.clone(),
        diarized: args.diarize,
        multi_channel: args.multi_channel,
        language_code: None,
        language_probability: None,
        audio_duration_seconds: None,
        transcription_id: resp
            .get("transcription_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        text: String::new(),
        words: Vec::new(),
        channels: Vec::new(),
        entities: Vec::new(),
        exported_formats: Vec::new(),
        saved: SavedFiles::default(),
        webhook_pending: true,
    };
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} webhook queued — transcription will be delivered asynchronously.",
            "~".yellow()
        );
        if let Some(id) = &r.transcription_id {
            println!("  transcription_id: {id}");
        }
    });
    Ok(())
}

// ── Human printing ─────────────────────────────────────────────────────────

fn print_human(r: &SttResult) {
    use owo_colors::OwoColorize;
    if let Some(p) = &r.saved.text {
        println!(
            "{} saved transcript -> {} ({} chars)",
            "+".green(),
            p.bold(),
            r.text.len()
        );
    }
    if let Some(p) = &r.saved.raw {
        println!("{} saved raw json   -> {}", "+".green(), p.bold());
    }
    if let Some(p) = &r.saved.words {
        println!(
            "{} saved words json -> {} ({} words)",
            "+".green(),
            p.bold(),
            r.words.len()
        );
    }
    for ef in &r.exported_formats {
        if let Some(p) = &ef.saved_path {
            println!(
                "{} saved {} -> {}",
                "+".green(),
                ef.requested_format.bold(),
                p
            );
        }
    }
    if let Some(lc) = &r.language_code {
        let p = r
            .language_probability
            .map(|p| format!(" (p={p:.2})"))
            .unwrap_or_default();
        println!("{} language: {lc}{p}", "i".dimmed());
    }
    if let Some(d) = r.audio_duration_seconds {
        println!("{} duration: {d:.2}s", "i".dimmed());
    }
    if !r.entities.is_empty() {
        println!("{} {} entities detected", "i".dimmed(), r.entities.len());
    }
    // Always show the transcript.
    println!("{}", r.text);
}
