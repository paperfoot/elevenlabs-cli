//! voices: list / show / search / library / clone / design / save-preview / delete

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::cli::VoicesAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct VoiceSummary {
    voice_id: String,
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    preview_url: Option<String>,
}

pub async fn dispatch(ctx: Ctx, action: VoicesAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        VoicesAction::List {
            search,
            sort,
            direction,
            limit,
        } => list(ctx, &client, search, sort, direction, limit).await,
        VoicesAction::Show { voice_id } => show(ctx, &client, &voice_id).await,
        VoicesAction::Search { query } => {
            list(ctx, &client, Some(query), "name".into(), "asc".into(), 50).await
        }
        VoicesAction::Library {
            search,
            page,
            page_size,
            gender,
            age,
            accent,
            language,
            use_case,
        } => {
            library(
                ctx, &client, search, page, page_size, gender, age, accent, language, use_case,
            )
            .await
        }
        VoicesAction::Clone {
            name,
            files,
            description,
        } => clone(ctx, &client, name, files, description).await,
        VoicesAction::Design {
            description,
            text,
            output_dir,
        } => design(ctx, &client, description, text, output_dir).await,
        VoicesAction::SavePreview {
            generated_voice_id,
            name,
            description,
        } => save_preview(ctx, &client, generated_voice_id, name, description).await,
        VoicesAction::Delete { voice_id } => delete(ctx, &client, &voice_id).await,
    }
}

// ── list ───────────────────────────────────────────────────────────────────

async fn list(
    ctx: Ctx,
    client: &ElevenLabsClient,
    search: Option<String>,
    sort: String,
    direction: String,
    limit: u32,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = vec![
        ("sort", sort),
        ("sort_direction", direction),
        ("page_size", limit.to_string()),
    ];
    if let Some(s) = search {
        params.push(("search", s));
    }

    let resp: serde_json::Value = client.get_json_with_query("/v1/voices", &params).await?;
    let voices: Vec<VoiceSummary> = resp
        .get("voices")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    output::print_success_or(ctx, &voices, |list| {
        use owo_colors::OwoColorize;
        if list.is_empty() {
            println!("(no voices)");
            return;
        }
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Voice ID", "Name", "Category"]);
        for v in list {
            t.add_row(vec![
                v.voice_id.dimmed().to_string(),
                v.name.bold().to_string(),
                v.category.clone().unwrap_or_default(),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}

// ── show ───────────────────────────────────────────────────────────────────

async fn show(ctx: Ctx, client: &ElevenLabsClient, voice_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/voices/{voice_id}");
    let resp: serde_json::Value = client.get_json(&path).await?;
    output::print_success_or(ctx, &resp, |v| {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    });
    Ok(())
}

// ── library (shared voices) ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn library(
    ctx: Ctx,
    client: &ElevenLabsClient,
    search: Option<String>,
    page: u32,
    page_size: u32,
    gender: Option<String>,
    age: Option<String>,
    accent: Option<String>,
    language: Option<String>,
    use_case: Option<String>,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = vec![
        ("page", page.to_string()),
        ("page_size", page_size.to_string()),
    ];
    if let Some(s) = search {
        params.push(("search", s));
    }
    if let Some(v) = gender {
        params.push(("gender", v));
    }
    if let Some(v) = age {
        params.push(("age", v));
    }
    if let Some(v) = accent {
        params.push(("accent", v));
    }
    if let Some(v) = language {
        params.push(("language", v));
    }
    if let Some(v) = use_case {
        params.push(("use_cases", v));
    }

    let resp: serde_json::Value = client
        .get_json_with_query("/v1/shared-voices", &params)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        let list = v
            .get("voices")
            .and_then(|vs| vs.as_array())
            .cloned()
            .unwrap_or_default();
        if list.is_empty() {
            println!("(no shared voices match)");
            return;
        }
        use owo_colors::OwoColorize;
        let mut t = comfy_table::Table::new();
        t.set_header(vec![
            "Voice ID", "Name", "Gender", "Age", "Accent", "Use Case",
        ]);
        for voice in &list {
            t.add_row(vec![
                voice
                    .get("voice_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .dimmed()
                    .to_string(),
                voice
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .bold()
                    .to_string(),
                voice
                    .get("gender")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("age")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("accent")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
                voice
                    .get("use_case")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .into(),
            ]);
        }
        println!("{t}");
    });
    Ok(())
}

// ── clone (IVC) ────────────────────────────────────────────────────────────

async fn clone(
    ctx: Ctx,
    client: &ElevenLabsClient,
    name: String,
    files: Vec<String>,
    description: Option<String>,
) -> Result<(), AppError> {
    if files.is_empty() {
        return Err(AppError::InvalidInput(
            "at least one sample file required".into(),
        ));
    }
    let mut form = reqwest::multipart::Form::new().text("name", name.clone());
    if let Some(d) = description.clone() {
        form = form.text("description", d);
    }

    for f in &files {
        let path = Path::new(f);
        if !path.exists() {
            return Err(AppError::InvalidInput(format!(
                "file does not exist: {}",
                path.display()
            )));
        }
        let bytes = crate::commands::read_file_bytes(path).await?;
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "sample.mp3".to_string());
        let mime = crate::commands::mime_for_path(path);
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|e| AppError::Http(format!("invalid mime '{mime}': {e}")))?;
        form = form.part("files", part);
    }

    let resp: serde_json::Value = client.post_multipart_json("/v1/voices/add", form).await?;

    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} cloned voice: {} ({})",
            "+".green(),
            v.get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&name)
                .bold(),
            v.get("voice_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    });
    Ok(())
}

