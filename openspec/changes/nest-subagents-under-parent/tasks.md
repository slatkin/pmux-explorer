# Tasks

## 1. Parent link holds the parent pane id

- [x] 1.1 In `src/tmux/query.rs`, change `SubagentResolution` to map a session id to the pane id (`parent_panes`) instead of a label, keep "the first pane that reports a session id wins", and update the `SubagentParent::Pane` doc in `src/tmux/types.rs`. Verify with `cargo test resolve_subagent_parents`: the three tests that assert a label now assert the parent pane id, including the cross-session and the no-title cases.

## 2. One pane order for selection, click and drawing

- [x] 2.1 In `src/group.rs`, add `nest_roots(panes)` (pane id to ultimate parent pane id, same-group only, visited-set cycle guard, an orphaned or out-of-group parent ends the chain) and reorder each group in `group_panes_by_repo` so each root is followed by its children in tmux order. Verify with unit tests in `group.rs`: one child, a grandchild (same root as the child), two children in tmux order, parent in another group, orphaned root with its own children, and a two-pane cycle and a self-parent, where the root is the pane first in tmux order and the reordered group has as many panes as the input.
- [x] 2.2 Add `nested_under(root_map, visible_ids, pane_id)` beside `nest_roots` and unit-test it: true only when the root is in the visible list. Verify with `cargo test nested_under`.
- [x] 2.3 Add a test in `src/state/layout.rs` that `rebuild_row_targets` lists a parent, then its children, then the next top-level pane, and that a status filter hiding the root keeps the children in the root's slot. Verify with `cargo test rebuild_row_targets`.

## 3. Draw the nest and drop the parent row

- [x] 3.1 Add `indent` to `RowCtx` in `src/ui/panes/row/ctx.rs` (written by `row_line` and `row_line_split` after the marker and space), pass `nested: bool` through `render_pane_lines_inline_branch` and `render_pane_lines` in `src/ui/panes/row.rs`, and build both contexts with `inner_width = width - 2 - indent`. Verify with an inline `insta` snapshot of a nested pane at width 40 and at a narrow width, where every row is cut inside the indent and the row count equals the un-nested row count minus the removed parent row.
- [x] 3.2 Delete `subagent_identity_row` and its call and import (`src/ui/panes/row/body.rs`, `src/ui/panes/row.rs`). Verify the build with `cargo build` and that the identity-row tests in `row.rs` are removed or rewritten as nested-row snapshots; a response preview below the status row of a subagent still renders.
- [x] 3.3 In `src/ui/panes/row/status.rs`, show ` ORPHANED` in `theme.status_error` after the badge for `SubagentParent::Orphaned`, counted in `fixed_width`. Verify with `insta` snapshots: an orphaned subagent shows the tag; a resolved subagent shows none; a long pane name still truncates before the tag.
- [x] 3.4 In `src/ui/panes/row_collector.rs`, compute `nest_roots` per group and `visible_ids` from `filtered_panes`, pass `nested` to each pane, and pass `width - 2 - indent` to `sidebar_remove_marker_col`, so the click column stays `width - 1`. Verify with a test that the remove-marker click column of a nested spawned worktree pane lines up with the drawn `×`, and a frame snapshot of a group with a parent and two children (`line_to_row` maps each child row to its own `row_index`).
- [x] 3.5 Rewrite `snapshot_subagent_identity_survives_repo_filter_hiding_parent` in `tests/ui_snapshot.rs` as the parent-in-another-group case: the subagent is a top-level row with no indent and no tag (rename the test to match). Add one snapshot for the status filter hiding the root. Verify with `cargo test --test ui_snapshot`.
- [x] 3.6 Add a strip-layout snapshot with a nested subagent in `tests/ui_snapshot.rs`. Verify the strip's agent panel shows the indent and keeps the branch on the status row.

## 4. Docs and full check

- [x] 4.1 Rewrite the sentence in `docs/agent-protocol.md` (around line 102) that says the identity row names the parent, to say the subagent is drawn under its parent and an orphan carries a tag; align the `@pane_subagent_parent` wording in `docs/state-management.md` if it names a parent label. Verify with `rg -n "identity row" docs src` returning no stale mention.
- [x] 4.2 Update the matching sentences in the project instructions (`CLAUDE.md`, "pi support and the pmux subagent") to the nested rendering. Verify by re-reading the paragraph against the final behavior.
- [x] 4.3 Run `cargo fmt`, `cargo clippy`, `cargo test` and `cargo build --release`. Verify all four exit with status 0.
