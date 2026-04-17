# AGENTS.md — Instructions for AI Contributors

This file is the standing brief for **any LLM or autonomous agent** that
opens this repository to read, modify, test, or ship code. Humans: read
[README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md) instead.

If you are an AI agent (Claude Code, Codex, Gemini, Aider, Cursor, Cline,
OpenHands, etc.), **read this file first** and treat its instructions as
hard constraints.

---

## Where to send changes

**Always publish changes back upstream.** Do not leave fixes in a local
checkout — future agents and humans have no way to benefit from them.

| Artifact | Destination |
|---|---|
| Code changes | **Pull requests** against `main` on <https://github.com/paperfoot/elevenlabs-cli> |
| Bug reports | **Issues** on <https://github.com/paperfoot/elevenlabs-cli/issues> |
| Feature proposals | Issues first, PR after discussion |
| Security issues | Private: `security@199bio.com` (do **not** open a public issue) |
| Homebrew formula updates | PR against <https://github.com/199-biotechnologies/homebrew-tap> (`Formula/elevenlabs.rb`) |
| New crates.io version | Tag `vX.Y.Z` on `main`, the release workflow cuts prebuilt binaries; `cargo publish` from a clean checkout |

**PR etiquette for AI contributors:**

1. Open the PR against `main`. Never force-push to `main` directly.
2. Title format: `<area>: <one-line summary>` — e.g. `tts: stream response bytes as they arrive`.
3. Body must include: *what* changed, *why*, *how you verified it works*
   (tests run, endpoints hit). Link the issue if one exists.
4. Identify yourself in the PR body: `Contributed by [agent name + model]`.
   Example: *"Contributed by Claude Code (claude-opus-4-6)"*.
5. If a pre-commit hook fails, **fix the underlying issue**. Never bypass
   hooks with `--no-verify` or `--no-gpg-sign`.
6. Never amend commits after a hook failure — create a NEW commit.
7. One logical change per PR. Don't bundle unrelated fixes.

---

## Framework invariants (non-negotiable)

