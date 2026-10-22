# Proposal

## Why

Agents do their work in tmux panes while the human watches from the sidebar, but the human cannot see the agent's plan. Claude panes show a compact task summary in the agent list, and pi panes show no task state at all because pi reports no task tool. A common write path lets each agent record and update its own per-pane todo list, with pi as the first agent that uses it.

## What Changes

- Add a third bottom-panel tab named Tasks beside Activity and Git, showing the focused pane's todo list.
- Add a shell write path (`task-set` / `task-update` style CLI) that any agent calls through Bash, keyed by `$TMUX_PANE`, so all agents share one contract with no new hook registration.
- Prove the path end to end with pi as the first writer: a live pi pane records and updates tasks and the Tasks tab shows them.
- Add two token-conscious reminders in common wording: record-first when the store is empty at turn start, and update-before-finish when tasks stay open at turn end. No per-tool or per-turn injection.

## Capabilities

### New Capabilities

- `agent-tasks`: agent-reported todo lists shown in the bottom panel Tasks tab, written through a shell command any agent can call.

### Modified Capabilities

- `pi-agent-support`: the "gaps stay gaps" requirement changes — a pi pane CAN now report task progress through the common task command into its own per-pane store, while permission-mode and in-process subagent tree stay unsupported as before.

## Impact

- TUI: `src/ui/bottom.rs` tab title/click/scroll dispatch, new `src/ui/bottom/tasks.rs`, `BottomTab` three-way cycling, per-pane tab memory and auto-switch default, scroll state for the new tab.
- CLI: new `task-set` / `task-update` subcommands plus a per-pane task store file with mtime polling in refresh, mirroring the activity-log pattern.
- Docs: `docs/agent-protocol.md` task section and the pi producer contract note that pi reports no tasks.
- No change to hook triggers, adapter tables, or the `shared-view-settings` capability (bottom tab stays local per instance).
