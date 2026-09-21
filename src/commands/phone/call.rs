//! `phone call` — place an outbound call via an agent. Dispatches to the
//! correct provider endpoint based on the phone number's provider field.

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    from_id: String,
    to: String,
    dynamic_variables: Option<String>,
    client_data: Option<String>,
    record: bool,
    ringing_timeout_secs: Option<u32>,
) -> Result<(), AppError> {
    // Determine provider type by looking up the phone.
    let list: serde_json::Value = client.get_json("/v1/convai/phone-numbers").await?;
    let arr = list
        .as_array()
        .cloned()
        .or_else(|| {
            list.get("phone_numbers")
                .and_then(|p| p.as_array())
                .cloned()
        })
        .unwrap_or_default();
    let phone = arr.iter().find(|p| {
        p.get("phone_number_id")
            .and_then(|v| v.as_str())
            .map(|s| s == from_id)
            .unwrap_or(false)
    });
    let provider = phone
        .and_then(|p| p.get("provider"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_lowercase();

    let path = match provider.as_str() {
        "twilio" => "/v1/convai/twilio/outbound-call",
        "sip_trunk" => "/v1/convai/sip-trunk/outbound-call",
        "" => {
            return Err(AppError::InvalidInput {
                msg: format!("phone number {from_id} not found in your account"),
                suggestion: None,
            });
        }
        other => {
            return Err(AppError::InvalidInput {
                msg: format!("unsupported phone provider: {other}"),
                suggestion: None,
            });
        }
    };

    // Build conversation_initiation_client_data. Precedence: --client-data is
    // the base object; --dynamic-variables is DEEP-MERGED into any existing
    // dynamic_variables map (overriding same-named keys, preserving others)
    // rather than replacing the map wholesale. Either flag alone works; both
    // together work.
    let mut ci_data: Option<serde_json::Value> = match client_data.as_deref() {
        Some(raw) => Some(parse_json_object("--client-data", raw).await?),
        None => None,
    };
    if let Some(raw) = dynamic_variables.as_deref() {
        let vars = parse_json_object("--dynamic-variables", raw).await?;
        let base = ci_data.get_or_insert_with(|| serde_json::json!({}));
        let map = base.as_object_mut().expect("base is a JSON object");
        let existing = map
            .remove("dynamic_variables")
            .unwrap_or_else(|| serde_json::json!({}));
        let merged = merge_dynamic_variables(existing, vars);
        map.insert("dynamic_variables".to_string(), merged);
    }

    let mut body = serde_json::json!({
        "agent_id": agent_id,
        "agent_phone_number_id": from_id,
        "to_number": to,
    });
    if let Some(cd) = ci_data {
        body["conversation_initiation_client_data"] = cd;
    }
    if record {
        // call_recording_enabled is only defined on the Twilio outbound-call
        // body in the OpenAPI spec; SIP-trunk rejects the extra field.
        if provider != "twilio" {
            return Err(AppError::bad_input_with(
                format!(
                    "--record is only supported on Twilio numbers (this number uses provider \
                     '{provider}')"
                ),
                "Drop --record for SIP-trunk calls, or record via your own telephony stack.",
            ));
        }
        body["call_recording_enabled"] = serde_json::Value::Bool(true);
    }
    if let Some(secs) = ringing_timeout_secs {
        body["telephony_call_config"] = serde_json::json!({ "ringing_timeout_secs": secs });
    }

    let resp: serde_json::Value = client.post_json(path, &body).await?;
    let result = serde_json::json!({
        "provider": provider,
        "agent_id": agent_id,
        "from_phone_number_id": from_id,
        "to": to,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} call placed via {} to {}",
            "+".green(),
            r["provider"].as_str().unwrap_or("").bold(),
            r["to"].as_str().unwrap_or("").bold()
        );
    })?;
    Ok(())
}

/// Parse a `<JSON>` or `<@file>` flag into a JSON object. The caller gets
/// back an `AppError::InvalidInput` with a concrete, flag-specific
/// suggestion for the common failure modes (file missing, non-object
/// top-level, broken JSON).
async fn parse_json_object(flag: &str, raw: &str) -> Result<serde_json::Value, AppError> {
    let text = if let Some(rest) = raw.strip_prefix('@') {
        let path = Path::new(rest);
        if !path.exists() {
            return Err(AppError::bad_input_with(
                format!("{flag} file does not exist: {}", path.display()),
                format!(
                    "Either drop the leading `@` to pass a literal JSON string, or point at an \
                     existing file: {flag} @{}",
                    path.display()
                ),
            ));
        }
        tokio::fs::read_to_string(path)
            .await
            .map_err(AppError::Io)?
    } else {
        raw.to_string()
    };

    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        AppError::bad_input_with(
            format!("{flag} is not valid JSON: {e}"),
            format!(
                "Pass a JSON object like {flag} '{{\"name\":\"Alex\"}}'. Use @file.json to load \
                 from disk if the JSON is large or contains special shell characters."
            ),
        )
    })?;
    if !val.is_object() {
        return Err(AppError::bad_input_with(
            format!("{flag} must be a JSON object (got {})", val_kind(&val)),
            format!("The top level must be an object: {flag} '{{\"key\":\"value\"}}'"),
        ));
    }
    Ok(val)
}

/// Merge `--dynamic-variables` into an existing `dynamic_variables` map.
/// Object-merges at one level (overriding keys from `incoming` into
/// `existing`). If either side isn't an object, `incoming` wins — the CLI
/// only accepts object-shaped JSON upstream via `parse_json_object`, but
/// this stays defensive against upstream shape changes.
fn merge_dynamic_variables(
    existing: serde_json::Value,
    incoming: serde_json::Value,
) -> serde_json::Value {
    match (existing, incoming) {
        (serde_json::Value::Object(mut base), serde_json::Value::Object(over)) => {
            for (k, v) in over {
                base.insert(k, v);
            }
            serde_json::Value::Object(base)
        }
        (_, v) => v,
    }
}

fn val_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
