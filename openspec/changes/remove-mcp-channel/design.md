# Design

## Context

The `--mcp` path was added by `channel-endpoint` (2026-10-25) and reshaped by `prompt-driven-channel-arm` (2026-10-27). Its only consumer is pmux's report delivery, which tries the parent's endpoint first and types the pointer with `send-keys` when the endpoint is not there. The pmux change `inbox-monitor-wake` puts an inbox before that chain and demotes the channel to a "legacy fallback"; its own proposal lists the removal as a follow-up, to open "after the rehearsal and a real campaign" (its tasks, "Workflow follow-up"). The rehearsal passed on 2026-10-28. This document does not claim that a real campaign has run.

See proposal.md for the motivation and the scope.

## What the `--mcp` path exposes

Everything below is removed. The last column is whether a pmux parent still needs it once the inbox is in use.

| Surface | Where | A pmux parent still needs it? |
|---|---|---|
| `pmux-explorer --mcp`: stdio MCP server; answers `initialize` and `ping`, any other request with `-32601`; declares only `capabilities.experimental["claude/channel"]`, no `tools`, `resources`, `prompts` or `instructions` | `src/mcp/mod.rs`, `src/mcp/rpc.rs`, `src/cli/mod.rs` | No |
| MCP tools | none: the server declares no `tools` capability | Not applicable |
| `notifications/claude/channel` event, emitted only for a `report` | `src/mcp/connection.rs` | No: the inbox monitor prints the pointer instead |
| `pmux-explorer channel-ready`: sends one `arm` request to its own pane's endpoint | `src/cli/mod.rs`, `src/mcp/client.rs` | No |
| Endpoint socket `<dir>/<h>-<n>.sock`, `<dir>` = `$XDG_RUNTIME_DIR/pmux-channel` or `/tmp/pmux-channel-<uid>`; mode 0600 in a 0700 directory; peer-uid gate; stale-socket replacement | `src/mcp/endpoint.rs`, `src/mcp/path.rs`, `src/mcp/connection.rs` | No |
| Socket requests `arm`, `status`, `report` | `src/mcp/connection.rs` | No. pmux sends only `report` (its `channel-endpoint.ts` builds one `kind: "report"` request) |
| Plugin wiring: `mcpServers.pmux` in `plugin.json`, `mcp.sh` | `.claude-plugin/plugin.json`, `mcp.sh` | No |
| Session opt-in flag `--dangerously-load-development-channels plugin:pmux-explorer@slatkin` | user's launch command | No: pmux's skill already tells new setups not to pass it |

None of the surface is needed by a pmux parent that runs the inbox monitor. A parent without a running monitor does need the channel today, but only as the middle step of pmux's chain (inbox, then channel, then typed pointer); see Risks.

## pmux-side callers (read only, for the pmux follow-up)

In `/home/slatkin/Dev/pmux`, a search for `channel-endpoint`, `report-channel`, `report-delivery`, `channel-ready` and `development-channels` finds:

- `pi-extension/pmux/channel-endpoint.ts`: the socket client (`submitChannel`, `channelEndpointPath`, `fnv1a64Hex`). It sends `report` only. `report-inbox.ts` imports `fnv1a64Hex` from it, so the follow-up must move that function before it deletes the file.
- `pi-extension/pmux/channel-reason.ts`: the reason vocabulary (`no-endpoint`, `refused`, `timeout`, `unarmed`, `write-failed`, `invalid`, `bad-reply`), shared by the client, the receipt log and the delivery flow.
- `pi-extension/pmux/report-delivery.ts`: `submitReportPointer` runs the inbox, then `submitChannel`, then `verifiedAgentPrompt`. It writes `channelError` and `via: "channel"` into its result.
- `pi-extension/pmux/cli.ts`: forwards `submitChannel` as a dependency and prints `channelError`. Also touched: `run.ts`, `runtime-dir.ts` and `sleep.ts` (comments or imports).
- `openspec/specs/report-channel/spec.md` and `openspec/specs/external-orchestration/spec.md`: the `report-channel` capability and the channel scenarios.
- `skills/pmux/SKILL.md` ("Channel wake-up (legacy fallback)") and `docs/guide.md` (the "Report transport" paragraph), which name the flag and `channel-ready`.
- Tests: `test/channel-endpoint.test.ts` and `test/report-delivery.test.ts`, plus channel cases in `test/cli.test.ts` and `test/run.test.ts`.

## Decisions

### D1. Remove the capability whole, not in parts

The eleven requirements form one chain: the socket exists to serve `report`, `arm` exists to gate `report`, the plugin entry exists to start the server. A partial removal (for example, keep the socket and drop MCP) has no consumer. The spec delta marks every requirement REMOVED, with the reason.

### D2. Order of removal

pmux degrades safely when the endpoint is missing: the channel result is `no-endpoint`, and `submitReportPointer` then types the pointer. So no explorer surface has to stay for pmux to keep working, and removal order is about which delivery path a parent is left with.

