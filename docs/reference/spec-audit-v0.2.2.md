# ElevenLabs CLI — spec audit (v0.2.2)

Excluded from ranking as intentionally out of scope per `README.md`/`CLAUDE.md`: agent-testing, branches/drafts/deployments, audio-native, studio, workspace/service accounts, MCP/secrets/settings/dashboard/billing surfaces.

## 1. Endpoint coverage gaps (top 10 adds)

1. `get_conversation_signed_link` — `GET /v1/convai/conversation/get-signed-url`
   Use case: create a pre-authenticated web/widget session for an agent without hand-building session URLs or asking the caller to manage tokens.
   Suggested invocation: `elevenlabs agents signed-url <agent_id> --client-data @client-data.json`

2. `list_available_llms` — `GET /v1/convai/llm/list`
   Use case: avoid the current `--llm accepts any string` footgun and let agents discover the server’s actual allowlist before `agents create` / `agents update`.
   Suggested invocation: `elevenlabs agents llms`

3. `get_knowledge_base_list_route` — `GET /v1/convai/knowledge-base`
   Use case: inspect existing KB docs/folders/IDs before attaching, moving, refreshing, or deleting them.
   Suggested invocation: `elevenlabs agents knowledge list --search policy`

4. `search_knowledge_base_content_route` — `GET /v1/convai/knowledge-base/search`
   Use case: search KB chunks/doc content when debugging RAG or confirming whether an agent can already cite a fact.
   Suggested invocation: `elevenlabs agents knowledge search "refund window"`

5. `refresh_url_document_route` — `POST /v1/convai/knowledge-base/{documentation_id}/refresh`
   Use case: re-fetch a URL-backed KB document after the source page changes; currently you can add URLs but not refresh them.
   Suggested invocation: `elevenlabs agents knowledge refresh <doc_id>`

6. `get_conversation_audio_route` — `GET /v1/convai/conversations/{conversation_id}/audio`
   Use case: download the actual call audio after `conversations show`, which is the core QA/debug loop for voice agents.
   Suggested invocation: `elevenlabs conversations audio <conversation_id> -o call.mp3`

7. `get_dubbing_resource` — `GET /v1/dubbing/resource/{dubbing_id}`
   Use case: inspect Studio-mode dubbing state (segments, speakers, languages) before calling `resource transcribe/translate/dub/render`.
   Suggested invocation: `elevenlabs dubbing resource show <dubbing_id>`

8. `text_to_speech_stream_with_timestamps` — `POST /v1/text-to-speech/{voice_id}/stream/with-timestamps`
   Use case: low-latency TTS with alignment in one pass; the current CLI hard-errors on `--stream + --with-timestamps`.
   Suggested invocation: `elevenlabs tts "..." --stream --with-timestamps -o out.mp3 --save-timestamps out.jsonl`

9. `get_phone_number_route` — `GET /v1/convai/phone-numbers/{phone_number_id}`
   Use case: inspect one number’s provider/trunk/agent binding instead of scraping the list output.
   Suggested invocation: `elevenlabs phone show <phone_number_id>`

10. `update_phone_number_route` — `PATCH /v1/convai/phone-numbers/{phone_number_id}`
    Use case: retarget a number to a different agent, relabel it, or edit trunk/livekit settings without leaving the CLI.
    Suggested invocation: `elevenlabs phone update <phone_number_id> --patch phone.json`

Honourable mentions that are useful but lower leverage than the 10 above: `GET /v1/history/{history_item_id}/audio`, `POST /v1/history/download`, `GET/DELETE /v1/speech-to-text/transcripts/{transcription_id}`, `POST /v1/audio-isolation/stream`, `POST /v1/speech-to-speech/{voice_id}/stream`, `GET /v1/convai/agents/{agent_id}/link`.

## 2. Request-body / query-param drops (by family)

### agents

