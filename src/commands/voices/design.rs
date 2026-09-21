//! voices design — POST /v1/text-to-voice/design.
//!
//! Writes each preview's audio_base_64 payload to disk when present. When
//! `stream_previews=true` the server returns IDs only.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub struct DesignArgs {
    pub description: String,
    pub text: Option<String>,
    pub output_dir: Option<String>,
    pub model: Option<String>,
    pub loudness: Option<f32>,
    pub seed: Option<u32>,
    pub guidance_scale: Option<f32>,
    pub enhance: bool,
    pub stream_previews: bool,
    pub quality: Option<f32>,
}

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: DesignArgs) -> Result<(), AppError> {
    if args.description.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "description is required".into(),
            suggestion: None,
        });
    }
    if let Some(t) = &args.text {
        let len = t.chars().count();
        if !(100..=1000).contains(&len) {
            return Err(AppError::InvalidInput {
                msg: "--text must be 100 to 1000 characters".into(),
                suggestion: None,
            });
        }
    }

    let mut body = serde_json::Map::new();
    body.insert(
        "voice_description".into(),
        serde_json::Value::String(args.description.clone()),
    );
    match &args.text {
        Some(t) => {
            body.insert("text".into(), serde_json::Value::String(t.clone()));
            body.insert("auto_generate_text".into(), serde_json::Value::Bool(false));
        }
        None => {
            body.insert("auto_generate_text".into(), serde_json::Value::Bool(true));
        }
    }
    if let Some(m) = &args.model {
        body.insert("model_id".into(), serde_json::Value::String(m.clone()));
    }
    if let Some(l) = args.loudness {
        body.insert("loudness".into(), serde_json::json!(l));
    }
    if let Some(s) = args.seed {
        body.insert("seed".into(), serde_json::json!(s));
    }
    if let Some(g) = args.guidance_scale {
        body.insert("guidance_scale".into(), serde_json::json!(g));
    }
    if args.enhance {
        body.insert("should_enhance".into(), serde_json::Value::Bool(true));
    }
    if args.stream_previews {
        body.insert("stream_previews".into(), serde_json::Value::Bool(true));
    }
    if let Some(q) = args.quality {
        body.insert("quality".into(), serde_json::json!(q));
    }

    let resp: serde_json::Value = client
        .post_json("/v1/text-to-voice/design", &serde_json::Value::Object(body))
        .await?;

    let previews = resp
        .get("previews")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let dir = args.output_dir.unwrap_or_else(|| ".".to_string());
    let ts = crate::commands::now_timestamp();

    let mut written: Vec<serde_json::Value> = Vec::new();
    for (i, preview) in previews.iter().enumerate() {
        let Some(gen_id) = preview.get("generated_voice_id").and_then(|g| g.as_str()) else {
            continue;
        };
        let Some(b64) = preview.get("audio_base_64").and_then(|a| a.as_str()) else {
            // stream_previews mode — no bytes, just list the id.
            written.push(serde_json::json!({
                "generated_voice_id": gen_id,
                "file": null,
                "bytes": 0,
            }));
            continue;
        };
        let bytes = decode_base64(b64)?;
        let fname = format!("voice_design_{gen_id}_{ts}_{i}.mp3");
        let out_path = Path::new(&dir).join(&fname);
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(AppError::Io)?;
            }
        }
        tokio::fs::write(&out_path, &bytes)
            .await
            .map_err(AppError::Io)?;
        written.push(serde_json::json!({
            "generated_voice_id": gen_id,
            "file": out_path.display().to_string(),
            "bytes": bytes.len(),
        }));
    }

    let result = serde_json::json!({
        "description": args.description,
        "model": args.model,
        "previews": written,
    });

    output::print_success_or(ctx, &result, |v| {
        use owo_colors::OwoColorize;
        println!("{} generated {} previews:", "+".green(), written.len());
        for p in v["previews"].as_array().unwrap_or(&vec![]) {
            let file = p["file"].as_str().unwrap_or("(stream-only)");
            println!(
                "  {} {}",
                p["generated_voice_id"].as_str().unwrap_or("").dimmed(),
                file.bold()
            );
        }
        println!(
            "\nUse {} to save one to your library.",
            "elevenlabs voices save-preview <id> <name> <description>".bold()
        );
    })?;
    Ok(())
}

// ── tiny base64 decoder (no extra crate) ───────────────────────────────────

fn decode_base64(s: &str) -> Result<Vec<u8>, AppError> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (i, &b) in ALPHABET.iter().enumerate() {
        lookup[b as usize] = i as u8;
    }

    let filtered: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();

    let mut out = Vec::with_capacity(filtered.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for b in filtered {
        if b == b'=' {
            break;
        }
        let v = lookup[b as usize];
        if v == 255 {
            return Err(AppError::Http("invalid base64 in preview audio".into()));
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}
