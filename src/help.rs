//! After-help Tips + Examples text attached to individual commands via clap's
//! `after_long_help`. These constants are displayed below the normal flag
//! reference when the user runs `--help`; they are the surface we rely on to
//! teach agents how to actually *use* a command (vs. just listing flags).
//!
//! Style rules:
//! 1. `TIPS` — non-obvious insights (flag combos, pitfalls, defaults).
//!    Anything already in the arg description is omitted; duplication is
//!    noise.
//! 2. `EXAMPLES` — copy-pasteable invocations. Every example must be a
//!    complete shell command starting with `$ elevenlabs …`.
//! 3. Keep each constant under ~40 lines. Agents read these in the
//!    context window; verbosity costs real tokens.
//! 4. Reinforce framework invariants: JSON-on-pipe, semantic exit codes,
//!    `doctor` for setup, `--json` to force JSON on a TTY.

// ── TTS ────────────────────────────────────────────────────────────────────

pub const TTS_HELP: &str = "TIPS
 - Default output is ./tts_<timestamp>.mp3 — pass --output path.mp3 if you
   want a stable name; pass --stdout to pipe audio bytes (implies --quiet).
 - Prefer --voice NAME over --voice-id; a miss errors out (exit 3) instead
   of silently picking the first voice in your library.
 - --format uses codec_samplerate_bitrate, e.g. mp3_44100_128, pcm_44100,
   ulaw_8000. The bitrate segment is only valid for mp3.
 - --stream begins writing audio as it arrives (lowest time-to-first-byte).
   --with-timestamps returns per-character alignment and is mutually
   useful with streaming for live karaoke; --save-timestamps <path.json>
   captures the alignment JSON.
 - Splitting long text? Pass --previous-text / --next-text (or the
   request-id variants) to preserve prosody across boundaries.

EXAMPLES
 # Single line with a named voice
 $ elevenlabs tts \"Welcome to the show.\" --voice Rachel --output hello.mp3

 # Low-latency streaming for live playback
 $ elevenlabs tts \"Ping\" --voice Rachel --stream -o ping.mp3

 # Karaoke: audio + per-character alignment
 $ elevenlabs tts \"$(cat script.txt)\" --voice Rachel \\
     --with-timestamps --save-timestamps script.align.json \\
     --output script.mp3

 # Pipe raw audio into ffmpeg (--stdout implies --quiet)
 $ elevenlabs tts \"Ambient\" --voice Rachel --stdout --format pcm_44100 \\
     | ffmpeg -f s16le -ar 44100 -ac 1 -i - -y out.wav";

// ── STT ────────────────────────────────────────────────────────────────────

pub const STT_HELP: &str = "TIPS
 - scribe_v2 (default) is slower but noticeably more accurate on noisy or
   overlapping speech. Use --model scribe_v1 only for legacy parity.
 - --from-url ingests S3/GCS/R2/CDN/pre-signed HTTPS URLs without a local
   download; --source-url ingests hosted video (YouTube, TikTok, …).
   These and <FILE> are mutually exclusive.
 - Diarization needs --diarize; pair with --num-speakers N (or
   --diarization-threshold) and --detect-speaker-roles to label agent vs
   customer on 2-party calls.
 - --timestamps character gives sub-word granularity for karaoke / lyric
   sync. --save-words captures only the word array for subtitle
   pipelines.
 - --format srt|txt|segmented_json|docx|pdf|html triggers multi-format
   export; combine with --format-include-speakers and
   --format-include-timestamps. --format-out-dir <dir> controls where
   they land.

EXAMPLES
 # Plain transcript from a local file
 $ elevenlabs stt call.m4a -o call.txt

 # Subtitles straight from a YouTube link
 $ elevenlabs stt --source-url 'https://youtu.be/...' \\
     --format srt --format-out-dir subs/

 # Two-party call: diarize + label agent/customer
 $ elevenlabs stt call.wav --diarize --num-speakers 2 \\
     --detect-speaker-roles --format docx -o call.json

 # Machine pipeline — JSON envelope on stdout, exit-code check
 $ elevenlabs stt podcast.mp3 | jq '.data.words[0]'; echo \"exit $?\"";

// ── SFX ────────────────────────────────────────────────────────────────────

