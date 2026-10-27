## Project Overview

A tmux sidebar TUI (built with Ratatui + Crossterm) that monitors AI coding agents (Claude Code, Codex, OpenCode, and pi) across all tmux sessions/windows/panes in real-time. Distributed as a single binary via tmux plugin managers.

This project was forked to add pi support and no longer tracks upstream. The pi work is the `AgentType::Pi` agent (color option, event adapter, process-tree integration), the documented producer protocol (`docs/agent-protocol.md`), and the pmux subagent identity for pi panes spawned by pmux. The design records for these changes live in `openspec/changes/archive/` (`pi-agent-support`, `pi-subagent-identity`, …).

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (strip + lto enabled)
cargo test                     # Run all tests
cargo test <test_name>         # Run a single test
cargo clippy                   # Lint
cargo fmt                      # Format code
cargo fmt --check              # Check formatting (used in CI)
```

CI runs `cargo test`, `cargo clippy`, and `cargo fmt --check` on every push/PR.

**Before creating any git commit**, always run `cargo fmt` first to avoid CI formatting failures. This applies to every commit, not just the final one.

After implementation is complete, run `cargo build --release`. The plugin directory is usually a symlink to this repo, so the binary is picked up automatically; only a worktree build needs a manual copy (see "Debugging" section below).

## Architecture

### Entry Points

The binary has two modes controlled by CLI args (`src/cli/mod.rs`):
1. **TUI mode** — default. `src/main.rs` handles CLI arg parsing, SIGUSR1 signal wiring, and TUI session setup, then delegates to `app::run` (`src/app.rs`) for the event loop.
2. **CLI subcommands** — `setup`, `hook`, `toggle`, `toggle-all`, `auto-close`, `set-status`, `task-set`, `task-update`, `task-describe`, `spawn`, `capture`, `channel-ready`, `--version` / `version`; plus the `--mcp` mode, a stdio channel server (see `mcp/` below).

### Core Data Flow

```
Producers (hook.sh, plugins, extensions) → CLI `hook` subcommand
                           ↓
        adapter/ normalizes raw JSON into AgentEventKind
                           ↓
        event/ builds an internal AgentEvent
                           ↓
        cli/hook/handlers dispatches on_* per event, which:
          • sets tmux pane options (@pane_status, @pane_attention, etc.)
          • appends to /tmp/pmux-explorer-activity*.log
                           ↓
TUI event loop (app::run) → AppState::sync_global_state()
          • reads tmux panes via single `list-panes -a`
          • parses /tmp/pmux-explorer-activity*.log
                           ↓
                ui::draw() renders frame
