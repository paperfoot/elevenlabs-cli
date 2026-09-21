//! music detailed — POST /v1/music/detailed
//!
//! Returns a `multipart/mixed` streaming response with two parts:
//!   1. JSON metadata: bpm, time_signature, key, sections (intro /
//!      verse / chorus boundaries), genre, mood, etc.
//!   2. Binary audio: the generated track in the requested output
//!      format.
//!
//! Both Python and JS SDKs model this as a streaming multipart response
//! (see `v1/music/detailed` in elevenlabs-python/src/elevenlabs/music/
//! raw_client.py). We parse the boundary from the Content-Type header,
//! split the body into parts, and write:
//!   - the audio part to `--output` (default `music_<ts>.<ext>`)
//!   - the metadata JSON to `--save-metadata` (default
//!     `<output>.metadata.json`)
//!
//! Historical note: pre-v0.2 this file treated the response as JSON
//! with `audio_base64`. The SDKs never modelled it that way. This
//! module now matches the real contract.
//!
//! The multipart parser is hand-rolled to avoid dragging in a dep for a
//! single endpoint. It walks `--<boundary>` separators, reads each
//! part's headers (terminated by `\r\n\r\n`), and peels off the body
//! up to the next boundary. `--<boundary>--` marks end-of-stream.

use reqwest::header::CONTENT_TYPE;

use crate::cli::DetailedArgs;
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    cfg: &crate::config::AppConfig,
    client: &ElevenLabsClient,
    args: DetailedArgs,
) -> Result<(), AppError> {
    let body = super::build_compose_body(
        args.prompt.as_deref(),
        args.length_ms,
        args.composition_plan.as_deref(),
        args.model.as_deref(),
        args.force_instrumental,
        args.seed,
        args.respect_sections_durations,
        args.store_for_inpainting,
        args.sign_with_c2pa,
    )
    .await?;

    let output_format = args.format.unwrap_or_else(|| cfg.default_output_format());
    let query = [("output_format", output_format.as_str())];

    // Explicit `Accept: multipart/mixed` so the server picks the right
    // response mode even via a proxy that strips unknown accept types.
    let resp = client
        .http
        .post(client.url("/v1/music/detailed"))
        .query(&query)
        .header(reqwest::header::ACCEPT, "multipart/mixed")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Surface non-2xx responses via the standard error channel. Bound
    // the snippet so we don't spew a 1MB HTML payload into stderr.
    if !status.is_success() {
        let code = status.as_u16();
        let text = resp.text().await.unwrap_or_default();
        let snippet = if text.is_empty() {
            format!("HTTP {code}")
        } else {
            text.chars().take(300).collect::<String>()
        };
        return Err(match code {
            401 | 403 => AppError::AuthFailed(snippet),
            429 => AppError::RateLimited(snippet),
            _ => AppError::Api {
                status: code,
                message: snippet,
            },
        });
    }

    let full = resp.bytes().await?.to_vec();

    let boundary = extract_boundary(&content_type).ok_or_else(|| {
        AppError::Http(format!(
            "music/detailed response missing multipart boundary; content-type={content_type}"
        ))
    })?;
    let parts = split_multipart(&full, &boundary)?;
    if parts.len() < 2 {
        return Err(AppError::Http(format!(
            "music/detailed returned {} multipart part(s), expected >= 2 (JSON metadata + audio)",
            parts.len()
        )));
    }

    // Classify parts by Content-Type. Contract is JSON first, audio
    // second, but we match on header so ordering can flip safely.
    let mut metadata_bytes: Option<&[u8]> = None;
    let mut audio_bytes: Option<&[u8]> = None;
    for part in &parts {
        let ct = part
            .content_type
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        if ct.contains("json") && metadata_bytes.is_none() {
            metadata_bytes = Some(&part.body);
        } else if (ct.starts_with("audio/") || ct.contains("mpeg") || ct.contains("octet-stream"))
            && audio_bytes.is_none()
        {
            audio_bytes = Some(&part.body);
        }
    }
    // Fallback to positional order if headers don't let us classify
    // (some servers ship parts without a Content-Type header).
    if metadata_bytes.is_none() {
        metadata_bytes = Some(&parts[0].body);
    }
    if audio_bytes.is_none() {
        audio_bytes = Some(&parts[parts.len() - 1].body);
    }
    let audio = audio_bytes.unwrap();
    let metadata_raw = metadata_bytes.unwrap();

    let ext = crate::commands::tts::extension_for_format(&output_format);
    let out_path = crate::commands::resolve_output_path(args.output, "music", ext);
    tokio::fs::write(&out_path, audio)
        .await
        .map_err(AppError::Io)?;

    let metadata_path = args
        .save_metadata
        .unwrap_or_else(|| format!("{}.metadata.json", out_path.display()));

    // Pretty-print whatever JSON the server sent so the companion file
    // is grep-friendly. If we can't parse it, persist verbatim — never
    // lose the bytes.
    let metadata_out: Vec<u8> = match serde_json::from_slice::<serde_json::Value>(metadata_raw) {
        Ok(v) => serde_json::to_vec_pretty(&v)
            .map_err(|e| AppError::Http(format!("serialize metadata: {e}")))?,
        Err(_) => metadata_raw.to_vec(),
    };
    tokio::fs::write(&metadata_path, &metadata_out)
        .await
        .map_err(AppError::Io)?;

    let result = serde_json::json!({
        "prompt": args.prompt,
        "composition_plan_file": args.composition_plan,
        "length_ms": args.length_ms,
        "seed": args.seed,
        "force_instrumental": args.force_instrumental,
        "output": out_path.display().to_string(),
        "metadata_path": metadata_path,
        "output_format": output_format,
        "bytes_written": audio.len(),
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} {} ({:.1} KB)",
            "+".green(),
            r["output"].as_str().unwrap_or("").bold(),
            r["bytes_written"].as_f64().unwrap_or(0.0) / 1024.0,
        );
        println!("  metadata: {}", r["metadata_path"].as_str().unwrap_or(""));
    })?;
    Ok(())
}

