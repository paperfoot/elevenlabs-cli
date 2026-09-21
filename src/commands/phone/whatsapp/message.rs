//! `phone whatsapp message` — POST /v1/convai/whatsapp/outbound-message
//!
//! Body shape (matches elevenlabs-python raw_client exactly):
//!
//!   {
//!     "whatsapp_phone_number_id": "<sending business number id>",
//!     "whatsapp_user_id": "<recipient user id>",
//!     "template_name": "<name>",
//!     "template_language_code": "<e.g. en_US>",
//!     "template_params": [
//!       { "type": "body",
//!         "parameters": [ { "parameter_name": "<k>", "type": "text", "text": "<v>" }, ... ]
//!       }
//!     ],
//!     "agent_id": "<agent id>",
//!     "conversation_initiation_client_data"?: { ... pass-through ... }
//!   }

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    whatsapp_phone_number_id: String,
    whatsapp_user_id: String,
    template_name: String,
    template_language_code: String,
    template_params_kv: Vec<String>,
    client_data_path: Option<String>,
) -> Result<(), AppError> {
    let parameters = parse_template_params(&template_params_kv)?;
    let template_params_payload = if parameters.is_empty() {
        serde_json::Value::Array(Vec::new())
    } else {
        serde_json::json!([
            {
                "type": "body",
                "parameters": parameters,
            }
        ])
    };

    let mut body = serde_json::Map::new();
    body.insert(
        "whatsapp_phone_number_id".into(),
        serde_json::Value::String(whatsapp_phone_number_id.clone()),
    );
    body.insert(
        "whatsapp_user_id".into(),
        serde_json::Value::String(whatsapp_user_id.clone()),
    );
    body.insert(
        "template_name".into(),
        serde_json::Value::String(template_name),
    );
    body.insert(
        "template_language_code".into(),
        serde_json::Value::String(template_language_code),
    );
    body.insert("template_params".into(), template_params_payload);
    body.insert("agent_id".into(), serde_json::Value::String(agent_id));

    if let Some(path_str) = client_data_path {
        let client_data = load_client_data(&path_str).await?;
        body.insert("conversation_initiation_client_data".into(), client_data);
    }

    let resp: serde_json::Value = client
        .post_json(
            "/v1/convai/whatsapp/outbound-message",
            &serde_json::Value::Object(body),
        )
        .await?;
    let result = serde_json::json!({
        "whatsapp_phone_number_id": whatsapp_phone_number_id,
        "whatsapp_user_id": whatsapp_user_id,
        "response": resp,
    });
    output::print_success_or(ctx, &result, |r| {
        use owo_colors::OwoColorize;
        println!(
            "{} WhatsApp message sent to user {}",
            "+".green(),
            r["whatsapp_user_id"].as_str().unwrap_or("").bold()
        );
    })?;
    Ok(())
}

fn parse_template_params(kv: &[String]) -> Result<Vec<serde_json::Value>, AppError> {
    let mut out = Vec::with_capacity(kv.len());
    for (idx, raw) in kv.iter().enumerate() {
        let (k, v) = raw.split_once('=').ok_or_else(|| AppError::InvalidInput {
            msg: format!(
                "--template-param #{} is malformed (expected `key=value`): {raw}",
                idx + 1
            ),
            suggestion: Some("use --template-param name=Alice --template-param code=1234".into()),
        })?;
        let key = k.trim();
        if key.is_empty() {
            return Err(AppError::InvalidInput {
                msg: format!(
                    "--template-param #{} has an empty key (got `{raw}`)",
                    idx + 1
                ),
                suggestion: None,
            });
        }
        out.push(serde_json::json!({
            "parameter_name": key,
            "type": "text",
            "text": v,
        }));
    }
    Ok(out)
}

async fn load_client_data(source: &str) -> Result<serde_json::Value, AppError> {
    let path = Path::new(source);
    if !path.exists() {
        return Err(AppError::InvalidInput {
            msg: format!("client-data file does not exist: {}", path.display()),
            suggestion: None,
        });
    }
    let text = tokio::fs::read_to_string(path)
        .await
        .map_err(AppError::Io)?;
    serde_json::from_str(&text).map_err(|e| AppError::InvalidInput {
        msg: format!("client-data file {} is not valid JSON: {e}", path.display()),
        suggestion: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_params_returns_empty_vec() {
        let out = parse_template_params(&[]).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn single_param_parses() {
        let out = parse_template_params(&["name=Alice".to_string()]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["parameter_name"], "name");
        assert_eq!(out[0]["type"], "text");
        assert_eq!(out[0]["text"], "Alice");
    }

    #[test]
    fn missing_equals_rejected() {
        let err = parse_template_params(&["nokey".to_string()]).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn empty_key_rejected() {
        let err = parse_template_params(&["=value".to_string()]).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }
}