```

The `<event>` names and JSON shape at this boundary are the **agent event protocol**, written down in `docs/agent-protocol.md`. A producer is a hook, a plugin, or an extension of an agent — the sidebar does not care which; it reads only `$TMUX_PANE` from the command's environment and one JSON object from stdin.

### Agent producers

Only the producers differ per agent:

- **claude / codex** — the hook script registered in a settings file; `setup claude|codex` (`src/cli/setup/`) merges the hook config into `~/.claude/settings.json` / `~/.codex/hooks.json` from the adapter's `HOOK_REGISTRATIONS` table.
- **opencode** — a plugin bridge (`.opencode/plugins/pmux-explorer.js`) symlinked into `~/.config/opencode/plugins/`.
- **pi** — the pi extension that pmux ships (implemented outside this repo). pi has no hook config file to merge, so `setup pi` prints the producer contract instead: per event, the JSON fields a producer sends, generated from `adapter/pi.rs`'s `PI_EVENTS` table. `PI_EVENTS` is the source of truth for pi's protocol side, and tests pin it and the adapter's `parse()` arms in step.

### pi support and the pmux subagent

pi's producer is external, but everything the sidebar does with pi's events lives here:

- A pmux subagent is a pi process that pmux starts in its own pane, identified by its first prompt: pmux writes the task as a file and pi expands the task argument into a prompt starting `<file name="/…/artifacts/<parent session id>/context|pmux-resume/<name>-<timestamp>.md">`. `adapter/pi.rs` parses that shape (through `pmux_task.rs`) instead of storing it as the pane prompt, and the hook handler writes `@pane_subagent_kind` (`spawned`/`resumed`), `@pane_subagent_parent`, and `@pane_subagent_task` (the fuzzy task; the fallback stems `subagent`/`resume` give none).
- The subagent identity exists only for pi panes (`parse_subagent_identity` in `src/tmux/query.rs` rejects every other agent). The `resolve_subagent_parents` post-pass, also in `query.rs`, matches the parent session id against the `list-panes` output; the row renders in `ui/panes/row/body.rs` — the `subagent`/`resumed` badge (borrowing Claude's permission-mode badge slot, which a pi pane leaves empty), the parent pane's title, and the fuzzy task name on a line just above the prompt text. Without a parent pane it renders `ORPHANED` in the error color.
- pi reports no permission mode and no task tool, so those rows stay empty for pi; its wait signal arrives as `notification` events from pi's `ui_prompt` behaviour.
- A pi process can die without any session-end event reaching the sidebar, so teardown is a poll-side sweep: when a Codex/OpenCode/pi pane has dropped back to the user's shell and the process tree (`src/process.rs`) no longer contains the agent, `clear_agent_pane_state` wipes the `@pane_*` options and the activity log. Claude is excluded — its `SessionEnd` hook drives cleanup.

### Key Modules

- **`state.rs` + `state/`** — `AppState` central struct plus topical submodules (`activity`, `focus`, `scroll`, `pane_runtime`, `layout`, `popup`, `notices`, `timers`, `filter`, `global`, `refresh`, `tab`, `tasks`, `thought`, `popout`). All UI is computed from this state. `state/popout.rs` holds the pop-out/pop-in action, the floating-panel geometry, and the bottom-panel clamp; `state/thought.rs` holds the Thought tab's reader state and the feed environment captured at startup.
- **`app.rs` + `app/`** — TUI orchestration: `setup` (prime `AppState`), `workers` (background git/version threads), `input` (keyboard/mouse handling), `render` (per-frame render entry), `panel_guard` (re-asserts the floating panel's height, width and right-edge anchor after a window resize). Split out from `main.rs` so the binary entry point only handles CLI dispatch, signal wiring, and TUI session setup.
- **`tmux.rs` + `tmux/`** — `tmux.rs` is a facade re-exporting five modules: `query.rs` (the single `list-panes -a` parse — `pane_name` extraction with the agent-OSC-title and em-dash rules, the poll-side stale-agent sweep for Codex/OpenCode/pi, and the `resolve_subagent_parents` post-pass), `panes.rs` (one per-tick `display-message` call reading the sidebar's own pane state and the shared `@sidebar_*` options, `SidebarPaneReading`), `types.rs` (`PaneInfo`/`PaneStatus`/`AgentType`/`PermissionMode`/`WorktreeMetadata`/`SubagentInfo`/`SubagentParent`), `options.rs` (`PANE_*`/`SIDEBAR_*` option-name constants), and `commands.rs` (tmux subprocess helpers).
- **`adapter/`** — Per-agent adapters (`claude`, `codex`, `opencode`, `pi`). Each binds its agent's upstream events to internal `AgentEventKind`s (a `HOOK_REGISTRATIONS` table for `claude`/`codex`, the printable `PI_EVENTS` producer contract for pi) and a `parse()` maps raw JSON payloads into `AgentEvent`, rewriting per-agent names onto the shared shape: pi's lowercase tool names (`read`, `bash`) become the canonical PascalCase labels and pi's `path` file key is copied beside `file_path`; alias spellings (`wait_reason` for `notification_type`, the `error*` variants for `error`) are accepted. Single source of truth consumed by the setup wizard, README snippets, and tests; table-versus-`parse()` drift is pinned by per-adapter tests.
- **`event.rs` + `event/`** — Internal event layer: `AgentEvent` (pre-extracted fields; handlers never touch raw JSON or agent names), `AgentEventKind` (compile-time enum for hook kinds), `EventAdapter` trait + `resolve_adapter`.
- **`pmux_task.rs`** — Parses the pmux task prompt a pi subagent's first turn carries into `PmuxTask { kind, parent_session_id, task }`. Only the `name` attribute of the `<file>` tag is read; a path not matching the `/artifacts/<parent session id>/context|pmux-resume/` shape returns `None`, and the prompt stays stored as the pane prompt. The contract is specified in `docs/agent-protocol.md` ("A pmux task prompt").
- **`task_store.rs` / `task_reminders.rs`** — Per-pane task store (one JSON file per pane at `/tmp/pmux-explorer-tasks*.json`) behind the `task-set` / `task-update` / `task-describe` subcommands: `record_task`/`update_task_status`/`set_description` write, the sidebar's one write is `clear_tasks` emptying a list that has sat all-completed and untouched for `TASK_CLOSE_SECS` (90 s — any store write restarts the window), and trim keeps the cap at [`MAX_ITEMS`]. `task_reminders.rs` is the single wording source for the two agent reminders the help text, `docs/agent-protocol.md`, and the pmux companion embed verbatim.
- **`thinking_feed.rs`** — The pi thinking feed behind the Thought tab: `feed_path` (`<dir>/<session id>.jsonl` under `$XDG_RUNTIME_DIR/pmux-thinking`, else `/tmp/pmux-thinking-<uid>`, rejecting an empty, `.`, `..` or `/`-holding session id), the last-thought reducer (`start` clears and restamps, `delta` appends with the 16000/8000 tail trim while the block is open, an `end` with text replaces and one without text closes, invalid lines are ignored, text is made terminal-safe at ingest), and the incremental `ThinkingFeedReader` (per-feed `(session id, offset, mtime)` cache, backward 64 KiB scan to the newest `start`, partial trailing line held back). `remove_pi_feed` is what the three teardown paths call with the session id read before the pane options are wiped. The path rule and record shape are shared with pmux's `thinking-feed` change and written down in `docs/agent-protocol.md` ("The thinking feed").
- **`mcp/`** — The Claude channel server behind `--mcp` and the `channel-ready` subcommand: a stdio MCP server (`mod.rs`: stdin + accept threads, socket removal and exit 0 at EOF) that declares only `capabilities.experimental["claude/channel"]` and negotiates its revision below `2026-07-28` (`rpc.rs` — hand-written JSON-RPC on `std::thread`, the generic `LineWriter<W>` over `Arc<Mutex<W>>` serializing every line). The per-pane Unix endpoint is keyed by an FNV-1a 64 hash of the tmux socket path plus the pane number (`path.rs`); `endpoint.rs` owns the 0700 directory, the 0600 socket, stale-socket replacement, and the live-owner rule (a socket that accepts a connection has an owner and is not taken over). Arming goes through the probe (`probe.rs`: one `notifications/claude/channel` probe after `notifications/initialized`, repeated on unarmed reports at most once per 60 s and 3 times per process; the nonce is 32 hex from `/dev/urandom`) answered by `channel-ready` (`client.rs`: one request line, one reply line, 2 s answer window); `connection.rs` applies the peer-uid gate (a foreign-uid connection is closed with no reply) and answers the socket's `report`/`status`/`arm` requests. The protocol is specified in `docs/agent-protocol.md` ("The channel endpoint"); the plugin loads the server through the inline `mcpServers.pmux` entry in `.claude-plugin/plugin.json`, whose `mcp.sh` sources the shared `bin-lookup.sh` binary lookup — the same cascade `hook.sh` sources, with one deliberate difference: a failed lookup reports on stderr and exits 1, so a dead server shows up in Claude's `/mcp` panel — and runs `exec "$BIN" --mcp`.
- **`cli/hook.rs` + `cli/hook/`** — Receives real-time status updates from agent producers; dispatch in `hook.rs`, with submodules `context` (shared helpers + `AgentContext`), `handlers` (per-event `on_*` handlers), `activity` (activity log writing), `notifications` (desktop notification helpers).
- **`git.rs`** — Git operations (branch, ahead/behind, PR numbers via `gh` CLI, diff stats). Runs in a background polling thread.
- **`activity.rs`** — Parses `/tmp/pmux-explorer-activity*.log` files, maps tool types to colors.
- **`group.rs`** — Groups panes by repository path.
- **`worktree.rs` / `tool_name.rs` / `version.rs` / `port.rs` / `clipboard.rs` / `desktop_notification.rs`** — Leaf helpers used across modules (worktree metadata parsing, tool-name classification, version reporting, port detection, clipboard + desktop notification shims).
- **`ui/`** — Rendering layer: `mod.rs` (entry `draw`), `panes.rs` (agent list + repo filter) with submodules under `panes/` (`filter_bar`, `row` split into `row/{status,branch,body,ctx}.rs` — status row, branch/ports row, body rows including the pmux subagent identity row, plus `row_collector`, `click_targets`, `popups`); `bottom.rs` + `bottom/` with submodules (`activity`, `tasks`, `git`, `thought`) for the Activity/Tasks/Git tabs and the Thought tab, which draws the focused pi pane's last thought with its `HH:mm:ss` start line and follow-the-end scrolling; `colors.rs` (256-color theme); `icons.rs` (agent/status glyphs); `notices.rs` (transient banner rendering); `text.rs` (text formatting/truncation).

### State Management

See `docs/state-management.md` for the full scope/update-frequency table, per-pane tmux options, data flow, and key type definitions.

Process-level detail not covered there: SIGUSR1 triggers an instant refresh on tmux pane focus change (handler in `src/main.rs` flips a shared `AtomicBool` that the `app::run` loop polls).

### Testing

Tests are in `/tests/` using Ratatui's `TestBackend` for UI rendering assertions. `test_helpers.rs` provides buffer-to-string conversion utilities. Heavy use of snapshot-style tests for UI regression prevention.

**UI test rule**: any test that renders a frame MUST use `insta::assert_snapshot!(output, @"...")` inline snapshots — never `assert!(output.contains(...))` or similar substring checks. A contains assertion only verifies that a specific string appears somewhere; it silently tolerates layout drift (border shifts, color changes, row reordering, new artifacts) that a snapshot diff would surface immediately. The stronger check is free — `cargo insta accept` regenerates the expected output when the change is intentional. Substring assertions are acceptable only for non-visual properties (`layout.repo_spawn_targets` contents, state struct fields, etc.) where there is no frame to snapshot.

## Debugging (Local tmux Plugin)

`~/.tmux/plugins/pmux-explorer` is typically a symlink to this repository, so `cargo build --release` alone updates the binary tmux loads. Just restart the sidebar (toggle off → on via the tmux keybinding) to pick up the new build.

```bash
cargo build --release
# Restart sidebar (toggle off → on via tmux keybinding)
```

**When working in a worktree**: Worktrees build into their own `target/release/`, which is not what the plugin directory points at, so the artifact must be copied manually AND re-signed. On macOS (Darwin 24+), `cargo` produces a `linker-signed` ad-hoc signature that the kernel will SIGKILL (signal 9) immediately after a `cp` — the kernel refuses to honor a linker-only signature on a file it didn't write itself. Replace it with a fresh ad-hoc signature to avoid the kill:

```bash
cp <worktree-path>/target/release/pmux-explorer ~/.tmux/plugins/pmux-explorer/target/release/pmux-explorer
codesign --force --sign - ~/.tmux/plugins/pmux-explorer/target/release/pmux-explorer
```

If tmux reports `terminated by signal 9` after a worktree build, you almost certainly skipped the `codesign` step. Clearing `com.apple.provenance` with `xattr -c` is not required — the kernel only cares about the signature flavor.

## Rust Edition

This project uses Rust edition 2024 (`Cargo.toml`).

## Writing Guidelines

- All documentation under `docs/` and all skill files under `.claude/skills/` must be written in English.