// ── multipart/mixed parsing ────────────────────────────────────────────────
//
// RFC 2046-compatible subset. We only need to handle what ElevenLabs
// actually sends: a handful of parts, each with a `Content-Type` header,
// separated by `--<boundary>\r\n` and terminated by `--<boundary>--`.
// The body between boundaries is raw bytes.

struct Part {
    content_type: Option<String>,
    body: Vec<u8>,
}

/// Pull the `boundary=<value>` token out of a Content-Type header.
/// Handles both quoted and unquoted forms. Boundary may contain the
/// full RFC-allowed charset, so we stop at `;` / whitespace / EOL.
fn extract_boundary(content_type: &str) -> Option<String> {
    let lower = content_type.to_ascii_lowercase();
    let idx = lower.find("boundary=")?;
    let rest = &content_type[idx + "boundary=".len()..];
    let trimmed = rest.trim_start();
    let raw: &str = if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"').unwrap_or(stripped.len());
        &stripped[..end]
    } else {
        let end = trimmed
            .find(|c: char| c == ';' || c.is_whitespace())
            .unwrap_or(trimmed.len());
        &trimmed[..end]
    };
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

/// Split a multipart/mixed body into parts. Each returned `Part` has
/// the raw body bytes (headers stripped) and the parsed Content-Type if
/// present.
fn split_multipart(body: &[u8], boundary: &str) -> Result<Vec<Part>, AppError> {
    let dash_boundary = format!("--{boundary}");
    let delim = dash_boundary.as_bytes();

    let mut positions: Vec<usize> = Vec::new();
    let mut i = 0;
    while i + delim.len() <= body.len() {
        if &body[i..i + delim.len()] == delim {
            positions.push(i);
            i += delim.len();
        } else {
            i += 1;
        }
    }
    if positions.len() < 2 {
        return Err(AppError::Http(format!(
            "multipart body missing delimiters (found {})",
            positions.len()
        )));
    }

    let mut parts = Vec::new();
    for window in positions.windows(2) {
        let start = window[0];
        let end = window[1];
        let after_delim = start + delim.len();
        // `--<boundary>--` marks end-of-stream: everything after is the
        // epilogue and must be ignored.
        if after_delim + 2 <= body.len() && &body[after_delim..after_delim + 2] == b"--" {
            break;
        }
        let headers_start = skip_crlf(body, after_delim);
        if headers_start >= end {
            continue;
        }
        let Some(sep_off) = find(&body[headers_start..end], b"\r\n\r\n") else {
            continue;
        };
        let headers_bytes = &body[headers_start..headers_start + sep_off];
        let body_start = headers_start + sep_off + 4;
        // Strip the CRLF that precedes the next boundary — per RFC the
        // boundary itself lives on its own line.
        let body_end = end.saturating_sub(2);
        if body_end < body_start {
            continue;
        }
        let content_type = parse_content_type(headers_bytes);
        parts.push(Part {
            content_type,
            body: body[body_start..body_end].to_vec(),
        });
    }
    Ok(parts)
}

fn skip_crlf(body: &[u8], mut pos: usize) -> usize {
    // Tolerant of bare `\n` for servers that mis-encode.
    if pos + 2 <= body.len() && &body[pos..pos + 2] == b"\r\n" {
        pos += 2;
    } else if pos < body.len() && body[pos] == b'\n' {
        pos += 1;
    }
    pos
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let end = haystack.len() - needle.len();
    let mut i = 0;
    while i <= end {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract the `Content-Type` header value from a header block.
/// Case-insensitive on the header name; value is trimmed of surrounding
/// whitespace. Returns `None` if the header isn't present.
fn parse_content_type(headers: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(headers).ok()?;
    for line in text.split("\r\n") {
        let mut split = line.splitn(2, ':');
        let name = split.next()?.trim();
        let value = split.next()?.trim();
        if name.eq_ignore_ascii_case("content-type") {
            return Some(value.to_string());
        }
    }
    None
}