pub const SFX_HELP: &str = "TIPS
 - Duration is 0.5-30 seconds. Omit --duration to let the model choose
   a length matched to the prompt.
 - --prompt-influence tops out at 1.0 (literal) and bottoms at 0.0
   (interpretive). 0.3 is the server default; push higher for onomatopoeia.
 - --loop requires model eleven_text_to_sound_v2 — the default model does
   not loop seamlessly.
 - For exact loop points, compose at pcm_44100 and re-encode downstream.

EXAMPLES
 # Short cinematic hit
 $ elevenlabs sfx \"Door slam, wooden, heavy\" --duration 1.2 -o slam.mp3

 # Seamless ambient loop (requires v2)
 $ elevenlabs sfx \"Rain on a tin roof\" --loop \\
     --model eleven_text_to_sound_v2 --duration 8 -o rain.mp3

 # High prompt fidelity
 $ elevenlabs sfx \"Glass shatters into tiny shards\" \\
     --prompt-influence 0.8 -o shatter.mp3";

// ── Voices ─────────────────────────────────────────────────────────────────

pub const VOICES_LIBRARY_HELP: &str = "TIPS
 - `voices library` searches the public shared library; `voices list`
   only returns voices already attached to *your* account.
 - Filters stack: --category, --gender, --age, --accent, --language,
   --locale, --use-case. All are optional; leaving them off sorts by
   popularity.
 - --sort cloned_by_count surfaces actually-useful voices; the default
   sort is featured-first, which is often just paid placement.
 - --page starts at 1, --page-size caps at 100. Pagination is cursorless
   — use --page to walk forward.

EXAMPLES
 # British male audiobook narrators, most-cloned first
 $ elevenlabs voices library --gender male --accent british \\
     --use-case audiobook --sort cloned_by_count

 # Professional-tier Spanish voices, 50 per page
 $ elevenlabs voices library --category professional --language es \\
     --page-size 50

 # Machine-pipe: extract every voice_id on page 1
 $ elevenlabs voices library --sort cloned_by_count \\
     | jq -r '.data.voices[].voice_id'";

pub const VOICES_DESIGN_HELP: &str = "TIPS
 - Two models: eleven_ttv_v3 (newer, more expressive, English-leaning) vs
   eleven_multilingual_ttv_v2 (broader language coverage, more stable).
   If unsure, start with eleven_ttv_v3.
 - Design is a TWO-STEP workflow: (1) `voices design` returns preview
   files + a generated_voice_id; (2) `voices save-preview
   <generated_voice_id> <name> <description>` attaches it to your library.
   Without step 2 the voice disappears.
 - --stream-previews returns IDs only (no preview audio); pair with the
   REST streaming endpoint for UI playback. Skip for headless use.
 - --guidance-scale too high (>~4) produces robotic delivery. Start near
   1.5-3.0.
 - --text 100-1000 chars overrides the auto-generated preview script
   when you need a specific sample read.

EXAMPLES
 # Design a v3 voice, save to ./previews/
 $ elevenlabs voices design \"Raspy aging detective, worn but kind\" \\
     --model eleven_ttv_v3 --output-dir previews/

 # Step 2: keep one of the previews
 $ elevenlabs voices save-preview gv_abcd1234 \"Detective\" \\
     \"Noir protagonist for the audiobook series\"

 # Reproducible design with a seed
 $ elevenlabs voices design \"Cheerful radio host, morning energy\" \\
     --seed 42 --guidance-scale 2.0 --model eleven_ttv_v3";

// ── Dialogue ───────────────────────────────────────────────────────────────

pub const DIALOGUE_HELP: &str = "TIPS
 - Two input modes: (1) positional colon-triples `label:voice_id:text`
   (quote each triple to survive shell splitting); (2) --input <file.json>
   with an array of `{text, voice_id}` entries. A single positional ending
   in `.json` is auto-detected as a file; pass `-` to read JSON from stdin.
 - Model defaults to eleven_v3 — the only one that actually does
   multi-speaker prosody well. Use --model only to opt *out*.
 - Up to 10 unique voice IDs per request; total text under ~2000 chars.
   The CLI pre-flights this before burning the API quota.
 - --stream + --with-timestamps combined routes to the NDJSON variant
   (stream + per-character alignment). Saves to --output and
   --save-timestamps respectively.