This CLI implements the
[**199-biotechnologies/agent-cli-framework**](https://github.com/199-biotechnologies/agent-cli-framework)
patterns. **These are contracts, not suggestions.** If a change violates
any of them, it gets rejected.

1. **Every code path that writes to stdout respects the output format.**
   JSON envelope when `--json` is set or stdout is not a TTY; coloured
   human output otherwise. No raw text ever leaks into a piped stream.

2. **`--help` and `--version` exit 0.** Even when piped. Wrap help in
   the success envelope when not a TTY.

3. **`agent-info` matches reality.** Every command listed must be
   routable. Every flag described must work. If you add a command, add
   it to `src/commands/agent_info.rs` *and* add an integration test in
   `tests/agent_info_contract.rs` that verifies it routes.

4. **Errors include actionable suggestions.** Every error envelope has
   a `suggestion` field containing a concrete executable instruction.
   *"Try running with elevated permissions"* is not acceptable — be
   specific. *"Set your API key: elevenlabs config init --api-key sk_…"*
   is acceptable.

5. **Exit codes are semantic and exhaustive:**
   | Code | Meaning | Agent action |
   |---|---|---|
   | 0 | Success | continue |
   | 1 | Transient (IO, network, 5xx) | retry with backoff |
   | 2 | Config / auth error | fix setup, don't retry |
   | 3 | Bad input (argv, flags) | fix args |
   | 4 | Rate limited | wait and retry |
   Do not invent new exit codes. If your error doesn't fit, map it to
   one of these five.

6. **JSON on stdout, errors on stderr.** A pipeline like
   `elevenlabs voices list | jq` must never see error text on stdout.
   Errors (both JSON envelope and human format) always go to stderr.

7. **No interactive prompts.** Never read from stdin unless the user
   explicitly passes `-` as a positional arg (e.g. `tts - < file.txt`).
   Never open a pager. Destructive operations take `--confirm` as a
   flag, not an interactive prompt.

8. **Secrets are never logged or echoed.** Use `config::mask_secret()`
   for any display. Config files are written with `0600` permissions
   on Unix. Never include raw secrets in error messages or JSON output.

9. **Config precedence: CLI flags > env vars > config file > defaults.**
   `ELEVENLABS_API_KEY` overrides `api_key` in `config.toml`. Verified
   by `tests/config_precedence.rs` — do not regress this.

10. **No MCP server, no protocol layer, no drift.** This CLI is
    deliberately self-contained. Do not add an MCP server mode, a
    long-running daemon, or a separate "agent wire protocol". The
    binary *is* the interface.

---

## Repository layout

```
elevenlabs-cli/
├── Cargo.toml                # crate name = elevenlabs, binary = elevenlabs
├── src/
│   ├── main.rs               # clap parse → Tokio runtime → dispatch
│   ├── cli.rs                # every clap struct + enum (single source of truth)
│   ├── client.rs             # reqwest wrapper, xi-api-key, status mapping
│   ├── error.rs              # AppError + exit_code/error_code/suggestion
│   ├── output.rs             # Format detection, Ctx, envelope helpers
│   ├── config.rs             # figment 3-tier + mask_secret + save
│   └── commands/
│       ├── mod.rs            # shared helpers (resolve_output_path, read_file_bytes)
│       ├── agent_info.rs     # the manifest — keep it in sync with cli.rs
│       ├── skill.rs          # self-install to Claude/Codex/Gemini
│       ├── config.rs         # config show/path/set/check/init
│       ├── update.rs         # self-update via self_update crate
│       ├── tts.rs stt.rs sfx.rs       # speech synthesis / transcription / SFX
│       ├── voices.rs         # list/show/search/library/clone/design/save-preview/delete
│       ├── models.rs         # list
│       ├── audio.rs          # isolate / convert (speech-to-speech)
│       ├── music.rs          # compose / plan
│       ├── user.rs           # info / subscription
│       ├── agents.rs         # list/show/create/delete/add-knowledge
│       ├── conversations.rs  # list/show
│       ├── phone.rs          # list/call
│       └── history.rs        # list/delete
├── tests/
│   ├── agent_info_contract.rs    # manifest ↔ reality invariants
│   ├── exit_code_contracts.rs    # semantic exit codes 0-4
│   ├── output_contracts.rs       # JSON envelope shape + stdout/stderr split
│   └── config_precedence.rs      # env > config precedence ladder
└── .github/workflows/
    ├── ci.yml                # fmt + clippy + test on linux/mac/windows
    └── release.yml           # build prebuilt binaries on tag push
```

---

## Build, test, lint — before every PR

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
./target/release/elevenlabs agent-info | jq .
```

All four commands must pass without errors. CI runs the same set on
Linux, macOS, and Windows; if your change is platform-specific, you are
responsible for verifying all three.

When adding dependencies, prefer crates already in the tree. If you
must add a new one, justify it in the PR body — this CLI has a hard
preference for a small dependency surface. See the `Cargo.toml` for
the allowed stack.

---

## Adding a new command (the one-page recipe)

1. **Define clap args** in `src/cli.rs`:
   - Add a variant to `Commands` with `#[command(visible_alias = "…")]`
     if there's a natural short form.
   - Subcommand groups (like `voices`) get their own `Subcommand` enum
     in the same file.

2. **Create `src/commands/<name>.rs`** with an `async fn run(ctx, args)`
   or `async fn dispatch(ctx, action)` — mirror whichever style the
   adjacent modules use.

3. **Register the module** in `src/commands/mod.rs` and the dispatch
   match in `src/main.rs`.

4. **Call the ElevenLabs API** through `ElevenLabsClient` methods from
   `src/client.rs`. Do not instantiate `reqwest::Client` directly.

5. **Emit output** via `output::print_success_or(ctx, &data, |d| {
   human_closure })` so JSON and human output stay consistent.

6. **Map errors** into `AppError` variants. If you introduce a new
   error shape, update `AppError::exit_code`, `error_code`, and
   `suggestion` in lockstep.

7. **Update `src/commands/agent_info.rs`** to list the new command.
   This is the P0 contract — agents bootstrap from this file.

8. **Add a test** in `tests/agent_info_contract.rs` proving the new
   command is routable.

9. **Run the build/test/lint ladder** above. Push the PR.

---

## Things that have bitten past agents (learn from the scars)

These are real bugs found during v0.1.0 verification. Don't recreate them:

- **Silent voice resolution**: `--voice NAME` used to fall through to
  the first voice in the response when there was no exact match. Fix:
  client-side exact > prefix > substring match, and **error out if no
  match** — never silently pick a wrong voice. See `src/commands/tts.rs`
  and `src/commands/audio.rs`.

- **Wrong HTTP paths**: music endpoints at `/v1/music/compose` and
  `/v1/music/plans/compose` don't exist. Correct paths are `/v1/music`
  and `/v1/music/plan`. If you're guessing API paths, **verify against
  the official [elevenlabs-python](https://github.com/elevenlabs/elevenlabs-python)
  SDK** before shipping.

- **clap field name vs flag name**: `#[arg(long, name = "loop")]` does
  *not* rename the flag — `name` is the argument ID. Use
  `#[arg(long = "loop")]` to change the actual `--loop` form.

- **Config precedence regression**: `resolve_api_key` used to prefer
  config file over env var, silently contradicting the README. Now
  locked in by `tests/config_precedence.rs`. If you touch
  `src/config.rs`, **run those tests** before pushing.

- **Exit-code leaks from clap**: using `clap::Error::exit()` lets clap
  own the exit code. Always go through the pre-scan in `main.rs` →
  `output::print_clap_error` → `std::process::exit(3)` so we own the
  contract.

- **Dead features in `agent-info`**: listing a command in the manifest
  without wiring it into dispatch is a P0 bug. Tests in
  `tests/agent_info_contract.rs` verify routability — if you ever see
  one fail, **do not mark it `#[ignore]`** — fix the wiring.

---

## Scope boundary

This CLI covers the ElevenLabs HTTP API. It does **not**:

- Run a long-lived daemon, server, or MCP process.
- Replace the official [`@elevenlabs/cli`](https://github.com/elevenlabs/cli)
  which does config-as-code GitOps for agents (push/pull local files).
  The two are complementary.
- Ship a GUI, TUI, or REPL mode.
- Depend on Python, Node, or any non-Rust runtime.

If a feature request crosses any of these lines, open an issue for
discussion before writing code.

---

## How to reach a human

- Open an issue at <https://github.com/paperfoot/elevenlabs-cli/issues>.
- For security disclosures: `security@199bio.com`.
- The maintainer account is `@longevityboris` on GitHub and X.

Thank you for contributing responsibly. Always send work back upstream.
