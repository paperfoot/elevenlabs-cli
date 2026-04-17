# Contributing to elevenlabs-cli

Thank you for your interest. This file is for **human contributors**.
If you're an AI agent (Claude, Codex, Gemini, Cursor, etc.), read
[AGENTS.md](AGENTS.md) instead — the rules there are more explicit.

---

## TL;DR

```bash
git clone https://github.com/paperfoot/elevenlabs-cli
cd elevenlabs-cli
cargo build --release
cargo test
./target/release/elevenlabs agent-info | jq .
```

- Open **issues** at <https://github.com/paperfoot/elevenlabs-cli/issues>
  for bugs, feature requests, and questions.
- Open **pull requests** against `main` with a clear title and a
  body that describes *what* changed and *why*.
- **Security issues**: email `security@199bio.com` — do not open a public
  issue.

---

## Before you open a PR

Please run these locally. CI runs the same set on Linux / macOS / Windows:

```bash
cargo fmt --all -- --check       # formatting
cargo clippy --all-targets -- -D warnings   # lint
cargo test                       # 26 integration tests
cargo build --release            # ensure release profile still builds
```

---

## Design principles (the short version)

The CLI follows the
[**agent-cli-framework**](https://github.com/199-biotechnologies/agent-cli-framework)
philosophy: it's built to be callable by both humans and AI agents, with a
stable JSON envelope, semantic exit codes, and a self-describing
manifest (`elevenlabs agent-info`).

The full list of invariants lives in [AGENTS.md](AGENTS.md#framework-invariants-non-negotiable).
The short version:

1. stdout is always parseable (JSON envelope when piped / `--json`).
2. Errors go to stderr and always include a machine-readable `code` and
   an actionable `suggestion`.
3. Exit codes are semantic: `0=ok, 1=transient, 2=config/auth, 3=bad
   input, 4=rate limited`.
4. Every command listed in `agent-info` must actually route.
5. No interactive prompts — every flag has an env/config fallback.
6. Secrets are masked on display and written with `0600` permissions.
7. Config precedence: **CLI flags > env vars > config file > defaults.**

---

## Commit style

- Keep commits focused — one logical change per commit.
- Title: `<area>: <summary>` (e.g. `tts: accept `-` to read from stdin`).
- Body: explain *why*, not just *what*. Link the issue if one exists.
- Sign your commits if you can.

## PR checklist

- [ ] Tests pass locally (`cargo test`)
- [ ] Lint passes locally (`cargo clippy -- -D warnings`)
- [ ] Formatting passes locally (`cargo fmt --all -- --check`)
- [ ] If you added a command, it's listed in `src/commands/agent_info.rs`
- [ ] If you added a command, there's a test in `tests/agent_info_contract.rs`
- [ ] No new dependencies without justification in the PR body
- [ ] No secrets, tokens, or API keys committed

---

## License

By contributing you agree that your contributions will be licensed under
the MIT license, the same as the rest of the project.