// ── design (voice generation previews) ─────────────────────────────────────

async fn design(
    ctx: Ctx,
    client: &ElevenLabsClient,
    description: String,
    text: Option<String>,
    output_dir: Option<String>,
) -> Result<(), AppError> {
    if description.trim().is_empty() {
        return Err(AppError::InvalidInput("description is required".into()));
    }

    let mut body = serde_json::Map::new();
    body.insert(
        "voice_description".into(),
        serde_json::Value::String(description.clone()),
    );
    match &text {
        Some(t) => {
            body.insert("text".into(), serde_json::Value::String(t.clone()));
            body.insert("auto_generate_text".into(), serde_json::Value::Bool(false));
        }
        None => {
            body.insert("auto_generate_text".into(), serde_json::Value::Bool(true));
        }
    }

    let resp: serde_json::Value = client
        .post_json(
            "/v1/text-to-voice/create-previews",
            &serde_json::Value::Object(body),
        )
        .await?;

    let previews = resp
        .get("previews")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let dir = output_dir.unwrap_or_else(|| ".".to_string());
    let ts = crate::commands::now_timestamp();

    let mut written: Vec<serde_json::Value> = Vec::new();
    for (i, preview) in previews.iter().enumerate() {
        let Some(gen_id) = preview.get("generated_voice_id").and_then(|g| g.as_str()) else {
            continue;
        };
        let Some(b64) = preview.get("audio_base_64").and_then(|a| a.as_str()) else {
            continue;
        };
        let bytes = decode_base64(b64)?;
        let fname = format!("voice_design_{gen_id}_{ts}_{i}.mp3");
        let out_path = Path::new(&dir).join(&fname);
        if let Some(parent) = out_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
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
        "description": description,
        "previews": written,
    });

    output::print_success_or(ctx, &result, |v| {
        use owo_colors::OwoColorize;
        println!("{} generated {} previews:", "+".green(), written.len());
        for p in v["previews"].as_array().unwrap_or(&vec![]) {
            println!(
                "  {} {}",
                p["generated_voice_id"].as_str().unwrap_or("").dimmed(),
                p["file"].as_str().unwrap_or("").bold()
            );
        }
        println!(
            "\nUse {} to save one to your library.",
            "elevenlabs voices save-preview <id> <name> <description>".bold()
        );
    });
    Ok(())
}

async fn save_preview(
    ctx: Ctx,
    client: &ElevenLabsClient,
    generated_voice_id: String,
    voice_name: String,
    voice_description: String,
) -> Result<(), AppError> {
    let body = serde_json::json!({
        "generated_voice_id": generated_voice_id,
        "voice_name": voice_name,
        "voice_description": voice_description,
    });
    let resp: serde_json::Value = client
        .post_json("/v1/text-to-voice/create-voice-from-preview", &body)
        .await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} saved voice: {} ({})",
            "+".green(),
            v.get("name").and_then(|x| x.as_str()).unwrap_or("").bold(),
            v.get("voice_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    });
    Ok(())
}

// ── delete ─────────────────────────────────────────────────────────────────

async fn delete(ctx: Ctx, client: &ElevenLabsClient, voice_id: &str) -> Result<(), AppError> {
    let path = format!("/v1/voices/{voice_id}");
    client.delete(&path).await?;
    let result = serde_json::json!({ "voice_id": voice_id, "deleted": true });
    output::print_success_or(ctx, &result, |_| {
        use owo_colors::OwoColorize;
        println!("{} deleted voice {}", "-".red(), voice_id.dimmed());
    });
    Ok(())
}

// ── tiny base64 decoder (no extra crate) ───────────────────────────────────

fn decode_base64(s: &str) -> Result<Vec<u8>, AppError> {
    // We avoid pulling the `base64` crate for one function; use a minimal
    // standard-alphabet decoder. Accepts optional padding.
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