EXAMPLES
 # Inline two-speaker snippet
 $ elevenlabs dialogue \\
     \"Alice:21m00Tcm4TlvDq8ikWAM:Welcome back.\" \\
     \"Bob:AZnzlk1XvdvUeBnXmlld:Thanks for having me.\" \\
     -o intro.mp3

 # Longer dialogue from a JSON file
 $ elevenlabs dialogue script.json -o scene.mp3 \\
     --with-timestamps --save-timestamps scene.align.json

 # Stdin JSON pipe from an upstream tool
 $ my_dialogue_tool | elevenlabs dialogue - -o out.mp3";

// ── Align ──────────────────────────────────────────────────────────────────

pub const ALIGN_HELP: &str = "TIPS
 - Returns both word-level AND character-level timings plus a `loss`
   value — lower loss means the transcript more tightly fits the audio.
 - Prefer --transcript-file for anything non-trivial. Inline transcripts
   that contain colons, newlines, or look like paths will confuse the
   heuristic.
 - --enabled-spooled-file is required for very large uploads (>~50MB).
 - Use `align` when you already have a known transcript; if you need a
   transcript from scratch, use `stt` (forced alignment is cheaper than
   full STT but requires ground truth).

EXAMPLES
 # Inline transcript (short text only)
 $ elevenlabs align narration.mp3 \"Hello world.\" -o timings.json

 # Real transcript via file
 $ elevenlabs align narration.mp3 --transcript-file script.txt \\
     -o timings.json

 # Large audio (>50MB) — spool and pipe for further processing
 $ elevenlabs align podcast.wav --transcript-file transcript.txt \\
     --enabled-spooled-file | jq '.data.words | length'";

// ── Music ──────────────────────────────────────────────────────────────────

pub const MUSIC_COMPOSE_HELP: &str = "TIPS
 - --length-ms is milliseconds, not seconds. 3000-600000 (3s-10min).
 - Compose workflow has two shapes:
   (1) One-shot: pass a PROMPT (and optionally --length-ms, --force-
       instrumental); the model invents a composition plan internally.
   (2) Plan-first: run `elevenlabs music plan \"...\" > plan.json`,
       edit the plan (sections, styles, lengths), then
       `elevenlabs music compose --composition-plan plan.json`. Plan
       generation is free; composition is billed.
 - --composition-plan is mutually exclusive with PROMPT, --length-ms,
   and --force-instrumental. Override per-section durations from the
   plan with --respect-sections-durations.
 - --sign-with-c2pa embeds provenance metadata in the mp3 — enable for
   anything that will ship publicly.

EXAMPLES
 # Quick instrumental
 $ elevenlabs music compose \"80s synthwave, driving, nostalgic\" \\
     --length-ms 30000 --force-instrumental -o drive.mp3

 # Plan-first workflow
 $ elevenlabs music plan \"Ambient forest, 2min, gentle build\" \\
     --length-ms 120000 > plan.json
 $ elevenlabs music compose --composition-plan plan.json \\
     --respect-sections-durations -o forest.mp3

 # Higher sample rate + C2PA signature for distribution
 $ elevenlabs music compose \"Lo-fi hip-hop loop\" --length-ms 60000 \\
     --format mp3_44100_192 --sign-with-c2pa -o track.mp3";

// ── Agents ─────────────────────────────────────────────────────────────────

pub const AGENTS_CREATE_HELP: &str = "TIPS
 - --system-prompt is REQUIRED — there is no interactive fallback. Keep
   prompts specific; vague prompts yield vague agents.
 - Defaults: --llm gemini-3.1-flash-lite-preview (cheap, fast, good
   enough for most IVR flows), --model-id eleven_flash_v2_5 (lowest
   TTS latency), --max-duration-seconds 300 (5 min — bump it for
   anything longer than a voicemail). Override --llm for reasoning-
   heavy use cases; override --model-id for higher-fidelity voice.
 - Valid --model-id values (server-enforced allowlist):
   eleven_turbo_v2, eleven_turbo_v2_5, eleven_flash_v2, eleven_flash_v2_5,
   eleven_multilingual_v2, eleven_v3_conversational. Passing `eleven_v3`
   is a common mistake — that's the dialogue/ttv model and agents reject
   it with \"English Agents must use turbo or flash v2\". For expressive
   v3 realtime use eleven_v3_conversational.
 - --expressive-mode only takes effect with
   --model-id eleven_v3_conversational; the server silently drops it on
   every other model. Pass --expressive-mode on its own and the CLI
   auto-upgrades the model for you. Requires a tier that has access to
   expressive TTS (Creator+ at the time of writing).
 - --llm is free-form but the Agents backend enforces its own allowlist
   at conversation time. Known-failing example seen in the wild:
   gemini-3.1-pro-preview (0 output tokens on the agents path even
   though the LLM is fine elsewhere). If `conversations show` reports
   0 output tokens, swap LLMs.
 - Agent gets a voice from --voice-id, NOT --voice name — pin a
   voice_id you trust. Use `elevenlabs voices list` first.
 - --voicemail-detection is OFF by default because it only helps
   outbound phone agents. For ANY agent you'll dial out to, turn it on
   — otherwise the greeting gets recorded onto the callee's answer-
   phone. Pass --voicemail-message to leave a message instead of
   hanging up silently.
 - Attach knowledge AFTER create via `agents add-knowledge` — the
   document is PATCHed onto the agent config, not just uploaded.