- `create_agent_route` exposes `conversation_config.turn.initial_wait_time`, `conversation_config.turn.silence_end_call_timeout`, and `conversation_config.turn.soft_timeout_config.{timeout_seconds,message,use_llm_generated_message}`. `agents create` neither exposes nor documents them, even though they directly affect phone-agent ringing/idle behaviour.
- `AgentConfigAPIModel-Input.prompt` exposes `reasoning_effort`, `thinking_budget`, `max_tokens`, `ignore_default_personality`, `custom_llm`, `cascade_timeout_seconds`, and `rag.{enabled,embedding_model,max_vector_distance,max_documents_length,max_retrieved_rag_chunks_count,num_candidates,query_rewrite_prompt_override}`. The CLI only surfaces `llm` and `temperature`; the rest are also absent from `AGENTS_UPDATE_HELP` and `agent-info common_patch_paths`.
- `get_agents_route` supports `search`, `archived`, `show_only_owned_agents`, `created_by_user_id`, `sort_by`, `sort_direction`, and `cursor`. `agents list` exposes none of them.
- `get_tools_route` supports `search`, `types`, `show_only_owned_documents`, `created_by_user_id`, `sort_by`, `sort_direction`, and `cursor`. `agents tools list` currently exposes none.

### phone / conversations

- `handle_twilio_outbound_call` / `handle_sip_trunk_outbound_call` accept `call_recording_enabled` and `telephony_call_config.ringing_timeout_secs`; `phone call` cannot set either.
- `ConversationInitiationClientDataRequest-Input` has top-level siblings `conversation_config_override`, `custom_llm_extra_body`, `user_id`, `source_info`, `branch_id`, `environment`, `starting_workflow_node_id`, and `dynamic_variables`. `phone call` only exposes `dynamic_variables`; `phone whatsapp call` exposes none; `phone whatsapp message` only takes a raw JSON file.
- `whatsapp_outbound_message` `template_params` supports `header`, `body`, and `button` components. Header params can be `text`, `image`, `document`, or `location`. The CLI only synthesizes one `body` component with text params, so many approved WhatsApp templates are uncallable.
- `get_conversation_histories_route` supports `call_successful`, date/duration/rating filters, `has_feedback_comment`, `user_id`, `tool_names*`, `main_languages`, `summary_mode`, `search`, `conversation_initiation_source`, `branch_id`, and `topic_ids`. `conversations list` only exposes `agent_id`, `page_size`, and `cursor`.

### voices

- `add_voice` (`POST /v1/voices/add`) supports `labels` and `remove_background_noise`; `voices clone` exposes neither.
- `create_voice` (`POST /v1/text-to-voice`) supports `labels` and `played_not_selected_voice_ids`; `voices save-preview` only sends `generated_voice_id`, `voice_name`, and `voice_description`.
- `text_to_voice_design` (`POST /v1/text-to-voice/design`) adds `reference_audio_base64`, `prompt_strength`, `remixing_session_id`, `remixing_session_iteration_id`, and is the spec-backed home for `model_id` + `stream_previews`. Current `voices design` calls `/v1/text-to-voice/create-previews`, so its `--model` and `--stream-previews` flags are not backed by that request schema.
- `get_library_voices` supports `descriptives` and plural `use_cases`; the CLI only offers single `--use-case` and no `--descriptive`.

### tts / stt / audio

- `text_to_speech_full` / `text_to_speech_full_with_timestamps` expose `pronunciation_dictionary_locators`; `tts` has no way to send them.
- `text_to_speech_stream_with_timestamps` exists in the spec, but `tts` hard-errors on `--stream + --with-timestamps`.
- `speech_to_text` POST is already close to complete. The bigger remaining STT gap is follow-up lifecycle coverage (`GET/DELETE /v1/speech-to-text/transcripts/{transcription_id}`), not missing POST fields.
- `speech_to_speech_full` and `audio_isolation` POSTs also look complete on body fields; the missing surfaces are the streaming sibling endpoints.

### music

