# Design

## Context

`refresh_pane_tasks` (`src/state/refresh.rs`) reads the store of `focus_state.focused_pane_id` and caches `(pane_id, mtime)` in `TasksState.store_cache`. A pi pmux subagent has its own `$TMUX_PANE`, so its writes go to its own store. The resolved parent is already on the pane: `PaneInfo.subagent.parent` is `SubagentParent::Pane(id)` or `Orphaned`, filled by `resolve_subagent_parents` in `src/tmux/query.rs`.

`group::nest_roots` already walks parent chains, but only inside one repo group and only for panes that the group draws. That limit suits the nesting box. It does not suit a data lookup (see Decision 2).

Claude in-process subagents share the parent's `$TMUX_PANE` and need no change. See proposal.md for scope.

## Goals / Non-Goals

**Goals:**
- One pure function maps a focused pane id to its board owner pane id.
- `refresh_pane_tasks` reads the owner's store and keys its cache on the owner.

**Non-Goals:**
- No change to the Tasks tab renderer, the `task-*` commands, `task_store.rs`, or the close sweep.
- No marker in the tab that says "parent's board". The spec does not ask for one.

## Decisions

**1. Read-side redirect only.** `refresh_pane_tasks` resolves the owner and reads its store. The write commands keep using `$TMUX_PANE`.
Alternative: redirect writes in `task-set`. Rejected: the CLI would have to look up the parent, but the parent pane id exists only after the sidebar's poll runs `resolve_subagent_parents`. A write redirect would also hide the subagent's own record.

**2. A new lookup over all panes, not `nest_roots`.** The helper takes the flat list of panes from all `repo_groups` and follows `SubagentParent::Pane` links until a pane has no subagent identity, an orphaned parent, or a parent that is not a live pane. It uses a visited set. On a loop it returns the pane that tmux lists first, the same tie-break `nest_roots` uses.
Alternative: reuse `nest_roots` per group. Rejected: it returns nothing for a parent in another group, and the status filter can remove the parent from the groups the user sees. The user wants the board in those cases (spec scenario "The parent need not be nested").
Alternative: share one chain walker with `nest_roots` by passing a lookup closure. Possible, but `ultimate_root` is private and coupled to `order`. Prefer a small separate function and a test that both agree on an in-group chain.

**3. Orphan fallback.** The walk stops at the last live pane it reached. An orphaned focused pane never moves, so its owner is itself. A chain that ends on an orphaned middle subagent gives that subagent as owner.

**4. Cache key.** `store_cache` keeps its shape `(String, SystemTime)`, but the string is now the owner id. Focus moves between a parent and its subagent then hit the cache. If the owner changes, the id differs and the store is re-read. The doc comment on the field changes to say "board owner".

**5. Where the pane list comes from.** `AppState.repo_groups` holds every live agent pane. The repo and status filters apply at render time and in the counters, not to this field. The helper iterates `repo_groups` directly.

## Risks / Trade-offs

- [The subagent's own tasks become invisible while focused] → Accepted; the user asked for the parent board. The own store still closes on its normal 90-second rule and is wiped at teardown.
- [A subagent that is focused before the next poll resolves its parent shows its own empty board for one tick] → Accepted; the next refresh corrects it.
- [Reminders: the record-first reminder checks the subagent's own empty store] → Out of scope. The reminder source is the pmux companion, outside this repo. Mention it in the final report.
- [The focused pane id is not in `repo_groups`] → The helper returns the pane id unchanged, so behavior equals today's.

## Open Questions

None.
