# Proposal

## Why

A pmux subagent is a pi process in its own tmux pane, so it has its own `$TMUX_PANE` and its own task store. The Tasks tab reads the store of the focused pane. When the user focuses a subagent pane, the tab shows the subagent's own store, which is almost always empty. The user wants the board of the work the subagent belongs to, not an empty list.

## What Changes

- The Tasks tab shows the board of the focused pane's **board owner**. For a pi pmux subagent with a live parent, the board owner is its ultimate parent pane. For every other pane, the board owner is the pane itself.
- The ultimate parent is found by following the `SubagentParent::Pane` chain over all panes. The chain does not depend on the repo group, the nesting box, or the status filter.
- An orphaned subagent (no live parent pane) shows its own board.
- The redirect is read-only. A subagent's `task-set`, `task-update` and `task-describe` still write the subagent's own store.
- The task cache key uses the board owner id, so a focus move between a parent and its subagent does not re-read the store.

Out of scope:

- Redirecting writes to the parent store.
- Claude in-process subagents. They share the parent's `$TMUX_PANE` and already write the parent's store.
- The 90-second close sweep. It keeps closing each pane's own finished list.
- Reminder wording and reminder timing. The pmux companion owns them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-tasks`: the requirement "The Tasks tab shows the focused pane todo list" changes so that a subagent pane shows its parent's list.

## Impact

- `src/state/refresh.rs`: `refresh_pane_tasks` resolves the board owner before it reads the store.
- `src/group.rs` or `src/state/`: a pure helper that walks the parent chain over all panes, with cycle protection.
- `src/state/tasks.rs`: the `store_cache` key and its doc comment.
- `docs/state-management.md` and `docs/agent-protocol.md`: one note each on the board owner.
- Tests for the helper and for `refresh_pane_tasks`; the Tasks tab render needs no change.
