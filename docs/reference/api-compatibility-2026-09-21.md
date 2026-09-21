# API compatibility audit — 2026-09-21

The installed Homebrew binary and latest published release were both v0.3.2
(2026-04-21). Authentication, voice listing, model listing, and the agent LLM
catalog succeeded against the live API. The refreshed OpenAPI snapshot contains
302 paths / 391 operations; none of the April snapshot's 225 paths disappeared.
Path presence alone does not establish request compatibility.

## Changes in v0.3.3

| Area | Finding | Correction |
| --- | --- | --- |
| STT | Clap accepted only Scribe v1/v2, blocking Medical and future models. Scribe v1 was scheduled for removal on July 9. | Accept batch model IDs; support Medical with no-verbatim; give a migration error for v1. |
| Music | Omitted model IDs selected deprecated `music_v1`; upload extraction sent deprecated `true`. | Explicit `music_v2_5` default and extraction model IDs; retain v1 for existing section plans and explicit overrides. |
| Music plans | Redirected CLI output was submitted with its JSON envelope still attached. | Unwrap successful plan output; reject known model/plan-format mismatches before requesting generation. |
| Voice design | Sent model and streaming flags to `create-previews`, which accepts neither. | Use `/v1/text-to-voice/design`, whose request and preview response match the CLI. |
| Shared voices | Sent page 1 by default, skipping page 0. | Zero-based pagination. |
| Voice search | Similarity filters and v2 `show_legacy` were unsupported, silently ignored fields. | Remove those flags; demographic filtering remains on `voices library`. |
| Agent creation | Live catalog marks `gemini-3.1-flash-lite-preview` deprecated and identifies `gemini-3.1-flash-lite` as its replacement. | Use the catalog's stable replacement. |
| Agent ASR | Hardcoded deprecated `elevenlabs` provider and a deprecated TTS latency setting. | Use `scribe_realtime`; omit the ineffective latency setting. |

The maintainer's empirically selected `turn_v2` remains the CLI default; both
turn detector versions are now documented in the schema. No existing agents
are modified by these scaffold changes.

## Sources

- [Live OpenAPI schema](https://api.elevenlabs.io/openapi.json)
- [Official Python SDK](https://github.com/elevenlabs/elevenlabs-python/tree/06db161cd6038b0d2d42e4d55516db3ab42de924/src/elevenlabs)
- [Scribe v1 retirement](https://elevenlabs.io/docs/changelog/2026/6/8)
- [Scribe v2 Medical](https://elevenlabs.io/docs/changelog/2026/9/11)
- [Music models and composition plans](https://elevenlabs.io/docs/eleven-api/guides/how-to/music/composition-plans)
- [Upload extraction model IDs](https://elevenlabs.io/docs/api-reference/music/upload)
- Live `elevenlabs agents llms` response, retrieved 2026-09-21.

## Scope

This is an update to existing command behavior, not full API coverage. New
surfaces such as Dubbing v2 project workflows and Exotel calls need separate
feature work. Existing legacy dubbing endpoints remain available.

Live smoke checks passed for default Music v2.5 composition-plan generation
(returned `chunks`) and Scribe v2 Medical with no-verbatim mode on locally
synthesized speech (returned the expected sentence). Authentication and voice/model
discovery also passed. No agent was created and no outbound call was placed.

HTTP contract regressions cover changed requests and validation. Local checks
passed: formatting, Clippy with warnings denied, 200 tests across 37 suites,
release build, and JSON agent-info validation. Mock tests do not
establish audio quality or live outbound-call behavior.
