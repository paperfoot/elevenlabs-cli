# CLI Snippets — Worker Handoff Protocol

Each worker writes their clap additions and dispatch glue here. The lead integrates all snippets into `src/cli.rs`, `src/main.rs`, `src/commands/mod.rs`, and `src/commands/agent_info.rs` in a single edit phase.

## Layout

```
plans/cli-snippets/
├── A/
│   ├── cli.rs            # clap Commands::Agents additions + AgentsAction enum changes
│   ├── dispatch.rs       # match arms for src/main.rs
│   ├── agent-info.json   # JSON fragments to append to agent_info.rs
│   ├── mod.txt           # `pub mod <name>;` lines (empty if none)
│   └── NOTES.md
├── B/
├── C/
...
```

## Snippet format

- **cli.rs** — valid Rust snippets that can be pasted into `src/cli.rs`. Reference existing imports; do not re-declare.
- **dispatch.rs** — match arms only, e.g.:
  ```rust
  Commands::Dialogue(args) => commands::dialogue::run(ctx, args).await?,
  ```
- **agent-info.json** — JSON object entries to merge into the `commands` map in `agent_info.rs`.
- **mod.txt** — one `pub mod <name>;` line per new top-level module.
- **NOTES.md** — free-form: what was built, what was tested, any deps added to Cargo.toml, open questions.

## Rules

1. Do NOT touch `src/cli.rs`, `src/main.rs`, `src/commands/mod.rs`, `src/commands/agent_info.rs`, `README.md`, `Cargo.toml` (except record dep needs in NOTES.md).
2. Do NOT commit — the lead commits after integration.
3. Keep snippets minimal — only what's needed to wire the feature.
