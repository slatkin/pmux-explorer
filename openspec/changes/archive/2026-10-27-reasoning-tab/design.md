# Design

## Context

See proposal.md for the motivation. The requirements are in `specs/thought-tab`, `specs/bottom-panel`, `specs/agent-tasks` and `specs/shared-view-settings`.

Current state that shapes the approach:

- `BottomTab` (`src/state.rs`) is a three-variant enum with a fixed `next()` cycle and `as_str`/`from_label` for the shared `@sidebar_bottom_tab` option. `from_label` maps any unknown value to Activity.
- `handle_bottom_tab_click` (`src/state/tab.rs`) uses hard-coded column ranges for the three labels. `build_tab_title` (`src/ui/bottom.rs`) draws the labels.
- The Activity and Tasks tabs read a per-pane file for the focused pane on the 1 s refresh (`refresh_activity_log`, `refresh_pane_tasks` in `src/state/refresh.rs`), with a `(pane id, mtime)` cache that skips an unchanged file.
- A pi pane's session id is already parsed into `PaneInfo.session_id` from `@pane_session_id`.
- `src/mcp/path.rs` holds the runtime-dir rule shared with pmux for the channel endpoint: `$XDG_RUNTIME_DIR/<name>`, else `/tmp/<name>-<uid>`.
- The feed is written by pmux (change `thinking-feed` in the pmux repository). Its record shape is the johnny-cant-read thinking feed: `{kind: "start"|"delta"|"end", ts, producer, text?}`, one JSON line each, deltas coalesced at 50 ms.

## Goals / Non-Goals

**Goals:**
- Show the newest thinking block of the focused pi pane with no extra timer and no read when the tab is not drawn.
- Keep the cost of one refresh constant as the feed grows during a long session.
- Keep the shared tab option safe for a mix of old and new sidebar binaries.
- Remove a pi pane's thinking feed file when the sweep clears its agent state — the sidebar owns feed deletion (settled with pmux's `thinking-feed` change on 2026-10-27).

**Non-Goals:**
- A port of johnny-cant-read's full stream assembler (`tail-render.mjs`): no multi-block buffer, no separators, no per-producer open blocks, no per-line stamps.
- Thinking from Claude, Codex or OpenCode.
- Rotation of feed files.

## Decisions

### D1. The effective tab is derived, the selected tab is stored

`global.bottom_tab` stays the stored, shared selection. A new `AppState` method returns the **displayed** tab: the stored tab, except that `Thought` becomes `Activity` — the tab the rule draws as `Tools` — when the focused pane is not pi. Every reader that today matches on `global.bottom_tab` for drawing, scrolling, the git poller flag or the cycle uses the displayed tab instead. Only an explicit selection (cycle key, click) writes the stored value, so a focus change never writes (spec: shared-view-settings).

*Alternative:* write `activity` when the tab hides. Rejected: it breaks the existing "focusing a different agent does not change the bottom tab" requirement and changes the tab in other windows that still focus a pi pane.

### D2. The cycle and the hitboxes come from the list of shown tabs

One function returns the shown tabs: `[Activity, Thought, Tasks, Git]` when the focused pane runs pi, `[Activity, Tasks, Git]` otherwise. The `Activity` variant keeps its name in the code and in the stored `activity` value; only its drawn label reads `Tools`, and it stays first so a non-pi pane keeps the tab order it always had. `next` steps through that list from the displayed tab. The click handler computes label spans from the same list and the same label strings that `build_tab_title` draws, so a hidden label has no hitbox and the columns can never drift from the drawing; the drawn label widths give `Tools` columns 2-6, `Tasks` 8-12 and `Git` 14-16 for a non-pi pane, and `Thought` 8-14, `Tasks` 16-20 and `Git` 22-24 for a pi pane, after the `─ ` prefix in columns 0-1 and one `│` separator between labels. The hard-coded ranges in `handle_bottom_tab_click` go away.

### D3. A last-thought reducer, not the stream assembler

The reader keeps one value per feed: the text of the last thought, the `ts` of the record that began it, and whether its block is still open. The rules in `specs/thought-tab` are the whole reducer:

- `start` → clear the text, take the `ts`, mark open.
- `delta` → append; when open and longer than 16000 chars, keep the last 8000 (the half-drop johnny's assembler uses, so a block does not trim on every delta).
- `end` with text → replace the text, mark closed; `end` without text → mark closed.

The `producer` field is ignored. pmux is the only producer, and the last thought is the newest block whatever its producer. Text is made terminal-safe at ingest: control characters other than `\n` and `\t` and every escape sequence are removed, the same intent as johnny's `terminalSafe`.

*Alternative:* port `createThinkingAssembler` whole. Rejected: the tab shows one block, so the buffer, eviction and separator rules have no use.

### D4. Incremental reads with a backward scan for the first read

State per feed: `(session id, byte offset, mtime, reducer)`. On a tick when the Thought tab is drawn:

1. Resolve the path from the focused pane's session id (D5). No id, or an unsafe id → empty state.
2. If the session id differs from the cached one, or the file is now shorter than the offset (a replaced file), start over: read the file backwards in 64 KiB chunks until a line whose record is a `start` is found (or the start of the file), and feed the reducer from that line.
3. Otherwise, if the mtime is unchanged, do nothing. Else read from the offset to the end, feed complete lines, and keep a trailing partial line for the next tick by leaving the offset before it.

This mirrors the `(pane id, mtime)` cache that Activity and Tasks use, and adds the offset because the feed only grows. A focus change to another pi pane costs one backward scan.

*Alternative:* re-read the whole file on each mtime change. Rejected: a long session's feed reaches several MB and would be parsed every second while the model thinks.

### D5. The feed path helper sits with the channel path rule

A small function returns `<dir>/<session id>.jsonl` with `<dir>` = `$XDG_RUNTIME_DIR/pmux-thinking` or `/tmp/pmux-thinking-<uid>`, taking the env values as arguments the way `EndpointEnv` does, so tests set no process env. It rejects a session id that is empty, `.`, `..`, or contains `/`. Both repositories pin the same path vector in a test, as they do for the channel endpoint.

### D6. Drawing

`src/ui/bottom/thought.rs` draws one yellow `HH:mm:ss` line (local time of the `start` record, in `theme.thought_timestamp` — `@sidebar_color_thought_timestamp`, default 256-color index 220), then the wrapped thought in the normal text color. The block draws in a padded rect — one column in from each panel edge, the text wrapped to the panel width less two, so a blank column stays on the right and the block is not flush against the border — and it starts at the panel's first content row, with the rows under a short block left blank. The scroll uses a `ScrollState` like the other tabs. "Follow the end" is kept as a flag: while the user has not scrolled up, each draw sets the offset to the last page (for a block that fits, `total - visible` is 0, so nothing moves); a scroll up clears the flag and a scroll back to the bottom sets it again. A new `start` resets the scroll to follow. The stamp is a row of the block, so a tall thought scrolls it away with the rows above the last page. The empty notice uses the existing `render_centered`, which centers it.

### D7. Every teardown path removes the feed

Three paths clear the state of a pi pane, and each gains the feed: `clear_agent_pane_state` (`src/tmux/query.rs`), which already wipes the pane's `@pane_*` options and removes its activity log when a Codex/OpenCode/pi pane's process is gone; `clear_dead_agent_metadata` on the port scan (`src/state/refresh.rs`), when the scanned process tree no longer holds the agent; and `run_session_end_teardown` (`src/cli/hook/context/pending.rs`), on a pi `session-end`. All three call `thinking_feed::remove_pi_feed`, which resolves the D5 path from the pane's `@pane_session_id` — read before the options are wiped — and removes the file for a pi pane. A rejected session id removes nothing, and a missing file is not an error, the same treatment the activity log's `remove_file` gets. Feed files of sessions that died while no sidebar ran stay until the directory is cleared (logout for `XDG_RUNTIME_DIR`, reboot for the `/tmp` fallback); a feed holds only one session's thinking, so the leftover is cheap.

The `clear_all_meta` caller list was checked: `run_session_end_teardown` is its only production caller, so the hook side needs one feed removal and no second sweep. `run_worktree_remove_teardown` is not a feed teardown — the pane keeps running, so its session may keep writing.

## Risks / Trade-offs

- [The feed holds model reasoning in a file other local processes of the same user can read] → pmux creates the directory 0700 and the file 0600; the sidebar only reads.
- [A feed whose pane died while no sidebar ran stays until reboot or logout] → Accepted; the sweep removes the feed of every pane it tears down (D7), and `XDG_RUNTIME_DIR` is cleared at logout.
- [The 1 s tick makes a growing thought move in steps] → Accepted by decision: the tab shows a last thought, not a live stream.
- [A thinking block that never gets an `end` (pi killed mid-thought) stays on screen as the last thought] → Accepted; it is the last thing the model thought. The pi teardown sweep clears the pane when the process is gone — a cleared `@pane_session_id` gives the empty notice — and removes the feed (D7).
- [An old sidebar binary reads `thought` as Activity] → Safe by `from_label`'s fallback; it does not write, so it does not reset a new binary's selection.

## Migration Plan

No migration. Ship pmux `thinking-feed` and this change in either order: without the feed the tab shows `No thinking yet`; without this change the feed is written and nothing reads it. Rollback is a binary downgrade.
