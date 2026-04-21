# Changelog

All notable changes to `elevenlabs-cli` are listed here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is [SemVer](https://semver.org).

## [0.3.0] — 2026-04-21

Driven by an OpenAPI spec audit (Codex, GPT-5.4 xhigh) against the
upstream spec at `https://api.elevenlabs.io/openapi.json` (snapshot
committed at `docs/reference/openapi.elevenlabs.json`). The audit
surfaced two silently-broken defaults from 0.2.1/0.2.2 and a meaningful
batch of missing commands; both are fixed here.

### Fixed — spec drift

- **P0** `agents create` wrote `conversation_config.turn.disable_first_message_interruptions`. The spec places this field on `conversation_config.agent.*` (schema `AgentConfigAPIModel`). Server was silently dropping it — so the "greeting can't be cut off by backchannels" fix shipped in 0.2.1 never took effect. Fixed in the scaffold, `AGENTS_UPDATE_HELP`, and `agent-info gotchas.turn_taking`.
- **P0** `AGENTS_UPDATE_HELP` and `agent-info gotchas.turn_taking` pointed users at `conversation_config.turn.background_voice_detection`. The spec places it on `conversation_config.vad.*` (schema `VADConfig`). Anyone following our docs was patching a no-op path. Fixed both.
- **P0** `turn_model` is enforced by the live API (enum `turn_v2`|`turn_v3`) but absent from the current OpenAPI spec. We now label the field as empirical / not spec-mounted everywhere it's documented so nobody assumes it's a contract.
- **P0** `dubbing get-transcript` called `GET /v1/dubbing/{id}/transcript/{lang}/format/{fmt}`. The spec path is `GET /v1/dubbing/{id}/transcripts/{lang}/format/{format_type}` (plural `/transcripts/`, operationId `get_dubbing_transcripts`). The old path returned the raw transcript, not the formatted one. Fixed the route + the three route-matching integration tests.

### Added — new command surfaces

- `agents llms` — `GET /v1/convai/llm/list`. Enumerates the LLMs the Agents backend currently accepts for `conversation_config.agent.prompt.llm`. Call this before `agents create --llm …` to avoid the "accepted by clap, silently fails at conversation time" footgun observed in 0.2.0 (gemini-3.1-pro-preview, etc).
- `agents signed-url <agent_id>` — `GET /v1/convai/conversation/get-signed-url`. Issues a short-lived pre-authenticated URL that can be embedded directly in a widget / web session.
- `agents knowledge {list,search,refresh}` — new subcommand tree:
  - `list` → `GET /v1/convai/knowledge-base` (filters: `--search`, `--page-size`, `--cursor`)
  - `search <query>` → `GET /v1/convai/knowledge-base/search` (chunk-level content search; flags: `--document-id`, `--limit`)
  - `refresh <doc_id>` → `POST /v1/convai/knowledge-base/{id}/refresh` (re-fetches URL-backed docs after the source page changes)
- `conversations audio <conversation_id>` — `GET /v1/convai/conversations/{id}/audio`. Downloads the call audio mp3. Default output `conv_<id>.mp3`; override with `-o`.

### Added — `phone call` per-call override surface

- `phone call --client-data <JSON or @file>` — forwards the full `conversation_initiation_client_data` payload. Spec-backed siblings: `conversation_config_override` (override `agent.first_message`, `tts.voice_id/stability/speed`, `agent.prompt.prompt/llm` per-call), `custom_llm_extra_body`, `user_id`, `source_info`, `branch_id`, `environment`, `starting_workflow_node_id`, `dynamic_variables`. When both `--client-data` and `--dynamic-variables` are passed, the latter is merged into the former.
- `phone call --record` — flips `call_recording_enabled` on Twilio / SIP-trunk outbound calls.
- `phone call --ringing-timeout-secs <n>` — maps to `telephony_call_config.ringing_timeout_secs` so callers can bound ring time per call.

### Added — richer `agent-info` + `AGENTS_UPDATE_HELP`

- `agent-info.commands.agents update` now enumerates spec-backed patch paths we previously omitted: `turn.initial_wait_time`, `turn.silence_end_call_timeout`, `turn.soft_timeout_config`, `turn.mode`, `agent.max_conversation_duration_message`, `agent.prompt.{reasoning_effort,thinking_budget,max_tokens,ignore_default_personality,rag}`, `platform_settings.evaluation.criteria[]`.
- `AGENTS_UPDATE_HELP` documents the corrected field placements (`agent.*` and `vad.*`), the empirical-vs-spec status of `turn_model`, and the additional spec-backed turn/prompt knobs.

### Internal

- New modules: `src/commands/agents/llms.rs`, `src/commands/agents/signed_url.rs`. `src/commands/agents/knowledge.rs` gains `list/search/refresh` alongside the existing `add`.
- `src/commands/conversations.rs` gains an `audio` handler + a small private `get_bytes` helper that reuses the shared client plus the crate-internal `redact_secrets`.
- OpenAPI snapshot + refresh script landed in `docs/reference/`. Future audits: `./docs/reference/refresh.sh` regenerates the spec + two inventories.
- Test suite still green (36 suites). Three dubbing transcript route tests updated to the corrected path.

## [0.2.2] — 2026-04-21

### Added — outbound-call ergonomics

- `elevenlabs agents create --voicemail-detection` registers the `voicemail_detection` system tool so the agent hangs up cleanly on answerphones instead of recording its opening over someone's voicemail. Default OFF so inbound/web-widget agents aren't affected.
- `elevenlabs agents create --voicemail-message <text>` — implies `--voicemail-detection` and leaves the specified message on voicemail instead of hanging up. Supports `{{placeholder}}` interpolation from `--dynamic-variables` on the call.
- `agent-info` gains a new `gotchas.outbound_calls` block that explains: (a) voicemail detection is OFF by default and almost always wanted for outbound phone agents, and (b) dynamic variables are per-call (set on `phone call`), not per-agent.
- `agents update --help` lists the `conversation_config.agent.prompt.tools[]` path so contributors know how to enable voicemail detection on an existing agent.

## [0.2.1] — 2026-04-21

### Added — surfaces agents kept tripping over

- `elevenlabs agents create --expressive-mode` — enables v3 expressive prosody. Passing the flag alone auto-upgrades `--model-id` to `eleven_v3_conversational` (the only model that honours it).
- `elevenlabs agents create --max-duration-seconds <n>` — per-agent hard call cap (default stays 300s). Previously callers had to PATCH the config after create.
- `elevenlabs phone call --dynamic-variables <JSON or @file>` — per-call `{{placeholder}}` values. Forwards to `conversation_initiation_client_data.dynamic_variables` on the outbound-call body. The help text had been advertising this flag since 0.2.0 but it was never wired up — calling it used to fail with an unexpected-argument error.

### Added — agent-info manifest + `--help` now document every footgun

- `agent-info`: new top-level `known_values` (agent_tts_model_ids, agent_turn_models, agent_turn_eagerness) and `gotchas` (agents, turn_taking). Agents that bootstrap from `elevenlabs agent-info` now see the complete server-enforced allowlists without having to probe the API.
- `agent-info`: `agents create`, `agents update`, and `phone call` entries are now structured objects (description + options + defaults + common_patch_paths) instead of one-liners.
- `agents update --help` (new `AGENTS_UPDATE_HELP`): documents every common patch path (`conversation_config.agent.prompt.*`, `.tts.*`, `.turn.*`, `.conversation.*`), the server-enforced `tts.model_id` allowlist, the `turn_model` / `turn_eagerness` / `disable_first_message_interruptions` / `background_voice_detection` knobs, and four copy-paste patch examples (swap to v3 conversational, extend call limit, change LLM, rename agent).
- `agents create --help`: expanded with the full allowlist, the `eleven_v3` → `eleven_v3_conversational` correction, and the silent-drop footgun for `expressive_mode`.
- `phone call --help`: documents the new `--dynamic-variables` flag's JSON-object requirement + `@file` loading.

### Fixed — client-side validation catches the two silent bugs we hit

- **P0** `agents create --model-id eleven_v3` now errors out with exit 3 and a concrete suggestion pointing to `eleven_v3_conversational`. The server previously rejected it with the opaque "English Agents must use turbo or flash v2"; now the pre-flight error spells out the correction.
- **P0** `agents update --patch` now pre-validates the JSON body. Two footguns are caught before we hit the server:
  1. `conversation_config.tts.model_id = "eleven_v3"` — rejected with a `eleven_v3_conversational` suggestion.
  2. `conversation_config.tts.expressive_mode = true` with any `model_id` other than `eleven_v3_conversational` — the server silently drops the flag; we now reject pre-flight so callers can't falsely believe expressive mode is on.
- **P1** `phone call --dynamic-variables` enforces JSON-object shape (not array/string) and provides file-not-found guidance for the `@path` form.

### Changed — defaults rebalanced from real-world dialing

- `agents create` now scaffolds `conversation_config.turn.disable_first_message_interruptions = true`. Tiny user backchannels ("yes", "mm-hmm") no longer cut off the greeting on the first ring — the single most common audio-UX regression observed in 0.2.0.
- `agents create` now writes `turn.turn_model = "turn_v2"`, `turn.turn_eagerness = "normal"`, `turn.turn_timeout = 7` explicitly (instead of relying on server defaults). `turn_v3` is documented as an opt-in because on real calls it was observed swallowing short turn-ends when paired with some LLMs, leaving the agent silent after the user's first reply.
- `agents create` no longer sets `background_voice_detection`; real-world testing showed `true` mutes the user's own voice on speakerphones. It stays a documented PATCH knob in `agents update --help` and the `gotchas.turn_taking` block of `agent-info`.

### Internal

- New module `src/commands/agents/agent_config.rs` holds the `AGENT_TTS_MODEL_IDS` allowlist, `validate_agent_tts_model`, `validate_patch`, and the shared `GOTCHAS` text used by `--help` and `agent-info`. 7 unit tests cover the allowlist round-trip and the two patch footguns.
- All existing tests still green. Clippy + `cargo fmt --check` clean.

## [0.2.0] — 2026-04-20

### Added — new command families

- `elevenlabs dialogue` — multi-speaker synthesis with `eleven_v3`. Accepts colon-delimited `label:voice_id:text` triples, a JSON file, or `-` for stdin. `--stream` and `--with-timestamps` route to the four official endpoints; streaming variants consume NDJSON.
- `elevenlabs align <audio> <transcript|path>` — `POST /v1/forced-alignment`. Returns per-word and per-character start/end timings plus a loss score.
- `elevenlabs dubbing {create, list, show, delete, get-audio, get-transcript, resource {transcribe, translate, dub, render, migrate-segments}}` — full dubbing lifecycle including Studio-mode editable dubs.
- `elevenlabs dict {list, add-file, add-rules, show, update, download, set-rules, add-rules-to, remove-rules}` — pronunciation dictionaries (IPA + alias lexicons). Rule flags (`--rule word:phoneme`, `--alias-rule word:alias`) split on the first `:` so IPA colons survive.
- `elevenlabs doctor` — structured diagnostics. Eight checks: config file, API key presence, env-var shadowing, API-key scope, network reachability, ffmpeg, disk writeability, `default_output_dir`. JSON mode returns `{checks, summary}`; exits 0 on pass/warn, 2 on any fail.

### Added — extensions to existing commands

- `elevenlabs agents update <id> --patch <json_file>` — `PATCH /v1/convai/agents/{id}`.
- `elevenlabs agents duplicate <id> [--name <new>]` — `POST /{id}/duplicate`.
- `elevenlabs agents tools {list, create, show, update, delete, deps}` — workspace tools CRUD + `dependent-agents`.
- `elevenlabs music {detailed, stream, upload, stem-separation, video-to-music}`. `detailed` parses the new `multipart/mixed` response (JSON metadata + binary audio). `stem-separation` extracts a streaming ZIP. `video-to-music` sends the multipart part as `videos` per SDK contract.
- `elevenlabs phone batch {submit, list, show, cancel, retry, delete}` — batch-calling with CSV or JSON recipients (`-` for stdin).
- `elevenlabs phone whatsapp {call, message, accounts {list, show, update, delete}}` — template-only outbound per WhatsApp platform rules.
- `elevenlabs voices {add-shared, similar, edit}` — add from the public library, find similar voices from an audio sample, edit a voice (rename, relabel, add/remove samples).
- `elevenlabs voices list` gains the full `/v2/voices` query-param set: `--next-page-token`, `--voice-type`, `--category`, `--fine-tuning-state`, `--collection-id`, `--include-total-count`, `--voice-id` (repeatable).

### Fixed

- **P0** `agents add-knowledge` now actually attaches the created document to the agent: after POSTing to `/v1/convai/knowledge-base/{url|file|text}` it GETs the agent, appends `{id, type, name, usage_mode: "auto"}` to `conversation_config.agent.prompt.knowledge_base`, and PATCHes the agent. Previously the doc was created but left orphaned.
- **P0** `agents delete` now requires `--yes`. Every other destructive op already does; this one was missed.
- **P0** `music detailed` now correctly parses the `multipart/mixed` response. Previously treated as JSON with `audio_base64`, which would have failed against the real API.
- **P0** `music stem-separation` now matches the SDK contract: multipart `file` upload, optional `--output-format` query, optional `--stem-variation-id` / `--sign-with-c2pa` form fields, streaming ZIP response extracted into `--output-dir`.
- **P0** `phone whatsapp message` now uses template-only sends (`--template`, `--template-language`, repeatable `--template-param key=value`). Free-form `--text` was rejected by WhatsApp's platform rules.
- **P1** `agents knowledge.rs` replaced an `.unwrap()` with an exhaustive `KbSource` match.
- **P1** Streaming dialogue (`--stream`, `--stream --with-timestamps`) now parses NDJSON chunks per SDK contract instead of expecting a single JSON envelope.
- **P1** Secret redaction now runs on ad-hoc error bodies in `dict download`, `dubbing get-audio/transcript`, and `music stream` — previously those three paths bypassed the central `check_status` redactor.
- **P1** `voices edit` pre-fetches `GET /v1/voices/{id}` when `--name` is absent so the SDK-required `name` field is always sent. Fetch failure maps to an actionable `invalid_input`.
- **P1** `doctor` network probe swapped from `HEAD /` (404 pre-auth) to `GET /v1/models` (200 unauthenticated).
- `AppError::InvalidInput` is now a struct `{msg, suggestion: Option<String>}`. ~120 call sites migrated; five high-value sites carry actionable suggestions so the error envelope never emits the generic "check arguments" fallback there.

### Changed — defaults (breaking)

- `elevenlabs agents create --llm` default: `gemini-2.0-flash-001` → `gemini-3.1-flash-lite-preview` (old default superseded).
- `elevenlabs agents create --model-id` default: `eleven_turbo_v2` → `eleven_flash_v2_5` (old default deprecated).

### Changed — breaking flags

- `phone whatsapp call`: `--whatsapp-account` → `--whatsapp-phone-number`; `--recipient` → `--whatsapp-user`; new required `--permission-template` + `--permission-template-language`.
- `phone whatsapp message`: same id rename; `--text` removed; new required `--template` + `--template-language`; repeatable `--template-param key=value`; optional `--client-data <json_file>`.
- `phone batch list`: `--status` / `--agent-id` removed (no SDK filter).
- `music upload`: `--name` / `--composition-plan` removed; `--extract-composition-plan` added.
- `music stem-separation`: positional `SOURCE` renamed to `FILE` (song_id path removed); `--stems` removed; `--output-format`, `--stem-variation-id`, `--sign-with-c2pa` added.
- `music video-to-music`: `--model` removed; `--sign-with-c2pa` added.

### Framework polish

- `after_long_help` Tips + Examples blocks on every high-value command (19 total) in `src/help.rs`.
- `cargo test`: 36 suites, 172 tests passing.
- Module splits: `agents.rs`, `voices.rs`, `music.rs`, `phone.rs` → directory modules (one submodule per action). No file over ~200 lines.

### Internal

- `ElevenLabsClient::patch_json` helper.
- `client::redact_secrets` now `pub(crate)` for the three ad-hoc error paths that cannot route through `check_status`.
- `zip` 2.x added for stem-separation response extraction.
- Reviewer artefacts preserved at `plans/reviews/codex-v0.2-review.md` + `gemini-v0.2-review.md`.

## [0.1.6] — 2026-04-17

- Saved config.toml wins over `ELEVENLABS_API_KEY` env var. The env var is now a fallback only (previously it sometimes silently shadowed the saved key).

## [0.1.5] — 2026-04-17

- Diagnose env-shadow so agents stop hitting silent auth failures.

## [0.1.4] — 2026-04-16

- P0 fixes + grounded coverage across every command.

## [0.1.3] — 2026-04-16

- STT: full REST parameter coverage + character-level timings.

## [0.1.2] — 2026-04-08

- Codex audit pass: `/v2` voices, text-to-voice path, hardening.

## [0.1.1] — 2026-04-08

- Env var precedence fix + governance files (AGENTS.md, CONTRIBUTING.md, SECURITY.md).
- Fix: voice name resolution + correct music endpoints.
- Fix: `--loop` flag name (clap was using field name `looping`).

## [0.1.0] — 2026-04-08

- Initial release. TTS, STT, SFX, voices, agents, music, user, history, phone, conversations, skill install, self-update, agent-info manifest, config precedence, semantic exit codes (0/1/2/3/4), JSON envelope on stdout.
