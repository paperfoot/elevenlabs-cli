# Changelog

All notable changes to `elevenlabs-cli` are listed here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is [SemVer](https://semver.org).

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
