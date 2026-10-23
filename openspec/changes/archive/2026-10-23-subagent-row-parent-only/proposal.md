# Proposal

## Why

The pmux identity row draws `▷ <parent>  <fuzzy task>`. The fuzzy task is the pmux launch name with hyphens as spaces (`rlr-u2a-<timestamp>` ⇒ `rlr u2a`), and pmux now writes that same launch name into the pane title (`<launch name> — <task cut>`), which the status row shows in its title slot. With the pane name already visible on the status row, the task text one row below it repeats the same words in a lighter colour and carries nothing new.

## What Changes

- The subagent identity row shows only the response arrow and the parent label (`▷ <parent>`), or `▷ ORPHANED` when no parent pane resolves. The fuzzy task is no longer drawn on the row.
- The stored identity is unchanged: `@pane_subagent_kind`, `@pane_subagent_parent` and `@pane_subagent_task` keep being written and parsed exactly as before. Only the row rendering drops the task.

## Capabilities

### Modified Capabilities

- `pi-agent-support`: the requirement "a subagent pane shows its parent and task" is replaced by "a subagent pane shows its parent" — the line under the status row names only the parent (or `ORPHANED`).

## Impact

- `src/ui/panes/row/body.rs` — `subagent_identity_row` drops the task piece; the per-piece cut keeps two colour boundaries.
- Tests — the identity-row snapshots in `src/ui/panes/row.rs` and the repo-filter snapshot in `tests/ui_snapshot.rs` need regenerating without the task text.
- Docs — `docs/agent-protocol.md` and `docs/state-management.md` describe the stored options, not the row; only the "each hyphen shown as a space" wording needs a small accuracy fix because nothing shows the task text any more.
