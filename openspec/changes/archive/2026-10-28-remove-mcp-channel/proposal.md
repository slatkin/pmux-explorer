# Proposal

## Why

The `--mcp` channel server exists to hand a Claude orchestrator a pmux report pointer without typing into its pane. The pmux change `inbox-monitor-wake` does that job another way: a per-parent report inbox, and `pmux inbox --follow` running under Claude's `Monitor` tool, which wakes the orchestrator when a report arrives. Its live rehearsal (task 6.2, pmux commit `b59b723`, `spike.md` "6.2 Live rehearsal") passed on 2026-10-28: a report reached an idle orchestrator through the inbox, its `ack` removed the entry, and no channel was armed or used.

The user has said the channel is superseded and not needed. It is also the part of this binary with the most cost for the least use: a second mode of the binary (`--mcp`), a hand-written JSON-RPC layer, a Unix socket per Claude pane, a peer-uid gate, a plugin wrapper script, and a research-preview Claude flag (`--dangerously-load-development-channels`) that every flagged session must carry.

## What Changes

- **BREAKING** `pmux-explorer --mcp` is removed. The binary has no stdio MCP server mode and no endpoint socket (`$XDG_RUNTIME_DIR/pmux-channel/*.sock`, or `/tmp/pmux-channel-<uid>/*.sock`).
- **BREAKING** `pmux-explorer channel-ready` is removed.
- **BREAKING** The Claude plugin stops declaring a server: the inline `mcpServers.pmux` entry leaves `.claude-plugin/plugin.json`, and `mcp.sh` is deleted. The flag `claude --dangerously-load-development-channels plugin:pmux-explorer@slatkin` no longer has a channel to load.
- `src/mcp/`, `src/test_support.rs` (all of its helpers serve the MCP tests), `tests/mcp_tests.rs` and `tests/plugin_manifest_tests.rs` are deleted.
- The capability `channel-endpoint` is removed: all eleven of its requirements.
- `bin-lookup.sh` stays (`hook.sh` sources it). Only its comments stop naming `mcp.sh`.
- Docs: the "The channel endpoint" section of `docs/agent-protocol.md`, the README step "Hand Claude a channel (optional)", and the `mcp/` entry and the `channel-ready` / `--mcp` mentions in `AGENTS.md` (`CLAUDE.md` is a symlink to it) are removed.

Out of scope: any edit in the pmux repository (`channel-endpoint.ts`, `report-channel`, `report-delivery.ts` and the others named in `design.md` are listed there for the pmux follow-up, not changed here), the hook protocol, the `@pane_*` and `@sidebar_*` options, the archived changes `channel-endpoint` and `prompt-driven-channel-arm` (they stay as history), and any cleanup of leftover socket files in a user's runtime directory.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `channel-endpoint`: every requirement is removed, so the capability goes away.

## Impact

- Code: `src/mcp/` (six files, 2086 lines), `src/test_support.rs`, `src/lib.rs` (two `mod` lines), `src/cli/mod.rs` (the `--mcp` arm, the `channel-ready` arm, `cmd_channel_ready`, `run_channel_ready` and their tests), `mcp.sh`, `.claude-plugin/plugin.json`, `tests/mcp_tests.rs`, `tests/plugin_manifest_tests.rs`, and two comment lines in `bin-lookup.sh`.
- Docs: `docs/agent-protocol.md`, `README.md`, `AGENTS.md`.
- Dependencies: none removed. `libc` and `serde_json` are used outside `src/mcp/`.
- Users: a Claude session that is already running keeps its `--mcp` process and its socket until the session ends. A new session loads no server. See `design.md` for the flagged-pane and stale-prompt risks.
- Not affected: the hook protocol, the adapters, the task store, the thinking feed, and the pi subagent identity.