EXAMPLES
 # Minimal agent
 $ elevenlabs agents create \"Support Bot\" \\
     --system-prompt \"You are the first-line triage assistant for ACME.\"

 # Long-form interviewer with expressive v3 realtime voice
 # + voicemail detection (hangs up on answerphones)
 $ elevenlabs agents create \"Legacy Interview\" \\
     --system-prompt \"$(cat prompts/interview.txt)\" \\
     --voice-id JBFqnCBsd6RMkjVDRZzb \\
     --expressive-mode --max-duration-seconds 1800 \\
     --voicemail-detection

 # Pinned voice + multilingual TTS
 $ elevenlabs agents create \"Research Assistant\" \\
     --system-prompt \"$(cat prompts/ra.txt)\" \\
     --voice-id 21m00Tcm4TlvDq8ikWAM \\
     --llm gemini-3.1-flash-preview --model-id eleven_multilingual_v2

 # Attach docs after create
 $ AGENT=$(elevenlabs agents create ... --json | jq -r '.data.agent_id')
 $ elevenlabs agents add-knowledge \"$AGENT\" \"policy.pdf\" \\
     --file docs/policy.pdf";

pub const AGENTS_UPDATE_HELP: &str = "TIPS
 - --patch JSON is forwarded to PATCH /v1/convai/agents/{id} verbatim.
   Only include the fields you want to change; everything else stays.
 - Common paths inside conversation_config (the agent's runtime config):
     agent.prompt.prompt             — system prompt text
     agent.prompt.llm                — LLM id (backend has its own allowlist)
     agent.prompt.temperature        — 0.0-1.0
     agent.first_message             — what the agent says first
     tts.voice_id                    — bound voice
     tts.model_id                    — see allowlist below
     tts.expressive_mode             — true only with eleven_v3_conversational
     tts.stability / similarity_boost — voice settings (0.0-1.0)
     conversation.max_duration_seconds — hard call limit in seconds
     turn.turn_timeout               — silence before the agent prompts (s)
 - tts.model_id allowlist (server-enforced): eleven_turbo_v2,
   eleven_turbo_v2_5, eleven_flash_v2, eleven_flash_v2_5,
   eleven_multilingual_v2, eleven_v3_conversational. `eleven_v3` is
   the dialogue/ttv model and is NOT valid for agents — the CLI
   rejects it pre-flight (exit 3).
 - tts.expressive_mode=true is SILENTLY DROPPED unless tts.model_id is
   eleven_v3_conversational in the same patch (or the agent already has
   that value). The CLI pre-scans for this footgun and errors out
   before sending the patch.
 - turn.turn_model values: 'turn_v2' | 'turn_v3'. EMPIRICAL — enforced
   by the live API, absent from the current OpenAPI spec. Our scaffold
   uses turn_v2 (empirically stable in 2026-04); v3 was observed
   swallowing turn-ends on some LLM configs.
 - turn.turn_eagerness: 'patient' | 'normal' | 'eager' (default
   'normal'). 'patient' over-suppresses on speakerphones.
 - turn.mode: 'silence' | 'turn' (default 'turn'). turn.initial_wait_time,
   turn.silence_end_call_timeout, and turn.soft_timeout_config are all
   spec-backed knobs for tuning ringing/idle behaviour.
 - disable_first_message_interruptions=true prevents 'yes' / 'mm-hmm'
   backchannels from cutting off the greeting. It lives at
   conversation_config.AGENT.disable_first_message_interruptions
   (not .turn.*) — pre-v0.3.0 of this CLI had the wrong path and the
   server silently dropped it.
 - background_voice_detection=true filters speakerphone room noise. It
   lives at conversation_config.VAD.background_voice_detection (not
   .turn.*). Real calls showed it also mutes the user's own voice on
   speakerphones — leave false unless a test dial proves otherwise.
 - Every PATCH is non-destructive — use `agents show` to read current
   state, then PATCH the delta. Don't re-post the full object unless
   you mean to reset fields.

EXAMPLES
 # Swap to expressive v3 realtime (requires Creator+ tier)
 $ cat > patch.json <<'JSON'
 {\"conversation_config\":{\"tts\":{
    \"model_id\":\"eleven_v3_conversational\",
    \"expressive_mode\":true}}}
 JSON
 $ elevenlabs agents update agent_abc --patch patch.json

 # Extend call limit from 5 min to 30 min
 $ echo '{\"conversation_config\":{\"conversation\":{\"max_duration_seconds\":1800}}}' > p.json
 $ elevenlabs agents update agent_abc --patch p.json

 # Change just the LLM
 $ echo '{\"conversation_config\":{\"agent\":{\"prompt\":{\"llm\":\"gemini-3.1-flash-preview\"}}}}' > p.json
 $ elevenlabs agents update agent_abc --patch p.json

 # Rename the agent (top-level, not inside conversation_config)
 $ echo '{\"name\":\"New Display Name\"}' > p.json
 $ elevenlabs agents update agent_abc --patch p.json";

pub const AGENTS_ADD_KNOWLEDGE_HELP: &str = "TIPS
 - Exactly ONE of --url, --file, --text is required (they are mutually
   exclusive). Passing zero or more than one errors out (exit 3).
 - v0.2 fix: the uploaded document is now also attached to the agent's
   knowledge_base via PATCH. In v0.1 it was created but orphaned — agents
   never saw the doc. If you upgraded mid-project, re-add orphaned docs.
 - --url ingests anything publicly reachable (HTML, PDF, TXT). --file
   ingests a local path. --text is for short inline snippets.
 - The agent immediately starts citing the new doc — no redeploy needed.

EXAMPLES
 # Attach a PDF
 $ elevenlabs agents add-knowledge agent_abc \"Q4 Policy\" \\
     --file docs/q4-policy.pdf

 # Attach a live URL (auto-refetched on conversation start)
 $ elevenlabs agents add-knowledge agent_abc \"Pricing Page\" \\
     --url https://acme.com/pricing

 # Short inline snippet
 $ elevenlabs agents add-knowledge agent_abc \"Escalation\" \\
     --text \"Escalate to human if the user says 'supervisor'.\"";

// ── Phone ──────────────────────────────────────────────────────────────────

pub const PHONE_CALL_HELP: &str = "TIPS
 - The agent MUST exist before the call (`agents create` first). This
   command dispatches to Twilio vs SIP-trunk endpoints based on the
   provider field on --from-id, so the phone number record drives the
   routing — you don't pick the provider yourself.
 - --to must be E.164 (leading +, country code, no punctuation).
 - --dynamic-variables '{\"name\":\"Alex\"}' sets per-call {{name}} etc.
   in the agent prompt. Must be a JSON object. Prefix with `@` for
   file loading.
 - --client-data takes the FULL conversation_initiation_client_data
   object — use this when you need to override agent.first_message,
   tts.voice_id/stability/speed, agent.prompt.prompt/llm, or pass
   user_id / source_info / branch_id / environment on a per-call
   basis. If both flags are passed, --dynamic-variables is merged
   into the client-data object (it overwrites same-named entries).
 - --record flips call_recording_enabled on. --ringing-timeout-secs
   caps how long the callee's phone rings before the call is
   abandoned (telephony_call_config.ringing_timeout_secs).
 - The returned conversation_id shows up in `conversations list` once
   the call connects. Poll `conversations show <id>` for the transcript;
   `conversations audio <id> -o call.mp3` downloads the recording.
   If the call ended silent after one turn, inspect the transcript AND
   the llm_usage.model_usage block — 0 output tokens with a fallback
   model means the agent's --llm was rejected by the backend.

EXAMPLES
 # Place an outbound call
 $ elevenlabs phone call agent_abc --from-id phn_123 --to +14155551212

 # Pass dynamic context inline
 $ elevenlabs phone call agent_abc --from-id phn_123 --to +14155551212 \\
     --dynamic-variables '{\"customer\":\"Alex\",\"order_id\":\"A-8821\"}'

 # Override the voice + first message for one call only
 $ elevenlabs phone call agent_abc --from-id phn_123 --to +14155551212 \\
     --client-data '{
        \"conversation_config_override\": {
          \"tts\": { \"voice_id\": \"JBFqnCBsd6RMkjVDRZzb\" },
          \"agent\": { \"first_message\": \"Hi Alex, Ellis here.\" }
        },
        \"user_id\": \"user_abc\"
      }' \\
     --dynamic-variables '{\"customer\":\"Alex\"}'

 # Enable recording + cap ring time
 $ elevenlabs phone call agent_abc --from-id phn_123 --to +14155551212 \\
     --record --ringing-timeout-secs 25

 # Script: fire a call and tail its transcript + audio
 $ CID=$(elevenlabs phone call ... --json | jq -r '.data.response.conversation_id')
 $ elevenlabs conversations show \"$CID\" --json | jq '.data.transcript'
 $ elevenlabs conversations audio \"$CID\" -o call.mp3";

