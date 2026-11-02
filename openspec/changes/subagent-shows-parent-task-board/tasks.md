# Tasks

## 1. Board owner lookup

- [ ] 1.1 Add a pure function in `src/group.rs` that maps a pane id and the unfiltered `repo_groups` to the board owner pane id. It follows `SubagentParent::Pane` links across all groups, stops at a non-subagent, an `Orphaned` parent, or a parent pane that is not in the groups, and ends a loop at the pane tmux lists first. Verify with unit tests for: a plain pane, a direct child, a two-level chain, a parent in another group, an orphan, a missing parent pane, a loop, and a pane that names itself. Run `cargo test group`.
- [ ] 1.2 Add a test that the new function and `nest_roots` give the same root for an in-group chain. Verify with `cargo test group`.

## 2. Tasks tab reads the owner's store

- [ ] 2.1 Change `refresh_pane_tasks` in `src/state/refresh.rs` to resolve the board owner of `focused_pane_id` and read that pane's store. Key `store_cache` on the owner id. Update the doc comments on `refresh_pane_tasks` and on `TasksState.store_cache` in `src/state/tasks.rs`. Verify with `cargo test refresh_pane_tasks`.
- [ ] 2.2 Add `refresh_pane_tasks` tests: a focused subagent shows the parent's items and description; a focused orphan shows its own items; a subagent's own store is not read while the parent is live; a focus move from parent to subagent does not re-read the store (cache hit); a parent in another group is followed. Verify with `cargo test refresh_pane_tasks`.
- [ ] 2.3 Verify that a subagent's `task-set` still writes its own store and leaves the parent's store unchanged, with a test in `src/cli/mod.rs` next to the existing task tests. Run `cargo test task`.

## 3. Documentation and gates

- [ ] 3.1 Add one note on the board owner to `docs/state-management.md` (the Tasks state) and one to `docs/agent-protocol.md` (the pmux subagent section: the sidebar shows the parent's board for a subagent pane, and a subagent's own writes stay in its own store). Verify by reading both files for the new text.
- [ ] 3.2 Run `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo build --release`. Verify all four succeed with no warnings.
- [ ] 3.3 Run `openspec validate subagent-shows-parent-task-board --strict` and verify it passes.

## Workflow follow-up

- Archive the change after review. The archive merges the `agent-tasks` delta into the main spec.
- Report to the user that the record-first reminder, owned by the pmux companion, still reads the subagent's own empty store.
