# Design

## Context

See proposal.md Why for motivation. The current state that shapes the approach:

- Bottom panel: `BottomTab` (`Activity | GitStatus`) in `src/state.rs`, auto-switch and per-pane memory in `src/state/tab.rs`, title/click/scroll dispatch in `src/ui/bottom.rs`, content in `src/ui/bottom/activity.rs` and `src/ui/bottom/git.rs`, scroll in activity state vs `ScrollStates.git`.
- Task data today: Claude `TaskCreate`/`TaskUpdate` tool entries in `/tmp/tmux-agent-activity<PANE>.log`, parsed by `parse_task_progress` in `src/activity.rs`, shown only as a compact row in the agent list. The `TaskCreated` hook event is a no-op. Pi reports no task tool (pinned by `docs/agent-protocol.md` and the `pi-agent-support` gaps requirement).
- Write precedent: `set-status` reads `$TMUX_PANE` and sets a tmux option; activity entries are files with mtime polling.
- Nudge precedent (pmux repo, not this one): one single `COMPLETION_NUDGE_TEXT`, conditional and capped delivery, fire-and-forget publishing.

## Goals / Non-Goals

**Goals:**

- A read-only Tasks tab beside Activity and Git that shows the focused pane's list.
- One shell write path usable by every agent, proven first with pi.
- Reminders that cost zero tokens when the agent already updates tasks.

**Non-Goals:**

- No dedicated pi tool registered by an extension (later step, different repo).
- No human editing of tasks in the TUI; the agent is the only writer.
- No change to hook triggers, adapter tables, or the Claude Task-tool parsing.
- No aggregation view across panes.

## Decisions

**Store is a per-pane file, not a tmux option.** Subjects hold free text with `|` and newlines, which tmux options mangle and length-limit. A file `/tmp/tmux-agent-tasks<PANE>.json` mirrors `log_file_path` encoding, mtime polling, and trim behavior. Alternative (pane options) rejected for escaping and size reasons.

**Tab order Activity, Tasks, Git; Tasks never auto-shows.** Auto-switch keeps today's defaults (new agent pane goes to Activity, non-agent pane goes to Git). Tasks is manual-only plus per-pane memory, so it never steals focus from a working view. Alternatives (auto-show on first task write, or placing Tasks last) rejected: surprise tab jumps break the reading flow the auto-switch rules protect.

**Scroll gets its own offset next to the existing two.** `scroll_bottom` dispatches on the tab; Tasks adds a third arm with the same clamp-on-shrink behavior as Activity. Click hitboxes in `handle_bottom_tab_click` are re-measured for three titles.

**CLI shape mirrors `set-status`.** New subcommands record and update items for `$TMUX_PANE`, exit 0 on every path including missing pane env (same contract as the hook command: never break the agent's turn). Refresh reads the store with the same mtime check as the activity log.

**Reminder wording lives in one place in this repo.** One module holds both reminder strings; the task-command help, `docs/agent-protocol.md`, and the pi proof prompt all reference it. Delivery to pi in the proof is prompt text, not an extension hook. The pmux-extension event hook (input/settle-time injection) is a named companion change in that repo, not this one — this keeps the slice repo-local while fixing the wording both sides will share.

## Risks / Trade-offs

- [Stale lists] An agent records tasks then never updates them → tab shows outdated plan. Mitigation: the end-of-turn reminder fires exactly in that condition; the tab shows recency implicitly through statuses.
- [Log-style unbounded growth] Long sessions accumulate items. Mitigation: cap stored items and trim oldest completed first, mirroring `trim_log_file`.
- [Pane reuse] A pane reused for new work keeps the old list. Mitigation: `session-start` clears the pane's store, matching activity-log session semantics and the `TASK_RESET_MARKER` barrier.
- [Scope split] Pi reminders need a pmux-side companion for the steady state. Mitigation: wording is fixed here first, so the companion change is delivery-only.

## Migration Plan

No migration. Additive tab, additive CLI, additive doc section. Rollback is removal of the tab arm and subcommands. The `pi-agent-support` gaps requirement delta archives with this change.

## Open Questions

- Exact item cap and trim policy (e.g. keep 50, trim completed first): safe to set at implementation time without changing specs.
- Whether the empty-at-start reminder should also cover the resumed-session case where the store was cleared: same condition, likely yes.
