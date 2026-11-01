# Proposal

## Why

Nothing owns the state of an agent pane. Each `on_*` hook handler builds its
own list of `set_pane_option` / `unset_pane_option` calls, and the sidebar's
own sweeps write the same options. The rules that decide what a write may do
are spread over those callers, and several are wrong or missing:

- `status_priority.rs` documents a status order that nothing enforces. The
  real rule is last writer wins.
- A late `activity-log` event turns `idle` or `background` back into `running`
  (`cli/hook/activity.rs:82`).
- An omitted `session_id` or `worktree` unsets the pane option
  (`cli/hook/context/location.rs`), while `docs/agent-protocol.md` says an
  omitted field is "not reported". A pi `stop` event omits `worktree`, so every
  pi stop clears the worktree metadata.
- `pane_writes_allowed` (the subagent guard) is opt-in per write, so most
  status, attention and prompt writes skip it.
- `@pane_wait_reason` is written raw. A newline in it splits the `list-panes`
  record and the pane drops out of the sidebar.
- One `Stop` makes about twenty tmux calls, so the one-second poll can read a
  half-finished transition.
- The sidebar's bg-shell sweep (`state/refresh.rs`) writes `@pane_status`
  outside every hook rule.

Issue 15, finding 1, names the missing seam. The rules need one owner.

## What Changes

- Add a pure transition function: given a snapshot of the pane's options and an
  `AgentEvent`, it returns the list of option writes. The priority order, the
  subagent guard and the not-reported rule live there and nowhere else.
- Add a `PaneStore` seam with two adapters: a tmux adapter that applies a
  whole write list in one batched `tmux` call, and an in-memory adapter that
  replaces the `cfg(test)` `tmux::test_mock`.
- Route every write to an agent `@pane_*` option through the store: the hook
  handlers and the dispatch-inline writes in `cli/hook.rs` (`cwd-changed`,
  `task-completed`), the `set-status` subcommand, the activity-log handler,
  the notification stamps and run id, the pending-teardown markers, the
  bg-shell sweep in `state/refresh.rs` and the option wipes of the three
  teardown paths. Option reads the rules do not need (notification titles,
  dedup stamps) go through the store too, so `tmux::test_mock` is fully
  replaced.
- Encode every written value in one place, so a value is always one field.
  This closes the newline-in-`wait_reason` hole.
- **Behaviour fix:** an `activity-log` event may move a pane out of `waiting`,
  and may not move it out of `idle` or `background`.
- **Behaviour fix:** a field clears a pane option only when the event is
  declared to report that field. Each adapter's event table gains that
  declaration. A pi `stop` stops clearing worktree metadata.
- Update `docs/agent-protocol.md` where it describes "not reported".

Out of scope:

- The `@sidebar_*` global options and the sidebar's own pane options
  (`@pane_role`, `@pane_sidebar_layout`, `@sidebar_pid`). They have no event,
  no previous state and no rules to own.
- The unification of the three teardown rituals, the pure parser and the single
  liveness rule (issue 15, finding 6). Only their option writes move onto the
  store.
- The `list-panes` schema table (finding 2) and the adapter context parsing
  (finding 9), except for the small "reports" list in the event tables.
- The read-modify-write of `@pane_subagents` and of the task store.

## Capabilities

### New Capabilities
- `pane-state-writes`: the rules for writing an agent pane's state: one owner
  for the writes, the status priority, the subagent guard, the not-reported
  rule, one-field encoding, and the all-or-nothing batch.

### Modified Capabilities

None. The existing `pi-agent-support` requirements stay true. The pi `stop`
scenario gains no new text, because the new capability states the worktree
rule for every agent.

## Impact

- Code: `src/cli/hook.rs` (the dispatch arms), `src/cli/hook/handlers/*`,
  `src/cli/hook/activity.rs`, `src/cli/hook/context/*`,
  `src/cli/hook/notifications.rs`, `src/desktop_notification.rs` (the stamp
  separator), `src/cli/mod.rs` (`set_status`, `set_attention`,
  `sanitize_tmux_value`, `set-status`), `src/tmux/options.rs`,
  `src/state/refresh.rs`, `src/tmux/query.rs` (wipe call only), `src/adapter/*`
  (the "reports" list).
- Docs: `docs/agent-protocol.md`, `docs/state-management.md`.
- Behaviour visible to a user: an idle pane no longer flips to running on a
  late tool event; a pi pane keeps its worktree label across turns; a wait
  reason with a newline no longer drops the pane.
- No new dependency. No change to the hook command line or to the payload shape.