- `generate`, `compose_detailed`, and `stream_compose` all expose `music_prompt`, `finetune_id`, and `use_phonetic_names`; the CLI exposes none of them.
- `compose_detailed` additionally exposes `with_timestamps` and `model_style_prefix`; `music detailed` cannot set either.
- `compose_plan` exposes `source_composition_plan`; `music plan` cannot seed from an existing plan.
- `stream_compose` does not include `respect_sections_durations` or `sign_with_c2pa` in its schema, but `music stream` inherits both flags from the shared builder. Those fields are likely being silently dropped.

### dubbing

- `create_dubbing` exposes `name`, `target_accent`, `foreground_audio_file`, `background_audio_file`, `csv_file`, and `csv_fps`; `dubbing create` cannot send any of them.
- `list_dubs` exposes `cursor`, `order_by`, and `order_direction`; `dubbing list` only exposes `dubbing_status`, `filter_by_creator`, and `page_size`.
- The Studio resource re-run endpoints are already patch-pass-through, so the main dubbing issue is not missing request knobs on those commands; it is the absence of the resource GET/speaker/language endpoints entirely.

## 3. Enums worth validating client-side

- `tts --format`, `audio convert --format`, `sfx --format`, `music {compose,detailed,stream,video-to-music,stem-separation} --format`
  Validate against the spec’s path-level `output_format` enums (`AllowedOutputFormats` / `TTSOutputFormat` / `NonStreamingOutputFormats` depending on endpoint). This is the biggest remaining “silent typo -> server 422” surface.

- `music --model`
  Allowlist: `music_v1`.
  Commands: `music compose`, `music detailed`, `music stream`, `music plan`.

- `agents update --patch` additional turn/prompt enums
  Allowlists:
  `conversation_config.turn.mode = silence|turn`
  `conversation_config.turn.turn_eagerness = patient|normal|eager`
  `conversation_config.agent.prompt.reasoning_effort = none|minimal|low|medium|high|xhigh`
  Surface: `known_values` + extra `validate_patch()` checks in `src/commands/agents/agent_config.rs`.

- `dubbing list` filters
  Allowlists:
  `dubbing_status = dubbing|dubbed|failed`
  `filter_by_creator = personal|others|all`
  `order_direction = ASCENDING|DESCENDING`
  Surface: `src/cli.rs` value parsers once those query params are exposed.

- `dubbing resource render`
  Allowlist: `render_type = mp4|aac|mp3|wav|aaf|tracks_zip|clips_zip`.
  Surface: new first-class render flags or JSON validator for `resource render --patch`.

- `phone update` (if added)
  Allowlists:
  `livekit_stack = standard|static`
  `outbound_trunk_config.transport = auto|udp|tcp|tls`
  `inbound_trunk_config.media_encryption / outbound_trunk_config.media_encryption = disabled|allowed|required`

- `whatsapp message` template components (if expanded)
  Allowlists:
  component `type = header|body|button`
  header param `type = text|image|document|location`
  button `sub_type` is required by schema and should be validated before send.

Note: STT `timestamps_granularity` and STT `model_id` are already clamped by `clap`; they are not remaining gaps.

## 4. Agent schema drift

- `conversation_config.turn.turn_model`
  Drift: present in the CLI scaffold, `AGENTS_UPDATE_HELP`, and `agent-info`, but absent from both `Body_Create_Agent_v1_convai_agents_create_post` and `AgentConfigOverride-Input`. The spec defines a standalone `TurnModel` schema, but nothing references it.
  Recommendation: flag as spec drift. Do not extend spec-derived validation around `turn_model`. If live traffic proves the field still works, keep it as an empirical knob but label it clearly as “not present in the 2026-04-21 OpenAPI spec”.

- `conversation_config.turn.disable_first_message_interruptions`
  Drift: CLI currently writes/documents it under `turn`; the spec places it at `conversation_config.agent.disable_first_message_interruptions`.
  Recommendation: fix the create scaffold and the documented patch path now.

- `conversation_config.turn.background_voice_detection`
  Drift: docs/`agent-info` advertise it under `turn`; the spec places it at `conversation_config.vad.background_voice_detection`.
  Recommendation: fix `AGENTS_UPDATE_HELP` and `agent-info` now. If you later add a flag, target `vad.background_voice_detection`.

