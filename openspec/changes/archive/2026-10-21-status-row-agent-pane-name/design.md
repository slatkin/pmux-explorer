# Design

## Context

The status row (`src/ui/panes/row/status.rs`) renders `icon  title  badge … elapsed`, where the title is the Claude session name when the pane has one and the agent label otherwise. The pane query (`src/tmux/query.rs`) already fetches `pane_title` in the same `list-panes` line, but `parse_pane_fields_with_processes` drops it: `PaneInfo` has no title field, and `pane_title` is consumed only inside `build_session_hierarchy` to label a subagent's parent.

tmux facts the design rests on (checked on tmux 3.8 and in `~/Dev/pmux`): `pane_title` defaults to the host name and agents overwrite it with OSC titles (pi writes `π - …`, observed as `π - send-report-channel`; Claude Code writes `✳ …`). `automatic-rename` is a window option that only affects the window name, so it does not tell whether a pane title is a name. pmux names a pane with `tmux select-pane -T <name>` (`pi-extension/pmux/tmux.ts:363`) and sets no pane option and no `automatic-rename`. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Agent label always visible in the title slot, pane name appended when real.
- Zero new tmux calls; the name is resolved from data the tick already fetches.
- Truncation keeps the elapsed label visible and never cuts the agent label.

**Non-Goals:**
- Surfacing the session name anywhere else (a later change can revive it).
- Renaming the badge slot's layout; the badge keeps its position after the title.
- Changes to the subagent identity row, the child-tree rows, or the parent-label resolution.

## Decisions

**Resolve the pane name in `parse_pane_fields_with_processes`, store `Option<String>` on `PaneInfo`.**
The pane-line format gains one trailing field, `host` (`#{host}`), after `pane_title`. Both inputs are then per-pane fields, so the per-pane parse resolves the name: `pane_name: Some(title)` unless the title is empty, equals the host field, or starts with `π - ` or `✳ `; `None` otherwise. The renderer stays dumb: it formats an `Option<String>`.
Alternatives rejected: gating on the window's `automatic-rename` (false premise, see Context); plumbing window context through `RowCtx` (not needed once the gate is per-pane); reading the title in the renderer (the host name is not available there); a separate tmux call for the host name (the field in the existing line costs nothing).
Known gap: agent-set titles other than pi's and Claude's (Codex, opencode) are not filtered and show as pane names.

**Truncation budget: reserve the agent label, cut the pane name.**
The existing arithmetic reserves icon, badge, and elapsed before sizing the title. The agent label's width joins the fixed reservation; the pane name gets what remains and takes the ellipsis when cut. The elapsed-truncation fallback stays as is. The agent label is at most nine columns (`opencode`), so reserving it cannot starve the elapsed label the way an unbounded session name did.

**Resumed badge becomes `↻`, spawned stays ♟.**
One match arm in `status_row` (`PmuxTaskKind::Resumed => ("↻", Some(theme.badge_auto))`). Both glyphs are single-width in the codebase's font stack, so the existing `display_width` badge math is unchanged. Alternative rejected: a second chess piece (♚) — it implies a rank hierarchy that does not exist and collides with the parent pane's identity.

**Remove the session-name pipeline outright.**
With the status-row use gone, `PaneInfo.session_name` has no production consumer (remaining hits are test fixtures). Leaving it would be dead code: `src/session.rs` (the `~/.claude/sessions` scanner) and `pub mod session` in `src/lib.rs`, the `session_poll_loop` thread in `src/app/workers.rs`, the initial scan in `src/app/setup.rs`, `refresh_session_names` and the `sessions.names` map in `src/state/`. Remove all of it rather than keep a map nobody reads.
Alternative rejected: keeping the pipeline but unused — the repo's own standards reject silent dead weight, and the field would still invite future readers to think it is live.

**Spec hygiene: the badge spec text lags main.**
`pi-agent-support`'s "a subagent pane carries a badge" still names the word `subagent`, but PR 4 already shipped ♟ on main. The delta MODIFIES that requirement to the shipped ♟ plus the new ↻, so the spec and the code converge in one change.

## Risks / Trade-offs

- [Snapshot churn] Every status-row snapshot changes because the agent label now always shows. → Expected; `cargo insta accept` once, and the diff doubles as the visual review of the new composition.
- [pmux names on non-task panes] pmux names panes it spawns for any purpose; an agent pane named by pmux will now show that name even when it is not a subagent. → Correct behavior: the name is real, so showing it is honest.
- [Ambiguous-width glyphs later] If the badge glyph ever changes to an East Asian ambiguous character, `display_width` math silently breaks. → Mitigation: keep badge glyphs to proven single-width shapes; note it next to the badge constant.
- [Removing the session pipeline touches the `app/workers.rs` thread wiring] → Small, mechanical removal; the poll loop has no other side effects than filling the map. Its removal is covered by compile + test gates.

## Migration Plan

Single commit series on one branch; no data, config, or tmux-state migration. Rollback is reverting the branch. The spec delta archives into `status-row` (new) and `pi-agent-support` (badge requirement).

## Open Questions

None.