1. **Gate (outside this repo).** The user's Claude orchestration prompts no longer carry `channel-ready` or the channels flag, and the pmux inbox monitor is the documented default. A `grep -rn "channel-ready\|development-channels"` over the user's prompt and alias sources must be clean (see R2).
2. **pmux follow-up lands first (recommended).** It removes `submitChannel` from the chain, so pmux no longer attempts a connection that always ends `no-endpoint`. This repo's change can still merge before it; the cost is one failed connect per un-monitored delivery, and `channelError: "no-endpoint"` in the receipt log.
3. **This change, in this commit order**, each commit compiling and passing `cargo test`:
   1. Plugin wiring: `mcpServers` out of `plugin.json`, delete `mcp.sh` and `tests/plugin_manifest_tests.rs`. New sessions stop loading the server.
   2. CLI: the `--mcp` and `channel-ready` arms, `cmd_channel_ready`, `run_channel_ready` and their tests.
   3. The module: `src/mcp/`, `src/test_support.rs`, `tests/mcp_tests.rs`, the `mod` lines.
   4. Docs and the spec delta.

What must stay until the pmux follow-up lands: nothing for correctness. For behavior, the `report` request contract (path rule, `report` shape, `unarmed` / `no-endpoint` replies) is the only surface pmux uses; keeping it until the follow-up is what preserves the "legacy fallback" that pmux's inbox change promises.

### D3. No tombstone for `channel-ready` (decided)

An unknown subcommand returns `None` from `cli::run`, and `main` then starts the TUI. After removal, a stale prompt that runs `pmux-explorer channel-ready` inside Claude's Bash tool (where `TMUX_PANE` is set) would start the sidebar TUI there instead of failing. A tombstone (a `channel-ready` arm that prints one line on stderr and exits 1) costs about ten lines and one test. Decision (user, 2026-10-28): no tombstone. The replacement is fully implemented in pmux, so no prompt needs the command.

### D4. Leave leftover sockets alone

A crashed Claude session can leave a socket under the runtime directory. `$XDG_RUNTIME_DIR` is cleared at logout or reboot, and `/tmp/pmux-channel-<uid>` is the rare fallback. Cleanup code in a binary that no longer owns the directory is speculative, so none is added.

## Risks / Trade-offs

- **R1. A pane that already has the flag.** A session started with `--dangerously-load-development-channels plugin:pmux-explorer@slatkin` before this change keeps its `--mcp` process (it runs from the old binary image) and its socket until the session ends, so reports still reach it, if armed. A session started after this change names a plugin that declares no server. What Claude Code does then (a warning, a refusal to start, or silence) was checked on 2026-10-28 (Claude Code 2.1.311) with a throwaway copy of the plugin manifest that has no `mcpServers`, loaded with `--plugin-dir` and the flag set to `plugin:pmux-explorer@slatkin`, in a detached tmux window. Claude Code showed the "WARNING: Loading development channels" prompt, which needed one Enter (option 1, "I am using this for local development"). It then started normally, and the banner read "Channels (experimental) messages from plugin:pmux-explorer@slatkin inject directly in this session". It did not refuse to start and printed no error about the missing server. Caveat: the banner may name the installed plugin, not the throwaway copy, so this run does not prove that a server-less plugin behaves the same way in every case. Task 5.1 stays open until the user accepts this result. The flag lives in the user's launch command or alias, which this repo does not own; the pmux skill already says new setups must not pass it.
- **R2. A stale `channel-ready` in a prompt.** See D3. It starts the TUI instead of a clean failure.
- **R3. The typed pointer becomes the only fallback.** Without the channel, a parent with no live inbox monitor gets the `send-keys` pointer, which can be lost with no error (a 5,568-byte report was lost this way on 2026-10-24, the reason for the channel). The inbox protects only a parent whose monitor is running. The rehearsal is one sample, with an idle orchestrator. pmux's `spike.md` section 1.4 records that the monitor does not come back after `claude --resume` and expires at its `timeout_ms` (1800000), so a long campaign needs the re-arm rule. Quit, `/compact` and `--resume` were not retested in the rehearsal. This is the main reason the gate in D2 asks for a real campaign.
- **R4. Claude version.** The inbox needs the `Monitor` tool (Claude Code 2.1.98 or later, per the pmux proposal). A parent on an older build has no monitor and, after this change, no channel.
- **R5. Less test coverage on the plugin manifest.** `tests/plugin_manifest_tests.rs` pins the `mcpServers` entry and the `mcp.sh` wrapper; with both gone the file goes too. `tests/plugin_hooks_tests.rs` keeps covering the hook wiring.
- **Not a risk:** other agents. The channel serves Claude only; Codex, OpenCode and pi panes never used it.

## Migration Plan

1. Meet the D2 gate (D3 is decided).
2. Merge the pmux follow-up (outside this repo), then this change, in the commit order of D2.
3. `cargo fmt`, `cargo test`, `cargo clippy`, then `cargo build --release`. Restart the sidebar.
4. Users: drop the channels flag from the launch command. Existing sessions end when they end.
5. Rollback: revert the commits. The endpoint protocol is unchanged in git history, so no data migrates either way.
