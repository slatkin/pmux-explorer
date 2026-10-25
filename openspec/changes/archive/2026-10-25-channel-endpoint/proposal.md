# Proposal

## Why

pmux delivers a subagent's report pointer to a Claude Code parent by
typing it into the parent's pane with `tmux send-keys`. Typed input can
be lost with no error: a busy prompt can accept the Enter without reading
the text, and the record proves only that the text was typed. A
5,568-byte report was lost this way on 2026-10-24.

Claude Code Channels lets an MCP server that the session spawned push an
event into that session, and the event queues while Claude is busy. No
command can inject into a session from outside, so the server must run
inside each parent session. pmux-explorer already runs beside every
Claude session as its Claude plugin, so it is the host for that server.
This change is the prerequisite of the pmux change `channel-report-wake`,
which sends reports to this endpoint first and falls back to
`send-keys`.

## What Changes

- New mode `pmux-explorer --mcp`: a stdio MCP server that declares only
  the `claude/channel` capability, with no tools and no `instructions`,
  so it adds no tokens to any turn. MCP is only the transport; the model
  sees CLI commands. It negotiates an MCP revision before `2026-07-28`, because Claude Code
  does not register a channel server on that revision.
- The server owns one local endpoint per parent pane: a Unix socket at a
  path made from the tmux socket path and `TMUX_PANE`. The path rule is
  shared with pmux, and both repositories pin one test vector.
- **Arm probe.** Claude Code drops channel events silently when the
  session did not register the server as a channel. So the server emits
  one probe event and is armed only after Claude runs the command the
  probe names, the new `pmux-explorer channel-ready --nonce <n>`, which
  sends the nonce back over the socket. While the server is not
  armed, the endpoint refuses reports with `unarmed`, and pmux uses its
  fallback.
- The endpoint accepts a `report` request (a pmux report pointer) and
  emits it as a channel event. It replies `emitted` only after the
  event is written to Claude Code. It also answers a `status` request
  with the arm state.
- The endpoint accepts only the same user (a peer uid check) and only
  text with the pmux pointer shape.
- The Claude plugin registers the server through `mcpServers` and a
  small wrapper script that finds the binary the way `hook.sh` does.
- The user turns the channel on per session with
  `claude --dangerously-load-development-channels plugin:pmux-explorer@slatkin`.
  Without the flag, the server runs but never arms, and nothing changes
  for the user.

## Capabilities

### New Capabilities

- `channel-endpoint`: the MCP channel server mode, the arm probe, the
  per-pane endpoint and its request contract, and the plugin
  registration.

### Modified Capabilities

None. The hook protocol, the TUI and the `@pane_*` and `@sidebar_*`
options do not change (`product-identity`, "Unchanged protocol and
options").

## Non-goals

- A reply path for the `ack`. pmux keeps its `ack` command.
- A permission relay (`claude/channel/permission`).
- Channels for Codex, OpenCode or pi.
- Any change to Claude Code.

## Impact

- New module `src/mcp/` (JSON-RPC framing, the server loop, the endpoint,
  the path rule) and a `--mcp` arm in `src/cli/mod.rs`.
- No new crates: `serde_json` and `libc` are enough, and the server uses
  `std::thread`, not an async runtime.
- New `mcp.sh` beside `hook.sh`, and `mcpServers` in
  `.claude-plugin/plugin.json`.
- Docs: `README.md`, `docs/agent-protocol.md` (a new section for the
  channel endpoint), `CLAUDE.md` (Key Modules).
- Tests: unit tests in `src/mcp/` and an integration test in `tests/`
  that drives the server over pipes and a socket in a temp dir.
- Consumer: pmux `channel-report-wake` (repository `slatkin/pmux`).