// ── Dubbing ────────────────────────────────────────────────────────────────

pub const DUBBING_CREATE_HELP: &str = "TIPS
 - Exactly one of --file (local media) or --source-url (publicly
   reachable) is required.
 - --target-lang is an ISO code (es, fr, de, ja, hi, …). --source-lang
   is optional — omit to auto-detect.
 - --dubbing-studio true enables Studio-mode, returning an *editable*
   resource. You then drive the dub via `dubbing resource {transcribe,
   translate,dub,render}` calls and fetch the final output via
   `dubbing get-audio`. Without --dubbing-studio the dub is one-shot.
 - --num-speakers helps the diarizer on multi-speaker sources.
   --drop-background-audio isolates voice; --disable-voice-cloning makes
   the dub use a default voice per speaker (useful when the source
   voice is low-quality).

EXAMPLES
 # One-shot dub to Spanish
 $ elevenlabs dubbing create --file interview.mp4 --target-lang es

 # Editable Studio-mode dub from a URL (returns editable resource ID)
 $ elevenlabs dubbing create --source-url https://cdn.example.com/talk.mp4 \\
     --target-lang de --dubbing-studio true --num-speakers 2

 # Fetch the dubbed output once status == dubbed
 $ elevenlabs dubbing show $DID
 $ elevenlabs dubbing get-audio $DID es -o interview_es.mp4";

