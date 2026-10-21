# Proposal

## Why

The status row shows one name — the Claude session name when it exists, otherwise the agent label — so the user cannot see which agent a named pane runs, and panes that pmux named (spawned subagents, hand-renamed panes) hide that name behind the session name. The pmux subagent badges are also inconsistent: a spawned subagent shows the ♟ pawn icon while a resumed one shows the word `resumed`, spending six extra columns on the most common row.

## What Changes

- The status row's title slot always starts with the agent label (`pi`, `claude`, `codex`, `opencode`), in the agent's colour.
- The tmux pane title is appended after the agent label when the pane has a real name: the title is non-empty, is not the host name (tmux's default pane title), and is not an agent-set OSC title (it does not start with `π - `, which pi writes, or `✳`, which Claude Code writes). pmux names a pane with `tmux select-pane -T <name>` and nothing else, so `pane_title` is the only signal; `automatic-rename` says nothing about it and is not used.
- The Claude session name (from `~/.claude/sessions/*.json`) no longer appears in the status row.
- The resumed pmux-subagent badge becomes the glyph `↻` (one column, `badge_auto` colour), matching the spawned badge's ♟.
- The pane name truncates first when the row runs out of width; the agent label never truncates and the elapsed label keeps its reserved space.

## Capabilities

### New Capabilities

- `status-row`: the composition of an agent pane's status row — agent label, pane name, badge slot, and the elapsed label — and the width behaviour when the row runs out of space.

### Modified Capabilities

- `pi-agent-support`: the subagent badge requirement is updated — the spawned badge is the ♟ pawn glyph (already shipped on main by PR 4, but the spec still says the word `subagent`) and the resumed badge becomes the ↻ glyph instead of the word `resumed`.

## Impact

- `src/tmux/types.rs` — `PaneInfo` gains the resolved pane name.
- `src/tmux/query.rs` — the pane-line format already fetches `pane_title`; it gains one trailing `host` field, and `parse_pane_fields_with_processes` resolves both into the new field. No new tmux call.
- `src/ui/panes/row/status.rs` — title composition, truncation budget, resumed badge glyph.
- Session-name pipeline removal — `src/session.rs`, `pub mod session` in `src/lib.rs`, `session_poll_loop` in `src/app/workers.rs`, the initial scan in `src/app/setup.rs`, `refresh_session_names` and the `sessions.names` map in `src/state/`, and `PaneInfo.session_name`.
- Tests — `src/ui/panes/row.rs` status-row tests and `tests/ui_snapshot.rs` snapshots regenerate; new parse tests for the pane-name resolution.
- Docs — `docs/state-management.md` gains the new `PaneInfo` field if it lists pane fields.
