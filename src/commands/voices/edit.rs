//! voices edit — POST /v1/voices/{voice_id}/edit (multipart).
//!
//! Grounded against elevenlabs-python voices/raw_client.py `update`:
//!   - name (optional str)
//!   - description (optional str)
//!   - labels (optional JSON-stringified object of string -> string)
//!   - files (optional repeated file parts — added samples)
//!   - remove_background_noise (optional bool)
//!
//! Removing samples is a separate endpoint (DELETE
//! /v1/voices/{voice_id}/samples/{sample_id}). We fan out per --remove-sample
//! and batch the add/rename side via the edit endpoint.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub struct EditArgs {
    pub voice_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub labels: Vec<String>,
    pub add_sample: Vec<String>,
    pub remove_sample: Vec<String>,
    pub remove_background_noise: bool,
}

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, args: EditArgs) -> Result<(), AppError> {
    if args.voice_id.trim().is_empty() {
        return Err(AppError::InvalidInput {
            msg: "voice_id is required".into(),
            suggestion: None,
        });
    }

    let nothing_to_do = args.name.is_none()
        && args.description.is_none()
        && args.labels.is_empty()
        && args.add_sample.is_empty()
        && args.remove_sample.is_empty()
        && !args.remove_background_noise;
    if nothing_to_do {
        return Err(AppError::InvalidInput {
            msg: "nothing to edit. Pass at least one of: --name, --description, \
             --labels, --add-sample, --remove-sample, --remove-background-noise"
                .into(),
            suggestion: None,
        });
    }

    // 1. Remove samples first. A rename+remove in one call is ambiguous on
    //    the server, so we always fan out removals via the dedicated
    //    endpoint.
    for sample_id in &args.remove_sample {
        let path = format!("/v1/voices/{}/samples/{}", args.voice_id, sample_id);
        client.delete(&path).await?;
    }

    // 2. Compose the edit multipart form (only if there's something non-sample
    //    to change, or samples to add).
    let needs_edit_post = args.name.is_some()
        || args.description.is_some()
        || !args.labels.is_empty()
        || !args.add_sample.is_empty()
        || args.remove_background_noise;

    let final_voice: serde_json::Value = if needs_edit_post {
        let mut form = reqwest::multipart::Form::new();

        if let Some(n) = &args.name {
            form = form.text("name", n.clone());
        }
        if let Some(d) = &args.description {
            form = form.text("description", d.clone());
        }

        // labels -> JSON object, serialized as a single form field per SDK.
        if !args.labels.is_empty() {
            let labels_obj = parse_labels(&args.labels)?;
            let labels_json = serde_json::to_string(&labels_obj)
                .map_err(|e| AppError::Http(format!("serialize labels: {e}")))?;
            form = form.text("labels", labels_json);
        }

        if args.remove_background_noise {
            form = form.text("remove_background_noise", "true");
        }

        for f in &args.add_sample {
            let path = Path::new(f);
            if !path.exists() {
                return Err(AppError::InvalidInput {
                    msg: format!("sample file does not exist: {}", path.display()),
                    suggestion: None,
                });
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

        let path = format!("/v1/voices/{}/edit", args.voice_id);
        client.post_multipart_json(&path, form).await?
    } else {
        serde_json::json!({ "voice_id": args.voice_id, "status": "ok" })
    };

    let result = serde_json::json!({
        "voice_id": args.voice_id,
        "edited": final_voice,
        "removed_samples": args.remove_sample,
    });

    output::print_success_or(ctx, &result, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} edited voice {}",
            "+".green(),
            v["voice_id"].as_str().unwrap_or("").dimmed()
        );
        if let Some(removed) = v["removed_samples"].as_array() {
            if !removed.is_empty() {
                println!(
                    "  removed samples: {}",
                    removed
                        .iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    })?;
    Ok(())
}

/// Parse `--labels key=value` strings into a flat string→string map.
/// Duplicate keys keep the last occurrence, matching the server contract
/// (labels are a dict, not a multi-map).
fn parse_labels(pairs: &[String]) -> Result<serde_json::Map<String, serde_json::Value>, AppError> {
    let mut out = serde_json::Map::new();
    for pair in pairs {
        let (k, v) = pair.split_once('=').ok_or_else(|| AppError::InvalidInput {
            msg: format!("--labels must be 'key=value' (got '{pair}')"),
            suggestion: None,
        })?;
        let k = k.trim();
        if k.is_empty() {
            return Err(AppError::InvalidInput {
                msg: format!("--labels key cannot be empty (got '{pair}')"),
                suggestion: None,
            });
        }
        out.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    Ok(out)
}