// ── Dict (pronunciation dictionaries) ──────────────────────────────────────

pub const DICT_ADD_RULES_HELP: &str = "TIPS
 - Rule syntax splits on the FIRST `:` only — IPA phonemes containing
   colons (tone marks, ejectives, length) survive. Two rule shapes:
     --rule WORD:PHONEME          (IPA phoneme rule)
     --alias-rule WORD:ALIAS      (spoken-form alias)
 - Both flags repeat — pass many in one call rather than building the
   dictionary in multiple API round-trips.
 - Phoneme rules use the IPA alphabet (server-side default). Provide the
   raw IPA string — e.g. 'ɛləvən' for 'eleven'.
 - After create, attach the dictionary to TTS requests via the
   `pronunciation_dictionary_locators` field on your agent / voice
   config; it does NOT apply globally.

EXAMPLES
 # Two phoneme rules in one call
 $ elevenlabs dict add-rules \"Medical terms v1\" \\
     --rule 'aorta:eɪˈɔːrtə' \\
     --rule 'aneurysm:ˈænjərɪzəm'

 # Alias (spoken form) for a brand name
 $ elevenlabs dict add-rules \"Brand\" --alias-rule 'ACME:ack me'

 # Upload a full PLS lexicon from file
 $ elevenlabs dict add-file \"IPA Pack\" lexicon.pls --description \"v2\"";

// ── Doctor ─────────────────────────────────────────────────────────────────

pub const DOCTOR_HELP: &str = "TIPS
 - Run FIRST on a new machine or after any config change. Covers: config
   file presence, file-mode on config.toml (must be 0600 on Unix), API
   key reachability, network path to api.elevenlabs.io, and the
   env-shadow check — when ELEVENLABS_API_KEY in the environment differs
   from the one in config.toml, doctor flags it so agents don't silently
   auth as the wrong account.
 - Exit codes follow the framework contract: 0 all green; 2 config/auth
   issue; 1 transient network. Pipe into a CI step and gate deploys on
   `elevenlabs doctor && …`.
 - JSON mode emits a structured check-by-check report — use --json in
   automation so you can parse individual check results.

