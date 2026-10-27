# Design

## Context

The task store (`src/task_store.rs`) is one JSON file per pane under `/tmp`,
written only by the agent-side CLI (`task-set` / `task-update` /
`task-describe`) and by `clear_tasks` at SessionStart
(`src/cli/hook/handlers/session.rs`). The TUI consumes it through two paths:
`refresh_pane_tasks` fills `TasksState` for the focused pane only, gated by
`file_cache_fresh` on `(pane_id, mtime)`; `refresh_task_progress` sweeps every
pane in `repo_groups` per tick with per-pane bookkeeping in `PaneRuntimeState`.
Both live in `src/state/refresh.rs`, whose `refresh_activity_data` pipeline runs
`refresh_activity_log` → `refresh_pane_tasks` → `refresh_task_progress` every
second. `AppState::now` is epoch seconds (`crate::time::now_epoch_secs`), and
the inactive-grace dismissal (`INACTIVE_GRACE_SECS` in `refresh_task_progress`)
already demonstrates the "timestamped condition evaluated per tick" pattern.
See proposal.md — Why for the motivation.

## Goals / Non-Goals

**Goals:**
- The close rule is a pure predicate, unit-testable without tmux or the TUI.
- A finished list closes on time regardless of which pane is focused, and
  regardless of sidebar restarts in between.
- A focused list that closes renders the empty state in the same frame.
- The store's lifecycle policy (cap, trim, close) stays owned by
  `task_store`, not by the state layer.

**Non-Goals:**
- Closing stores of panes the sidebar no longer lists (dead panes):
  `clear_tasks` at SessionStart and `/tmp`'s own lifecycle cover them.
- Making the 90 seconds configurable. A tmux option would need its own
  spec change if ever wanted.
- Changing the task-progress row's dismissal policy (the activity-log-driven
  ✔◼◻ row keeps its instant dismiss), the task reminders, or the tab cycle.
- Reviving a closed list — the data is gone by design.

## Decisions

**D1: The close rule is a stateless predicate over (items, mtime, now).**
`task_store` gains `close_due(list: &TaskList, mtime_epoch: Option<u64>,
now_epoch: u64) -> bool`: true when the list holds at least one item, every
item is `completed`, and `now_epoch - mtime_epoch >= TASK_CLOSE_SECS`. Every
mutation of a store goes through `write_store`, which bumps the file mtime,
so "all-done and unchanged for 90 seconds" is exactly the spec rule: an added
or reopened item makes all-done false, and a refinished list carries the new
mtime into the next window. The predicate needs no stored deadlines, so it
survives sidebar restarts and focus changes for free. Alternative considered:
an explicit per-pane arm/cancel deadline (`all_done_since` beside
`inactive_since`) — rejected as more state to maintain and reset, with no
behavioral gain; a poller cannot observe sub-tick write transitions either
way. On record: one benign deviation from the literal rule — *any* store write
including `task-describe` restarts the window, because mtime cannot tell which
field changed. A write means the list is being touched; restarting is the
friendly behavior.

**D2: The close action is the existing `clear_tasks`.** It removes the store
file, which is exactly "empty items and description together" (the exploration
decision), and it is already the sidebar's store-teardown primitive at
SessionStart. This makes the TUI the second caller of a store write — the
first sidebar-side one. The contract comment in `task_store.rs`, the header of
`ui/bottom/tasks.rs`, and `docs/agent-protocol.md` ("The task command") get
reworded from "the TUI only reads" to "the agent writes items and statuses;
the sidebar empties a finished list". Alternatives rejected: rewriting the
store with empty items but a kept description (leaves the stale pinned header
the exploration ruled out); routing the close through a new CLI subcommand
(indirection with no second caller — the predicate and the sweep are both
TUI-side).

**D3: The sweep covers every pane in `repo_groups`, per tick.** A new
`refresh_task_close` walks the same pane population as `refresh_task_progress`
and keeps a per-pane cache in `PaneRuntimeState` (`task_store_mtime` plus a
`task_store_all_done` flag): one `store_mtime` stat per pane per tick, and a
`read_task_list` only when the mtime changed. When `close_due` holds it calls
`clear_tasks` and drops the cache entry. Alternative rejected: focused-only
observation (reusing `TasksState` alone) — an unfocused finished list would
survive arbitrarily long and then close at an unpredictable moment on focus,
which violates the "same 90 seconds, focused or not" scenario.

**D4: `TASK_CLOSE_SECS: u64 = 90` lives in `task_store.rs` beside
`MAX_ITEMS`**, as part of the store's lifecycle policy; the predicate takes
epoch seconds to match `AppState::now` and the inactive-grace precedent.
Tests: the predicate is pure (boundary at 89/90 seconds, empty list, mixed
statuses); the sweep is tested with real `/tmp` stores like `task_store`'s
own tests, backdating mtime via `std::fs::File::set_modified` (stable).

**D5: The sweep runs at the head of `refresh_activity_data`**, before
`refresh_pane_tasks`, so a focused list that closes renders the empty state in
the same frame. Even out of order the focused tab would self-heal one tick
later: after removal, `file_cache_fresh` finds a `None` mtime against a real
cache and re-reads.

**D6: Edge semantics.** A list with zero items never closes — there is
nothing to close, and a description-only store (agent described, then died)
lingers; accepted. A corrupt store reads as empty and therefore never closes;
accepted (the file dies with `/tmp`). An mtime in the future (clock jump)
reads as "not due": the list closes late, never early.

## Risks / Trade-offs

- [A close wipes a list someone is reading] → the 90-second window is the
  mitigation; the close fires only on a fully completed, quiet list.
- [Benign writes (describe, idempotent re-complete) delay a close by up to
  90 s] → accepted under D1; a write is evidence the list is being touched.
- [Sidebar not running while the window elapses] → the list closes on first
  observation afterwards; the predicate needs no wall-clock continuity. A
  finished list may be seen to close immediately at sidebar start — it
  "should have closed" already.
- [An agent returns later expecting its finished list] → the close rule is
  documented next to the task command in `docs/agent-protocol.md`, and the
  record-first reminder already steers the next turn to a fresh list.

## Migration Plan

None. `/tmp` stores are ephemeral and the close only removes files; rollback
is a revert.
