# Proposal

## Why

When an agent finishes every item on its pane's task list, the finished list
sits in the Tasks tab forever. The list has done its job: it should retire
itself so the sidebar returns to its empty state — while the human still gets
90 seconds to read the final state, and the list stays open for as long as the
agent keeps adding or reopening work.

## What Changes

- A pane's task list closes on its own: once every item is completed and the
  list has been unchanged for 90 seconds, the sidebar empties the store — both
  the items and the pane-wide description — and the Tasks tab shows its empty
  state again.
- Adding a task or reopening one keeps the list open. When the list reaches
  all-done again, a fresh 90-second window starts from that change.
- The close runs per pane regardless of which pane is focused, so a finished
  list cannot survive by being unfocused. The 90 seconds is a named constant,
  not a configuration option.
- The sidebar makes its first task-store write: the close empties the store
  through the existing `clear_tasks` (today it is called only at SessionStart
  from the hook side). The "the TUI only reads" contract is reworded to "the
  agent writes items and statuses; the sidebar empties a finished list" in
  `src/task_store.rs`, `src/ui/bottom/tasks.rs`, and `docs/agent-protocol.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-tasks`: one added requirement — a finished task list closes after 90
  seconds unless a task is added or reopened within that time. The existing
  requirements (write path, tab cycle, reminders, tab display) do not change.

## Impact

- `src/task_store.rs` — the 90-second constant beside `MAX_ITEMS` and a pure
  close predicate over (items, mtime, now); `clear_tasks` stays the close
  action.
- `src/state/refresh.rs` — a per-tick close sweep over the same pane
  population as `refresh_task_progress`, plugged into `refresh_activity_data`
  before the focused-pane reads so the tab clears in the same frame.
- `src/state/pane_runtime.rs` — a per-pane store cache (mtime plus an
  all-done flag) so the sweep stats per tick and re-reads only on change.
- `docs/agent-protocol.md` ("The task command"), `docs/state-management.md`,
  and the `AGENTS.md` module contract — document the close behavior and the
  reworded write contract.
- Tests — pure-predicate unit tests in `task_store`, sweep tests with
  backdated store files, and a Tasks-tab snapshot pinning the empty state
  after a close.