- `conversation_config.turn.initial_wait_time`, `conversation_config.turn.silence_end_call_timeout`, `conversation_config.turn.soft_timeout_config`
  Status: present in the create/update schemas, absent from CLI flags and absent from patch-path docs.
  Recommendation: document them in `AGENTS_UPDATE_HELP` / `agent-info` before adding flags. They are high-value phone-agent knobs.

- `conversation_config.agent.max_conversation_duration_message`
  Status: present in `AgentConfigAPIModel-Input` and `ConversationInitiationClientDataRequest-Input.agent`, absent from create flags and patch-path docs.
  Recommendation: document as a patch-only knob; optional future flag.

- `conversation_config.agent.prompt.ignore_default_personality`
  Status: present, not surfaced.
  Recommendation: document in `AGENTS_UPDATE_HELP` / `agent-info` rather than add a flag immediately.

- `conversation_config.agent.prompt.reasoning_effort`, `thinking_budget`, `max_tokens`, `custom_llm`, `cascade_timeout_seconds`, `rag.*`
  Status: present, not surfaced, not documented.
  Recommendation: add to patch-path documentation now; only promote to flags if repeated real-world demand appears.

- `platform_settings.evaluation.criteria[]` and `platform_settings.analysis_llm`
  Status: present, not surfaced or documented.
  Recommendation: document as patch-only surfaces for QA/evaluation workflows.

- Privacy / safety naming drift
  Current spec names are `platform_settings.privacy.zero_retention_mode`, `platform_settings.privacy.conversation_history_redaction`, and `platform_settings.guardrails.*`.
  I did not find current schema fields named `pii_redaction` or `safety.sanitize`.
  Recommendation: document the current path names; ignore the legacy names unless a live API trace proves aliases still exist.

- Newly required fields
  I did not find a new create-time required field beyond top-level `conversation_config`. The current agent drift is mostly wrong nesting, missing optional knobs, and documentation that treats non-spec fields as canonical.

## 5. phone call / conversation_initiation_client_data shape

Yes. The full schema is `ConversationInitiationClientDataRequest-Input`, and `phone call` currently only exposes the narrowest branch of it.

Top-level siblings worth exposing:

- `conversation_config_override`
- `custom_llm_extra_body`
- `user_id`
- `source_info.{source,version}`
- `branch_id`
- `environment`
- `starting_workflow_node_id`
- `dynamic_variables`

The highest-value sub-surface is `conversation_config_override`, which already supports:

- `turn.soft_timeout_config.message`
- `tts.{voice_id,stability,speed,similarity_boost}`
- `conversation.text_only`
- `agent.{first_message,language,max_conversation_duration_message}`
- `agent.prompt.{prompt,llm,tool_ids,native_mcp_server_ids,knowledge_base}`

Recommendation:

- Add `--client-data <JSON|@file>` to `phone call` and `phone whatsapp call`.
- Keep `--dynamic-variables` as convenience sugar, but merge it into the full client-data object instead of special-casing it as the only supported shape.
- Keep `phone whatsapp message --client-data`, but consider allowing inline JSON as well as file paths for parity with `phone call`.

Separate from `conversation_initiation_client_data`, the Twilio/SIP outbound-call body also has two non-client-data siblings that are worth surfacing:

- `call_recording_enabled`
- `telephony_call_config.ringing_timeout_secs`

## 6. Response-field sanity check

- `tts --with-timestamps`
  Code depends on `audio_base64`, `alignment`, and `normalized_alignment`.
  Spec: `AudioWithTimestampsResponseModel` still exposes exactly those fields.
  Verdict: no drift found.

- `voices design`
  Code depends on `previews[].generated_voice_id` and `previews[].audio_base_64`.
  Spec: `VoicePreviewResponseModel` still exposes exactly those fields.
  Verdict: no response-name drift; the bigger issue is request-path/request-body drift (`/create-previews` vs `/design`), not response parsing.

