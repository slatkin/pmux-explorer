# Design

## Context

See `proposal.md` for the motivation. Current state that shapes the approach:

- The pane order is flat and is built twice from the same walk of `group.panes`: `rebuild_row_targets` (`src/state/layout.rs:465`) builds the selection and click order, and `row_collector::collect` (`src/ui/panes/row_collector.rs:107`) builds the drawn rows. `row_index` in `collect` must equal the index in `pane_row_targets`.
- `resolve_subagent_parents` (`src/tmux/query.rs:259`) runs once per refresh over all panes, before any filter. It maps a recorded parent session id to a *label*.
- Repo and status filters are applied at render time and in `rebuild_row_targets`, not in `group_panes_by_repo`. The status filter hides a pane inside a group; the repo filter hides whole groups.
- `PaneInfo` and `RepoGroup` are built by struct literal in many tests, so a new field on either costs one edit per literal.
- A repo group is keyed by the repo root, so panes in worktrees of one repo share a group. A parent and its subagent normally share a group, but not always.

## Goals / Non-Goals

**Goals:**
- Remove the parent row and show the parent link by indent.
- Keep one definition of the pane order that selection, click and drawing all use.
- Keep every row of a nested pane inside the width left after the indent.

**Non-Goals:**
- No change to the stored `@pane_subagent_*` options, to the in-process subagent tree (`subagent_rows`, `@pane_subagents`), or to the `♟`/`↻` badge.
- No collapse or expand control for children.
- No nesting across repo groups.

## Decisions

### 1. The parent link holds the parent pane id

`SubagentParent::Pane(String)` keeps its shape and now holds the parent's pane id. `SubagentResolution.parent_labels` becomes a map from session id to pane id (`parent_panes`). The first pane that reports a session id still wins.

*Alternative:* keep the label and add an id field. Rejected: the label has no reader once the identity row is gone, and a dead field invites drift.

### 2. The ultimate parent comes from one helper in `src/group.rs`

`nest_roots(panes)` takes a group's panes and returns a map from pane id to its ultimate parent's pane id, for subagent panes that have one. For each subagent it follows `SubagentParent::Pane(id)` while that id is a pane in the same group, and stops at a pane that is not a subagent, whose parent is orphaned, or whose parent is outside the group. A visited-set stops a cycle at the first repeated pane; that pane stays top-level. In a cycle, including a pane that names itself as parent, the pane that comes first in tmux order is the root. Every pane of the group appears exactly once after the reorder.

If the chain stops at a subagent (its parent is orphaned or in another group), that subagent is the root. This matches the confirmed rule "the top of the chain that is reachable in the group".

### 3. Order is fixed once, in `group_panes_by_repo`; indent is decided at draw time

After a group is filled, its panes are reordered so each ultimate parent is followed by its children in their original order. A pane that is not a child keeps its relative position. This single order feeds `rebuild_row_targets` and `collect`, so they stay in step with no change to their loops.

Whether a child is *indented* depends on the filters, which the group order cannot know. A helper `nested_under(root_map, visible_ids, pane_id) -> bool` returns true only when the pane has a root and that root is in the visible (filtered) pane list. `collect` builds `visible_ids` from `filtered_panes`. `rebuild_row_targets` needs no indent, so it only uses the order.

When a filter hides the root, its children stay in the order slot of the root, grouped together, and draw as top-level rows. This is acceptable: they remain a cluster and no row is lost.

*Alternative:* store the root id on `PaneInfo` or `RepoGroup`. Rejected: each adds a field to every test literal; the map is cheap to recompute (one pass over a group of a few panes).

*Alternative:* sort at draw time only. Rejected: `rebuild_row_targets` would need the same sort, and the two copies could drift.

### 4. Indent lives in `RowCtx`

`RowCtx` gets `indent: usize`. `row_line` and `row_line_split` write the marker, the space, then `indent` spaces; both contexts build `inner_width` as `width - 2 - indent`. The `┃` marker stays at column 0, and the selection background still paints the whole row, indent included.

`render_pane_lines` takes a `nested: bool` (its argument list already carries `#[allow(clippy::too_many_arguments)]`) and `render_pane_lines_inline_branch` forwards it. The `sidebar_remove_marker_col` click target in `collect_pane` receives `width - 2 - indent` and adds the indent back. The `×` stays pinned to the rightmost column, so the click column is `width - 1` for any indent.

The strip layout calls the same row functions through `collect`, so it nests too.

### 5. The `ORPHANED` tag is part of the status row

In `status.rs`, when `pane.subagent` holds `SubagentParent::Orphaned`, a span ` ORPHANED` in `theme.status_error` follows the badge. Its width is added to `fixed_width` so the pane name and the lifted branch label shrink first. No tag for any other subagent state.

### 6. `subagent_identity_row` is deleted

Together with its call in `render_pane_lines` and its import. No other code reads the parent label.

## Risks / Trade-offs

- [A hidden root splits the visual order] → Children stay clustered at the root's slot; documented in the decision above and pinned by a snapshot.
- [The indent costs 2 columns on a narrow sidebar] → All rows use `inner_width - indent` and the existing truncation; a narrow-width snapshot pins it.
- [`rebuild_row_targets` and `collect` could disagree] → Both read the same reordered group; a test asserts that the target order equals the drawn pane order for a group with children.
- [A parent in another repo group draws the child as top-level without a hint] → Accepted by the confirmed design; the badge still marks it as a subagent.
- [Cycle in parent session ids] → The visited-set in `nest_roots`; a test covers a two-pane cycle.

## Migration Plan

No data migration. The tmux options do not change. Ship with the normal build; roll back by reverting the commit.

## Open Questions

None.
