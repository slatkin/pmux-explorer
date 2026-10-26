# Tasks

## 1. Close predicate in the task store

- [x] 1.1 Add `TASK_CLOSE_SECS: u64 = 90` beside `MAX_ITEMS` and the pure `close_due(list, mtime_epoch, now_epoch)` predicate in `src/task_store.rs` (true only for a non-empty list whose every item is `completed` and whose mtime is at least 90 seconds old; a future mtime reads as not due), and verify with unit tests in the file's test module covering the 89/90-second boundary, an empty list, mixed statuses, a single completed item, and a future mtime — `cargo test task_store` passes.

## 2. Per-tick close sweep with its tests and docs

- [x] 2.1 Add per-pane store cache fields (`task_store_mtime`, `task_store_all_done`) with accessors to `PaneRuntimeState` in `src/state/pane_runtime.rs`, verified by default-state and accessor unit tests beside the existing `PaneRuntimeState` tests — `cargo test pane_runtime` passes.
- [x] 2.2 Implement `refresh_task_close` in `src/state/refresh.rs`: sweep every pane in `repo_groups` per tick, stat via `store_mtime`, re-read via `read_task_list` only on mtime change, call `clear_tasks` when `close_due` holds and drop that pane's cache entry; run it at the head of `refresh_activity_data`, before `refresh_pane_tasks`. Verify with sweep tests over real `/tmp` stores (backdate mtime with `std::fs::File::set_modified`, clean up like `task_store`'s tests): a finished list closes after 90 seconds, still shows at 89, an unfocused pane's list closes on time, an added or reopened item keeps the list open, and a refinished list closes 90 seconds after the re-completion.
- [x] 2.3 Reword the write contract from "the TUI only reads" to "the agent writes items and statuses; the sidebar empties a finished list" in the `src/task_store.rs` and `src/ui/bottom/tasks.rs` module headers, and document the 90-second close rule in `docs/agent-protocol.md` ("The task command"), `docs/state-management.md`, and the `AGENTS.md` `task_store` module contract. Verify: `rg "only reads"` finds no stale contract wording, and each named document states the close rule and its add/reopen exception.
- [x] 2.4 Pin the empty state after a close with an inline `insta::assert_snapshot!` in `tests/bottom_tests.rs`: a focused pane with a finished, backdated store, one tick run, and the Tasks tab renders "No tasks recorded" with no description header. Verify: `cargo test bottom_tests` passes with the snapshot inline (UI test rule).

## 3. Gates

- [x] 3.1 Run `cargo fmt`, `cargo clippy`, the full `cargo test` suite, and `openspec validate task-list-auto-close --strict`; verify all pass with no new warnings and the change validates.

## Workflow follow-up

- Archive the change after implementation and review: `openspec archive task-list-auto-close` folds the delta into `openspec/specs/agent-tasks/`.