EXAMPLES
 # Interactive human report
 $ elevenlabs doctor

 # CI gate: abort if any check fails
 $ elevenlabs doctor || { echo 'elevenlabs misconfigured'; exit 2; }

 # Inspect individual checks
 $ elevenlabs doctor --json | jq '.data.checks[] | select(.status!=\"ok\")'";

// ── Config ─────────────────────────────────────────────────────────────────

pub const CONFIG_INIT_HELP: &str = "TIPS
 - Non-interactive: --api-key is required in headless environments
   (CI, containers). No stdin prompt is ever issued.
 - Writes config.toml with 0600 permissions on Unix. Do NOT commit it to
   git; it contains a secret.
 - Precedence: CLI flag > ELEVENLABS_API_KEY env var > config.toml >
   defaults. Re-running `config init` only rewrites the file, so an env
   var set in the shell will still win.
 - Follow up with `elevenlabs doctor` to confirm the key actually works
   and that no other env var is shadowing it.

EXAMPLES
 # First-time setup
 $ elevenlabs config init --api-key sk_your_key_here

 # Verify everything works
 $ elevenlabs doctor

 # Inspect the effective merged config (secrets masked)
 $ elevenlabs config show";

// ── Update ─────────────────────────────────────────────────────────────────

pub const UPDATE_HELP: &str = "TIPS
 - Fetches prebuilt binaries from GitHub Releases (paperfoot/elevenlabs-cli).
   Homebrew users should run `brew upgrade elevenlabs` instead — this
   command writes to the current binary path and won't touch Cellar.
 - --check reports without installing; exit 0 = up to date, exit 1 =
   update available.
 - After update, `elevenlabs --version` reflects the new version; agents
   should re-read `elevenlabs agent-info` to pick up any new commands.

EXAMPLES
 # Install the latest release
 $ elevenlabs update

 # CI check — fail if a newer version is out
 $ elevenlabs update --check || echo 'update pending'";

// ── Skill ──────────────────────────────────────────────────────────────────

pub const SKILL_INSTALL_HELP: &str = "TIPS
 - Installs the ElevenLabs skill markdown into each detected agent
   platform: Claude Code (~/.claude/skills/), Codex CLI, Gemini CLI.
   Platforms that aren't installed are skipped silently.
 - The skill file teaches AI agents to bootstrap via `elevenlabs
   agent-info` rather than guessing flags — install it once per
   machine/user account.
 - Run `skill status` to see which platforms currently have the skill.

EXAMPLES
 # Install everywhere detected
 $ elevenlabs skill install

 # Check coverage
 $ elevenlabs skill status";

// ── History / User (low priority) ──────────────────────────────────────────

pub const HISTORY_LIST_HELP: &str = "TIPS
 - --page-size caps at 1000. Paginate via --start-after <history_item_id>
   returned from the previous page; cursor-style, not offset.
 - --source TTS filters to synthesis jobs; STS filters speech-to-speech.
 - --search matches free text within history items (prompts, filenames).
 - Combine --voice-id and --model-id for per-voice usage audits.

EXAMPLES
 # Last 50 TTS jobs with a specific voice
 $ elevenlabs history list --page-size 50 --source TTS \\
     --voice-id 21m00Tcm4TlvDq8ikWAM

 # Walk pagination
 $ elevenlabs history list --json | jq -r '.data.history[].history_item_id'";

pub const USER_SUBSCRIPTION_HELP: &str = "TIPS
 - Check before running batch jobs — `character_count` vs
   `character_limit` tells you remaining quota for the billing cycle.
 - The `next_character_count_reset_unix` timestamp is when your quota
   refills — pipe through `date -r` to get a human time.
 - `voice_slots_used` / `voice_limit` matter before `voices clone` or
   `voices design save-preview`; designs/clones count against the slot
   cap.

EXAMPLES
 # Remaining characters this cycle
 $ elevenlabs user subscription --json \\
     | jq '.data | (.character_limit - .character_count)'

 # When does the quota reset?
 $ elevenlabs user subscription --json \\
     | jq -r '.data.next_character_count_reset_unix' | xargs -I{} date -r {}";