- `phone call` / `phone whatsapp call` / `phone whatsapp message`
  The CLI relies on the raw response carrying `conversation_id`.
  Spec: `TwilioOutboundCallResponse`, `WhatsAppOutboundCallResponse`, and `WhatsAppOutboundMessageResponse` all still contain `conversation_id`; Twilio also still contains `callSid`.
  Verdict: field names are stable. It is worth documenting `conversation_id` explicitly in help/`agent-info` because downstream `conversations show` / `conversations audio` flows depend on it.

- `dubbing create`
  Code depends on `dubbing_id` and `expected_duration_sec`.
  Spec: `DoDubbingResponseModel` still exposes both.
  Verdict: no drift found.

- `dubbing get-transcript`
  Drift: the CLI calls `GET /v1/dubbing/{dubbing_id}/transcript/{language_code}/format/{format}`.
  Spec exposes:
  `GET /v1/dubbing/{dubbing_id}/transcript/{language_code}` (`get_dubbed_transcript_file`)
  `GET /v1/dubbing/{dubbing_id}/transcripts/{language_code}/format/{format_type}` (`get_dubbing_transcripts`)
  The CLI path is the only checked-in endpoint literal that does not appear in `docs/reference/endpoints-inventory.txt`.
  Verdict: real path drift, not just a missing feature.

## 7. Top 5 concrete fixes to ship next

1. Fix agent path drift in create/help/manifest
   Files: `src/commands/agents/create.rs`, `src/help.rs`, `src/commands/agent_info.rs`
   Change: move `disable_first_message_interruptions` to `conversation_config.agent.*`; stop documenting `turn.background_voice_detection`; document `vad.background_voice_detection`; label `turn_model` as empirical/non-spec or remove it from spec-derived help.
   Impact: prevents agents from relying on wrong patch paths and reduces the chance of another “flag looked accepted but the server ignored it” bug.

2. Add full `--client-data` pass-through to outbound call commands
   Files: `src/cli.rs`, `src/commands/phone/call.rs`, `src/commands/phone/whatsapp/call.rs`, `src/help.rs`, `src/commands/agent_info.rs`
   Change: add `--client-data <JSON|@file>`; merge `--dynamic-variables` into it; optionally add `--call-recording-enabled` and `--ringing-timeout-secs`.
   Impact: unlocks per-call overrides (`conversation_config_override`, `user_id`, `environment`, etc.) that real outbound workflows need, without forcing users into batch JSON just to set one call up correctly.

3. Implement `agents llms` and feed it back into agent safety surfaces
   Files: `src/cli.rs`, `src/main.rs`, new command module (for example `src/commands/agents/llms.rs`), `src/commands/agent_info.rs`, optionally `src/commands/agents/agent_config.rs`
   Change: add `GET /v1/convai/llm/list`; surface the list in human/JSON output; use it to tighten `agent-info` and, if desired, pre-flight validation/documentation around `--llm`.
   Impact: removes the biggest remaining guesswork surface on agent creation after the `model_id` / `expressive_mode` fixes.

4. Fix `dubbing get-transcript` to the spec path
   Files: `src/commands/dubbing/transcript.rs`, `src/commands/dubbing/mod.rs`, `src/commands/agent_info.rs`
   Change: switch the formatted-download command to `GET /v1/dubbing/{dubbing_id}/transcripts/{language_code}/format/{format_type}`; optionally add a separate raw transcript command for `GET /v1/dubbing/{dubbing_id}/transcript/{language_code}`.
   Impact: removes a concrete endpoint drift that can already fail against the checked-in spec.

5. Split spec-backed voice-design surfaces correctly
   Files: `src/commands/voices/design.rs`, `src/cli.rs`, `src/help.rs`, `src/commands/agent_info.rs`
   Change: either route flagful design requests to `POST /v1/text-to-voice/design`, or split `/create-previews` and `/design` into separate commands; add the missing `/remix`/preview-stream follow-ons later.
   Impact: fixes a likely silent-drop surface where `--model` and `--stream-previews` are exposed on a request path whose schema does not define them.
