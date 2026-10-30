# Proposal

## Why

A pmux subagent pane spends one extra row on `▷ <parent label>` just to say who owns it. The agent list is short on rows, and the same fact can be shown by position: a subagent drawn indented under its parent needs no label row and reads as a tree.

## What Changes

- The subagent identity row (`▷ <parent>` / `▷ ORPHANED`) is removed. A subagent pane gets one row fewer.
- A subagent pane is drawn indented by 2 columns, directly after its **ultimate parent** in the same repo group. All nesting depths use the same indent. The ultimate parent is the top of the parent chain that is reachable in the group.
- Children keep tmux order under their ultimate parent. Keyboard selection and mouse clicks walk the new order, so the selected row index and the drawn row index stay equal.
- A subagent that cannot nest stays a top-level row:
  - its parent session is not found (`ORPHANED`);
  - its parent pane is in another repo group;
  - the status filter hides the ultimate parent.
- Only an `ORPHANED` subagent carries a tag: the word `ORPHANED` in the theme's error color on its status row, after the badge. The other fallback rows carry no tag.
- The parsed parent now holds the parent pane id instead of the parent's display label. The label has no reader once the identity row is gone.
- The stored options `@pane_subagent_kind`, `@pane_subagent_parent` and `@pane_subagent_task` do not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pi-agent-support`:
  - "the parent is resolved at each refresh" now resolves to the parent pane id, and drops the "shown by its title" and "still shows the parent when the filter hides it" behaviors.
  - "a subagent pane shows its parent" is replaced by "a subagent pane is nested under its parent".
  - A new requirement covers the `ORPHANED` tag.

## Impact

- `src/tmux/types.rs` and `src/tmux/query.rs` — `SubagentParent::Pane` holds a pane id; `SubagentResolution` maps session id to pane id.
- `src/group.rs` — `group_panes_by_repo` orders a group so each ultimate parent is followed by its children, and exposes the nest-root lookup.
- `src/state/layout.rs` (`rebuild_row_targets`) and `src/ui/panes/row_collector.rs` (`collect`) — both decide nesting from the filtered pane list with the same helper.
- `src/ui/panes/row.rs`, `row/ctx.rs`, `row/body.rs`, `row/status.rs` — an indent in `RowCtx`; `subagent_identity_row` is removed; the status row gets the `ORPHANED` tag. The strip layout shares these row functions, so it follows.
- Tests — the identity-row snapshots in `src/ui/panes/row.rs` and `snapshot_subagent_identity_survives_repo_filter_hiding_parent` in `tests/ui_snapshot.rs` change; `query.rs` parent-label tests change to pane ids.
- Docs — `docs/agent-protocol.md` line 102 ("the identity row below the status row names only the parent") needs a rewrite.
