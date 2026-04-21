//! `agents create` — build a new conversational AI agent from a minimal
//! set of flags. The full `conversation_config` + `platform_settings`
//! scaffolding is filled in with sensible defaults so agents don't have to
//! know the entire schema to spin up a working agent.

use crate::client::ElevenLabsClient;
use crate::commands::agents::agent_config::{EXPRESSIVE_MODEL_ID, validate_agent_tts_model};
use crate::config::AppConfig;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    ctx: Ctx,
    cfg: &AppConfig,
    client: &ElevenLabsClient,
    name: String,
    system_prompt: String,
    first_message: Option<String>,
    voice_id: Option<String>,
    language: String,
    llm: String,
    temperature: f32,
    model_id: String,
    expressive_mode: bool,
    max_duration_seconds: u32,
    voicemail_detection: bool,
    voicemail_message: Option<String>,
) -> Result<(), AppError> {
    // Passing --voicemail-message implies --voicemail-detection; agents
    // don't need to remember both. The reverse is not implied — enabling
    // detection without a message defaults to hanging up on voicemail.
    let voicemail_detection = voicemail_detection || voicemail_message.is_some();
    let voice_id = voice_id.unwrap_or_else(|| cfg.default_voice_id());

    // If the caller opted into expressive_mode but didn't override --model-id
    // away from the CLI default, auto-pick the only model that actually
    // supports it. This is a convenience: `--expressive-mode` on its own
    // should just do the right thing.
    let effective_model_id = if expressive_mode && model_id == "eleven_flash_v2_5" {
        EXPRESSIVE_MODEL_ID.to_string()
    } else {
        model_id
    };

    validate_agent_tts_model(&effective_model_id)?;

    if expressive_mode && effective_model_id != EXPRESSIVE_MODEL_ID {
        return Err(AppError::bad_input_with(
            format!(
                "--expressive-mode is incompatible with --model-id {effective_model_id}. The \
                 server only honours expressive_mode on {EXPRESSIVE_MODEL_ID} and silently drops \
                 it on any other model"
            ),
            format!(
                "Drop --model-id to auto-pick {EXPRESSIVE_MODEL_ID}, or pass \
                 --model-id {EXPRESSIVE_MODEL_ID} explicitly."
            ),
        ));
    }

    // Build the system-tools list. end_call is always present; voicemail
    // detection is opt-in because it's only useful on outbound phone
    // agents — web-widget agents that answer voicemail systems are rare.
    let mut tools = vec![serde_json::json!(
        {"type": "system", "name": "end_call", "description": ""}
    )];
    if voicemail_detection {
        let mut vmt = serde_json::json!(
            {"type": "system", "name": "voicemail_detection", "description": ""}
        );
        if let Some(msg) = voicemail_message.as_ref() {
            vmt["voicemail_message"] = serde_json::Value::String(msg.clone());
        }
        tools.push(vmt);
    }

    let conversation_config = serde_json::json!({
        "agent": {
            "language": language,
            "prompt": {
                "prompt": system_prompt,
                "llm": llm,
                "tools": tools,
                "knowledge_base": [],
                "temperature": temperature,
            },
            "first_message": first_message,
            "dynamic_variables": { "dynamic_variable_placeholders": {} },
            // Per OpenAPI spec (AgentConfigAPIModel), this field lives on
            // agent.*, not turn.*. v0.2.1 / v0.2.2 wrote it under turn which
            // the server silently dropped — so the "greeting can't be cut
            // off" guarantee never took effect. Fixed in v0.3.0.
            "disable_first_message_interruptions": true
        },
        "asr": {
            "quality": "high",
            "provider": "elevenlabs",
            "user_input_audio_format": "pcm_16000",
            "keywords": []
        },
        "tts": {
            "voice_id": voice_id,
            "model_id": effective_model_id,
            "expressive_mode": expressive_mode,
            "agent_output_audio_format": "pcm_16000",
            "optimize_streaming_latency": 3,
            "stability": 0.5,
            "similarity_boost": 0.8
        },
        "turn": {
            // turn_model is not in the checked-in OpenAPI spec, but the live
            // API enforces the enum {turn_v2, turn_v3} (verified via probe,
            // 2026-04-21). turn_v2 is the proven-working detector; turn_v3
            // was observed swallowing short turn-ends on some LLM configs.
            // Treat this as an empirical knob, not a spec-backed one.
            "turn_model": "turn_v2",
            "turn_timeout": 7,
            "turn_eagerness": "normal"
        },
        "conversation": {
            "max_duration_seconds": max_duration_seconds,
            "client_events": [
                "audio", "interruption", "user_transcript",
                "agent_response", "agent_response_correction"
            ]
        },
        "language_presets": {},
        "is_blocked_ivc": false,
        "is_blocked_non_ivc": false
    });

    let platform_settings = serde_json::json!({
        "widget": {
            "variant": "full",
            "avatar": { "type": "orb", "color_1": "#6DB035", "color_2": "#F5CABB" },
            "feedback_mode": "during"
        },
        "evaluation": {},
        "auth": { "allowlist": [] },
        "overrides": {},
        "call_limits": { "agent_concurrency_limit": -1, "daily_limit": 100000 },
        "privacy": {
            "record_voice": true,
            "retention_days": 730,
            "delete_transcript_and_pii": true,
            "delete_audio": true,
            "apply_to_existing_conversations": false
        },
        "data_collection": {}
    });

    let body = serde_json::json!({
        "name": name,
        "conversation_config": conversation_config,
        "platform_settings": platform_settings,
    });

    let resp: serde_json::Value = client.post_json("/v1/convai/agents/create", &body).await?;
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        println!(
            "{} created agent {} ({})",
            "+".green(),
            name.bold(),
            v.get("agent_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .dimmed()
        );
    });
    Ok(())
}
