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
            show_legacy,
        } => list(ctx, &client, search, sort, direction, limit, show_legacy).await,
        VoicesAction::Show { voice_id } => show(ctx, &client, &voice_id).await,
        VoicesAction::Search { query } => {
            list(
                ctx,
                &client,
                Some(query),
                "name".into(),
                "asc".into(),
                50,
                false,
            )
            .await
        }
        VoicesAction::Library {
            search,
            page,
            page_size,
            category,
            gender,
            age,
            accent,
            language,
            locale,
            use_case,
            featured,
            min_notice_days,
            include_custom_rates,
            include_live_moderated,
            reader_app_enabled,
            owner_id,
            sort,
        } => {
            library(
                ctx,
                &client,
                LibraryArgs {
                    search,
                    page,
                    page_size,
                    category,
                    gender,
                    age,
                    accent,
                    language,
                    locale,
                    use_case,
                    featured,
                    min_notice_days,
                    include_custom_rates,
                    include_live_moderated,
                    reader_app_enabled,
                    owner_id,
                    sort,
                },
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
            model,
            loudness,
            seed,
            guidance_scale,
            enhance,
            stream_previews,
            quality,
        } => {
            design(
                ctx,
                &client,
                DesignArgs {
                    description,
                    text,
                    output_dir,
                    model,
                    loudness,
                    seed,
                    guidance_scale,
                    enhance,
                    stream_previews,
                    quality,
                },
            )
            .await
        }
        VoicesAction::SavePreview {
            generated_voice_id,
            name,
            description,
        } => save_preview(ctx, &client, generated_voice_id, name, description).await,
        VoicesAction::Delete { voice_id, yes } => delete(ctx, &client, &voice_id, yes).await,
    }
}

struct LibraryArgs {
    search: Option<String>,
    page: u32,
    page_size: u32,
    category: Option<String>,
    gender: Option<String>,
    age: Option<String>,
    accent: Option<String>,
    language: Option<String>,
    locale: Option<String>,
    use_case: Option<String>,
    featured: bool,
    min_notice_days: Option<u32>,
    include_custom_rates: bool,
    include_live_moderated: bool,
    reader_app_enabled: bool,
    owner_id: Option<String>,
    sort: Option<String>,
}

struct DesignArgs {
    description: String,
    text: Option<String>,
    output_dir: Option<String>,
    model: Option<String>,
    loudness: Option<f32>,
    seed: Option<u32>,
    guidance_scale: Option<f32>,
    enhance: bool,
    stream_previews: bool,
    quality: Option<f32>,
}

// ── list ───────────────────────────────────────────────────────────────────

async fn list(
    ctx: Ctx,
    client: &ElevenLabsClient,
    search: Option<String>,
    sort: String,
    direction: String,
    limit: u32,
    show_legacy: bool,
) -> Result<(), AppError> {
    let mut params: Vec<(&str, String)> = vec![
        ("sort", sort),
        ("sort_direction", direction),
        ("page_size", limit.to_string()),
    ];
    if let Some(s) = search {
        params.push(("search", s));
    }
    if show_legacy {
        params.push(("show_legacy", "true".to_string()));
    }

    // Use /v2/voices — the only endpoint that honours search/sort/page_size.
    // /v1/voices ignores those params and returns the full library, which
    // is what bit the voice-name resolver in v0.1.0.
    let resp: serde_json::Value = client.get_json_with_query("/v2/voices", &params).await?;
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

async fn library(ctx: Ctx, client: &ElevenLabsClient, args: LibraryArgs) -> Result<(), AppError> {
    // The API is 1-indexed on page. v0.1.3 and earlier sent 0-indexed which
    // caused a silent off-by-one on this endpoint.
    let mut params: Vec<(&str, String)> = vec![
        ("page", args.page.to_string()),
        ("page_size", args.page_size.to_string()),
    ];
    if let Some(s) = args.search {
        params.push(("search", s));
    }
    if let Some(v) = args.category {
        params.push(("category", v));
    }
    if let Some(v) = args.gender {
        params.push(("gender", v));
    }
    if let Some(v) = args.age {
        params.push(("age", v));
    }
    if let Some(v) = args.accent {
        params.push(("accent", v));
    }
    if let Some(v) = args.language {
        params.push(("language", v));
    }
    if let Some(v) = args.locale {
        params.push(("locale", v));
    }
    if let Some(v) = args.use_case {
        params.push(("use_cases", v));
    }
    if args.featured {
        params.push(("featured", "true".to_string()));
    }
    if let Some(v) = args.min_notice_days {
        params.push(("min_notice_period_days", v.to_string()));
    }
    if args.include_custom_rates {
        params.push(("include_custom_rates", "true".to_string()));
    }
    if args.include_live_moderated {
        params.push(("include_live_moderated", "true".to_string()));
    }
    if args.reader_app_enabled {
        params.push(("reader_app_enabled", "true".to_string()));
    }
    if let Some(v) = args.owner_id {
        params.push(("owner_id", v));
    }
    if let Some(v) = args.sort {
        params.push(("sort", v));
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

async fn design(ctx: Ctx, client: &ElevenLabsClient, args: DesignArgs) -> Result<(), AppError> {
    if args.description.trim().is_empty() {
        return Err(AppError::InvalidInput("description is required".into()));
    }
    if let Some(t) = &args.text {
        let len = t.chars().count();
        if !(100..=1000).contains(&len) {
            return Err(AppError::InvalidInput(
                "--text must be 100 to 1000 characters".into(),
            ));
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
        .post_json(
            "/v1/text-to-voice/create-previews",
            &serde_json::Value::Object(body),
        )
        .await?;

    // When stream_previews=true, the API returns only IDs (no audio bytes).
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
    // Correct path per the official Python SDK:
    //   elevenlabs-python/src/elevenlabs/text_to_voice/raw_client.py
    //   line 167 — "v1/text-to-voice", method POST.
    // v0.1.1 used "/v1/text-to-voice/create-voice-from-preview" which 404s.
    let resp: serde_json::Value = client.post_json("/v1/text-to-voice", &body).await?;
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

async fn delete(
    ctx: Ctx,
    client: &ElevenLabsClient,
    voice_id: &str,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        return Err(AppError::InvalidInput(format!(
            "deleting '{voice_id}' is irreversible — pass --yes to confirm"
        )));
    }
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
