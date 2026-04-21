//! Shared constants + validators for conversational-AI agent config.
//!
//! The ElevenLabs Agents API has a handful of allowlists and silent-drop
//! behaviours that consistently trip up LLM contributors. We surface them
//! client-side so the wrong inputs error out with an actionable suggestion
//! before we ever hit the server. Two invariants in particular:
//!
//! 1. `conversation_config.tts.model_id` must be one of the six values in
//!    [`AGENT_TTS_MODEL_IDS`]. The superficially similar `eleven_v3` is the
//!    multi-speaker dialogue / text-to-voice model and is **not** accepted by
//!    agents — the server returns "English Agents must use turbo or flash v2".
//!    The correct realtime model is `eleven_v3_conversational`.
//!
//! 2. `conversation_config.tts.expressive_mode = true` only takes effect
//!    with `model_id = "eleven_v3_conversational"`. With any other model the
//!    server **silently drops** the flag — no error, no warning — which the
//!    caller then mis-reads as "expressive mode is active". We reject this
//!    combination client-side instead.
//!
//! Helpful pitfall documentation also lives here ([`GOTCHAS`]) so the
//! `agent-info` manifest and `--help` screens can render the same text
//! without duplication.

use crate::error::AppError;

/// Every `conversation_config.tts.model_id` the Agents backend currently
/// accepts. Sourced from the 400 response body on invalid input and
/// cross-checked against `TTSConversationalConfig-Input.model_id` in the
/// vendored OpenAPI spec. `eleven_flash_v2_5` is the current agent
/// recommendation; the `eleven_turbo_v2*` entries are still accepted but
/// marked deprecated in the public models docs.
pub const AGENT_TTS_MODEL_IDS: &[&str] = &[
    "eleven_flash_v2_5",
    "eleven_flash_v2",
    "eleven_multilingual_v2",
    "eleven_v3_conversational",
    "eleven_turbo_v2_5",
    "eleven_turbo_v2",
];

/// The one and only model that actually honours `expressive_mode`.
pub const EXPRESSIVE_MODEL_ID: &str = "eleven_v3_conversational";

/// Human-readable pitfall strings, shared between `--help` and `agent-info`.
pub const GOTCHAS: &[&str] = &[
    "Agent TTS model_id allowlist: eleven_flash_v2_5 (recommended), eleven_flash_v2, \
     eleven_multilingual_v2, eleven_v3_conversational. eleven_turbo_v2_5 and eleven_turbo_v2 \
     are still accepted but DEPRECATED per the public models docs — prefer the flash equivalents. \
     `eleven_v3` (no `_conversational`) is the dialogue/ttv model and agents reject it with \
     'English Agents must use turbo or flash v2'.",
    "expressive_mode only takes effect with model_id=eleven_v3_conversational. With any other \
     model the server silently drops the flag — the PATCH succeeds but expressive_mode stays false.",
    "--llm accepts any string but the Agents backend has its own allowlist. Discover the live list \
     with `elevenlabs agents llms`. This CLI defaults to `gemini-3.1-flash-lite-preview` (the newest \
     Gemini flash preview in the ElevenLabs enum) — the OpenAPI spec default `gemini-2.5-flash` is \
     the older stable release. If an accepted LLM fails to generate at conversation time (0 output \
     tokens in `conversations show`), swap LLMs.",
    "Spec default for conversation.max_duration_seconds is 600 (10 min). This CLI's \
     `agents create` defaults to the same value. Bump to 1800 (30 min) or higher for long-form \
     interviews / coaching — calls hang up hard at this limit regardless of transcript state.",
];

/// Reject a user-supplied agent `model_id` if we know the server will.
pub fn validate_agent_tts_model(model_id: &str) -> Result<(), AppError> {
    if AGENT_TTS_MODEL_IDS.contains(&model_id) {
        return Ok(());
    }
    if model_id == "eleven_v3" {
        return Err(AppError::bad_input_with(
            "model_id 'eleven_v3' is not valid for conversational AI agents (the server will \
             reject this with: \"English Agents must use turbo or flash v2\")",
            "Use --model-id eleven_v3_conversational for the realtime expressive model, or \
             eleven_flash_v2_5 for the low-latency default.",
        ));
    }
    Err(AppError::bad_input_with(
        format!(
            "model_id '{model_id}' is not in the agent allowlist ({})",
            AGENT_TTS_MODEL_IDS.join(", ")
        ),
        format!(
            "Pass one of: {}. For the v3 expressive realtime model use eleven_v3_conversational \
             (not eleven_v3 — that's the dialogue/ttv model).",
            AGENT_TTS_MODEL_IDS.join(", ")
        ),
    ))
}

