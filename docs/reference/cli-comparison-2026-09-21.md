# ElevenLabs tooling comparison — 2026-09-21

Paperfoot v0.3.3 has useful advantages for shell automation, but is not more
complete than the official CLI. The earlier API refresh checked existing
commands; it was not an ecosystem comparison or a full API conformance audit.

## Versions and evidence

- **Paperfoot:** released v0.3.3, commit `b21ccf8ccedfdffc13d37ff75ef6a6f758c7e805`,
  installed Homebrew arm64 macOS binary.
- **Official CLI:** [v1.3.2](https://github.com/elevenlabs/cli/releases/tag/v1.3.2),
  published September 19, commit `3866acd25af5555a045830e8baaa7c94c88e841e`.
  Downloaded the arm64 macOS release and verified its published SHA-256.
  Executed it from a temporary directory without changing PATH or the installed CLI.
- **Official MCP:** reviewed the current hosted service documentation and the
  archived local server; no authenticated hosted MCP session was tested.
- **Community alternative:** source and local MCP handshake check of
  [hongkongkiwi/elevenlabs-cli](https://github.com/hongkongkiwi/elevenlabs-cli/tree/9bedd65ffea70357f1d32a1b438195a67a838da4)
  at commit `9bedd65ffea70357f1d32a1b438195a67a838da4`.

## Official tools have changed

ElevenLabs [launched CLI v1 on August 24, 2026](https://elevenlabs.io/blog/elevenlabs-cli-v1).
It now combines generated API commands with agent configuration workflows.
Its published binary is Rust, and provides scoped, typed `--schema` discovery,
request previews with `--dry-run`, and agent push/pull. These are no longer
capabilities that can be dismissed as absent from a GitOps-only tool.

The [v1.3.2 reference](https://github.com/elevenlabs/cli/blob/v1.3.2/README.md)
also documents JSON request bodies, additional parameters, automatic pagination,
regional endpoints, shell completions, generated skills, and `say` with audio
playback. We verified discovery and a TTS dry-run locally, not every API command.

The old Python MCP was [deprecated in August](https://elevenlabs.io/docs/changelog/2026/8/22).
The replacement [hosted MCP](https://elevenlabs.io/docs/eleven-agents/operate/hosted-mcp)
uses OAuth and workspace permissions, with no local server or copied API key.
Its [current product page](https://elevenlabs.io/mcp) also describes creative
generation including voice, music, images, and video. Hosted MCP capability
descriptions are documentation evidence, not results from a live account test.

## Measured binary behavior

Both binaries ran with piped output, stdin closed, an empty temporary home,
no inherited credentials, and an isolated working directory. All discovery
checks succeeded without API access. Sizes are bytes, not model tokens.

| Check | Paperfoot 0.3.3 | Official 1.3.2 |
|---|---:|---:|
| macOS arm64 executable bytes | 5,739,312 | 16,436,864 |
| Full discovery entries | 96 command entries | 387 API operations |
| Full discovery output bytes | 23,402 | 45,238 |
| TTS discovery output bytes | 6,040 | 9,944 |
| Music compose discovery output bytes | 5,488 | 6,538 |
| TTS JSON compacted identically | 6,040 | 8,205 |
| Music JSON compacted identically | 5,488 | 5,039 |
| Median warm TTS discovery, 25 samples | 12.174 ms | 100.442 ms |
| Invalid command / missing argument exit | 3 / 3 | 3 / 3 |
| Error stream in those two checks | JSON on stderr; stdout empty | JSON on stdout; stderr empty |
| Default piped help / version | JSON; exit 0 | Text; exit 0 |
| TTS request preview without credentials | No general dry-run | JSON preview; exit 0 |

The discovery counts are not interchangeable: our manifest includes local
commands and convenience workflows; the official index lists API operations.
The current vendored API has 391 operations. Neither command count establishes
an exact coverage percentage or full API compatibility.

The official TTS schema contains typed request properties and required fields.
Our manifest contains shorter usage descriptions, defaults, and selected
gotchas, while retaining global metadata even for one command. Smaller output
does not establish equivalent information or fewer agent mistakes. In the
music case, compacting both JSON documents makes the official schema smaller.

Timing covers warm process startup plus local discovery on this Mac, not API
latency, generation speed, cold start, or agent task success. Byte counts can
vary with the config path embedded in our manifest. No tokenizer or full agent
task benchmark was run. The earlier 76% reduction compared our scoped TTS
discovery with our own old full manifest, not with an official CLI or MCP.

## Other community CLI

The community alternative's README advertises audio commands, streaming,
completions, and an optional MCP mode. Building with `--features mcp` succeeded.
In an isolated local run, MCP initialization succeeded but `tools/list` returned
`{"tools":[]}`; five non-JSON log lines also appeared on protocol stdout.
The [server implementation](https://github.com/hongkongkiwi/elevenlabs-cli/blob/9bedd65ffea70357f1d32a1b438195a67a838da4/src/mcp/server.rs#L209-L220)
implements server information without connecting the tool dispatcher to the
protocol handlers. This result applies to the tested checkout and build; its
ordinary CLI commands were not comprehensively tested.

## Gaps worth addressing in Paperfoot

Follow-up work is tracked in [issue #15](https://github.com/paperfoot/elevenlabs-cli/issues/15).

1. **Discovery accuracy and maintenance:** add typed required inputs, complete
   flags, and command-specific output contracts; automatically compare declared
   routes/fields with the API snapshot. The refresh script currently inventories
   paths, not full request/response conformance. Source: `agent_info.rs` and
   `docs/reference/refresh.sh`.
2. **Request previews and consistent confirmation:** no general dry-run exists.
   Agent deletion requires `--yes`, but history deletion has no equivalent
   gate. The contributor instructions say `--confirm`; implementation and
   guidance need a deliberate compatibility decision, not a silent rename.
3. **Coverage:** important missing workflows include current Dubbing v2,
   agent branches/testing, workspace administration, Studio, and professional
   voice cloning. The existing [API audit](api-compatibility-2026-09-21.md)
   already records incomplete coverage. Agent GitOps is outside this project's
   scope; the official CLI supplies it.
4. **Streaming:** TTS `--stream` selects the streaming endpoint but buffers
   `resp.bytes()` before writing audio. It does not establish incremental
   playback or bounded memory use for long responses. Source:
   `src/commands/tts.rs` and `src/client.rs`.
5. **Contract consistency:** API keys deliberately use saved-file precedence
   in code and regression tests, but AGENTS.md/CONTRIBUTING.md still say the
   environment wins. Some recovery suggestions are prose rather than commands.
   Raw audio `--stdout` is another intentional exception to the JSON envelope.
   The README now describes the observed behavior; runtime compatibility is
   unchanged by this documentation audit.

## Reproduce and validation

Download the two pinned release binaries and verify their checksums before
running them. Keep separate absolute paths because both are named `elevenlabs`.

```bash
python3 docs/reference/compare-cli-discovery.py \
  /absolute/path/to/paperfoot/elevenlabs \
  /absolute/path/to/official/elevenlabs > comparison.json
```

The [recorded JSON](cli-comparison-2026-09-21.json) includes binary hashes,
versions, environment details, return codes, output sizes, and timings. The
script uses Python's standard library as a maintainer utility; the shipped CLI
still has no Python dependency. It runs discovery, invalid-input checks, and
one official dry-run. It performs no authenticated generation or deletion.

Paperfoot validation on the comparison branch: formatting, Clippy with warnings
denied, 212 tests across 37 suites, release build, and parsed v0.3.3 discovery
all passed. Previous live API checks are recorded in the compatibility audit.
This report and README correction do not add the missing capabilities above.
