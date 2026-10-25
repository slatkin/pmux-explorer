# Tasks

## 1. JSON-RPC framing

- [x] 1.1 Add `src/mcp/rpc.rs`: parse one JSON-RPC line, build replies and
  errors, and the shared line writer (`Arc<Mutex<..>>`, one line plus a
  flush per write; D1). Verify with unit tests: a request, a
  notification, a malformed line, and `-32601` for an unknown method.
- [x] 1.2 Add the `initialize` reply: only the channel capability, no
  `instructions`, and the revision rule (D2). Verify with unit tests:
  the capabilities object has one key and the reply has no
  `instructions`; `2026-07-28` → `2025-06-18`, `2025-03-26` →
  `2025-03-26`, an unknown value → `2025-06-18`; `tools/list` →
  `-32601`.

## 2. Endpoint path

- [x] 2.1 Add `src/mcp/path.rs`: FNV-1a 64 of the tmux socket path, the
  `XDG_RUNTIME_DIR` or `/tmp/pmux-channel-<uid>` directory, and the pane
  number (D4). It takes the env values as arguments, so tests set no
  process env. Verify with unit tests: the shared vector
  `/tmp/tmux-1000/default,1,0` + `%12` + `/run/user/1000` →
  `/run/user/1000/pmux-channel/79301cf9592f327a-12.sock`, the unset
  runtime dir, and a missing `TMUX` or `TMUX_PANE` → no path.

## 3. Endpoint socket

- [x] 3.1 Add `src/mcp/endpoint.rs`: the directory (0700, owner check), the
  bind with the stale-file and live-owner rules, and the socket mode 0600.
  Verify with tests in a `tempfile` dir: a stale file is replaced, a live
  owner is kept, and a directory with a foreign owner is refused (test
  the owner check through an injected uid).
- [x] 3.2 Add the accept loop: the peer uid check (`SO_PEERCRED` on Linux,
  `getpeereid` on macOS; D5), one request line (4 KiB limit, 2 s read
  timeout, injected so tests do not sleep 2 s), one reply line, and the
  `invalid` cases. Verify with one table-driven test: an oversized line,
  a timeout, bad JSON, `v:2` and an unknown `kind` all get `invalid`.
- [x] 3.3 Add the `report` and `status` handlers: the id and text shape
  checks, `unarmed`, the emit with `content` and `meta`, the reply after
  the flush, and `write-failed` (through an injected writer that
  fails). Verify with tests for each scenario of "Report request",
  "Opaque report fields" and "Status request".

## 4. Arm probe

- [x] 4.1 Add the probe: a nonce from `/dev/urandom`, the probe event after
  `notifications/initialized` with the shell-quoted absolute path of the
  running binary (`std::env::current_exe`), the `arm` socket request,
  and the arm state (D3). Verify with unit tests for each scenario of
  "Arm probe" and "Arm request".
- [x] 4.2 Add the `channel-ready --nonce <n>` subcommand to
  `src/cli/mod.rs`: the endpoint of its own pane, one `arm` request, the
  reply on stdout, and the exit codes. Verify with tests for each
  scenario of "Channel-ready command", against a socket in a temp dir.
- [x] 4.3 Add the probe repeat: a `report` on an unarmed server emits the
  probe again, at most once per 60 s and at most 3 repeats per process,
  through an injected clock. Verify with unit tests for "Report while
  unarmed", "Probe limit" and "Lifetime cap".

## 5. Server mode

- [x] 5.1 Add `--mcp` to `src/cli/mod.rs` and the loop in `src/mcp/mod.rs`:
  the stdin thread, the accept thread, logs on stderr, and socket removal
  with exit 0 at EOF. With no endpoint, the server still answers MCP.
  Verify with an integration test `tests/mcp_tests.rs` that runs the
  built binary over pipes with `TMUX`, `TMUX_PANE` and `XDG_RUNTIME_DIR`
  set to a temp dir: initialize → probe → `channel-ready` with the
  probe's nonce →
  a report through the socket → one notification on stdout → EOF →
  the socket is gone. Assert that every stdout line is JSON-RPC.

## 6. Plugin wiring and docs

- [x] 6.1 Add `mcp.sh` beside `hook.sh` with the same lookup order, a
  final `command -v pmux-explorer` check, `exec "$BIN" --mcp`, and exit 1
  with a stderr message when no binary is found (D6). Add the inline
  `mcpServers.pmux` entry, and no `channels` key, to
  `.claude-plugin/plugin.json`. Verify with a new
  `tests/plugin_manifest_tests.rs`: the "Manifest shape" scenario, and
  `mcp.sh` run with an empty plugin dir, an empty `HOME` and a `PATH`
  without the binary exits 1.
- [x] 6.2 Document the endpoint: a "Channel endpoint" section in
  `docs/agent-protocol.md` (the path rule, the request and reply lines,
  the opaque fields, the arm probe), a README section on starting Claude
  with `claude --dangerously-load-development-channels
  plugin:pmux-explorer@slatkin` and its requirements (claude.ai or
  Console auth, org `channelsEnabled`, an interactive session), and
  `src/mcp/` in `CLAUDE.md` Key Modules. Verify by reading them against
  the spec and D6.
- [x] 6.3 Run `cargo fmt --check`, `cargo clippy`, `cargo test` and
  `cargo build --release`, then
  `openspec validate channel-endpoint --strict`. Run `claude plugin
  validate` by hand on the checkout (it is not hermetic, so no test runs
  it). (Automated gates green 2026-10-25; `claude plugin validate` and
  the flagged-session live check are user-run — see PR.)

## Workflow follow-up

- Release a version with `mcp.sh` and the `plugin.json` change, then
  start pmux `channel-report-wake`.
- Archive the change after its PR merges.