/// Scan an `agents update --patch` body for the same footguns. The patch is
/// passed through to the API verbatim, so we only *reject* — never rewrite —
/// known-bad values here. A success means the patch is either free of the
/// fields we validate, or every field we recognise looks correct.
pub fn validate_patch(patch: &serde_json::Value) -> Result<(), AppError> {
    let tts = patch
        .get("conversation_config")
        .and_then(|cc| cc.get("tts"));

    let model_id = tts.and_then(|t| t.get("model_id")).and_then(|m| m.as_str());
    let expressive = tts
        .and_then(|t| t.get("expressive_mode"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);

    if let Some(mid) = model_id {
        validate_agent_tts_model(mid)?;
    }

    if expressive {
        match model_id {
            Some(m) if m == EXPRESSIVE_MODEL_ID => { /* OK */ }
            Some(m) => {
                return Err(AppError::bad_input_with(
                    format!(
                        "patch sets tts.expressive_mode=true with model_id='{m}', but the server \
                         only honours expressive_mode on '{EXPRESSIVE_MODEL_ID}' and silently drops \
                         it on any other model"
                    ),
                    format!(
                        "Either set tts.model_id to '{EXPRESSIVE_MODEL_ID}' in the same patch, or \
                         drop tts.expressive_mode (leaving it false)."
                    ),
                ));
            }
            None => {
                return Err(AppError::bad_input_with(
                    "patch sets tts.expressive_mode=true but leaves model_id unchanged. The \
                     server silently drops expressive_mode unless model_id is also set to \
                     'eleven_v3_conversational' in the same patch, or already is that value \
                     on the agent."
                        .to_string(),
                    format!(
                        "Add \"model_id\":\"{EXPRESSIVE_MODEL_ID}\" to conversation_config.tts \
                         in the same patch (or verify via `agents show` that it already is \
                         {EXPRESSIVE_MODEL_ID})."
                    ),
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_eleven_v3_with_friendly_pointer() {
        let err = validate_agent_tts_model("eleven_v3").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("eleven_v3"), "msg = {msg}");
        let suggestion = err.suggestion();
        assert!(
            suggestion.contains("eleven_v3_conversational"),
            "suggestion must name the correct model: {suggestion}"
        );
    }

    #[test]
    fn accepts_every_allowlisted_id() {
        for id in AGENT_TTS_MODEL_IDS {
            validate_agent_tts_model(id).unwrap_or_else(|e| {
                panic!("{id} should be accepted: {e}");
            });
        }
    }

    #[test]
    fn patch_rejects_expressive_with_turbo() {
        let patch = json!({
            "conversation_config": {
                "tts": { "model_id": "eleven_turbo_v2", "expressive_mode": true }
            }
        });
        let err = validate_patch(&patch).unwrap_err();
        assert!(format!("{err}").contains("expressive_mode"));
    }

    #[test]
    fn patch_rejects_expressive_without_model_change() {
        let patch = json!({
            "conversation_config": { "tts": { "expressive_mode": true } }
        });
        validate_patch(&patch).unwrap_err();
    }

    #[test]
    fn patch_accepts_expressive_with_v3_conversational() {
        let patch = json!({
            "conversation_config": {
                "tts": { "model_id": "eleven_v3_conversational", "expressive_mode": true }
            }
        });
        validate_patch(&patch).unwrap();
    }

    #[test]
    fn patch_rejects_eleven_v3_in_body() {
        let patch = json!({ "conversation_config": { "tts": { "model_id": "eleven_v3" } } });
        validate_patch(&patch).unwrap_err();
    }

    #[test]
    fn patch_allows_unrelated_fields() {
        let patch = json!({ "name": "Renamed", "conversation_config": { "agent": { "prompt": { "prompt": "hi" } } } });
        validate_patch(&patch).unwrap();
    }
}
