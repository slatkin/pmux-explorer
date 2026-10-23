# Design

## Context

See proposal.md - Why for the problem. The facts that shape the fix:

- The bottom tab is currently local: `AppState::bottom_tab` (`src/state.rs`)
  is written by `next_bottom_tab`, `handle_bottom_tab_click`, and
  `auto_switch_tab` (`src/state/tab.rs`). `auto_switch_tab` runs from
  `refresh_activity_data` (`src/state/refresh.rs`) on every 1s tick and on
  SIGUSR1, so a focus change rewrites the tab continuously.
- `GlobalState` (`src/state/global.rs`) already owns the shared
  settings: `status_filter`, `selected_pane_row` and `repo_filter` are
  written with `set -g @sidebar_*` and read back through `apply_all`, which
  only adopts a value that differs from the matching `last_saved_*`. That
  guard is what keeps a failed write from reverting the user's choice.
- The read side rides one per-tick `display-message`: `SHARED_OPTION_FIELDS`
  in `src/tmux/panes.rs` appends the shared options to the pane-state
  fields, and `parse_sidebar_pane_reading` derives its split count from the
  same list. The repo filter is last because a repo name may contain
  spaces.
- `git_tab_active` (`AtomicBool`, `src/app/workers.rs`) gates the git
  background thread. `src/app.rs` re-stores it from `state.bottom_tab`
  after every refresh tick, and the two input paths store it immediately on
  a manual switch.
- `AppState::new` starts on `BottomTab::Activity`; `load_from_tmux` runs
  before the first `refresh()` in `src/app/setup.rs`.

## Goals / Non-Goals

**Goals:**
- One selected bottom tab per tmux server, adopted by every instance
  within one refresh tick, through the existing shared-option read.
- No tab change from focus movement, agent start, or pane relaunch.
- No extra tmux subprocess per tick or per tab switch.

**Non-Goals:**
- Sharing scroll offsets, focus, popups, or git data — those stay local
  (the unchanged part of the `shared-view-settings` spec).
- Sharing anything across tmux servers; the tmux server remains the
  boundary, as for the other shared settings.
- A push channel (SIGUSR1 on tab change, a socket, a daemon). The 1s
  poll already satisfies the one-tick requirement.
- Changing the tab set (Activity/Tasks/Git) or the cycle order.

## Decisions

**D1: The tab lives in `GlobalState`, not on `AppState`.**
`bottom_tab` moves to `GlobalState` beside `status_filter`, with its own
`last_saved_bottom_tab` and a `save_bottom_tab` writing
`@sidebar_bottom_tab` via `set -g`. This mirrors the filter exactly, so
the failed-save guard and the pending-write retry come from the same shape
the spec already pins for shared settings.
*Alternative:* keep `bottom_tab` on `AppState` and have `apply_all` return
an adopted tab. Rejected: split ownership — the guard would live in one
struct and the value in another, and every future writer could bypass the
save path by assigning the field.
Production reads of `state.bottom_tab` are handfuls (UI title/content,
`git_tab_active` stores, the cycle key); tests touch it most and are being
rewritten anyway. Call sites become `state.global.bottom_tab`.

**D2: The tab rides `SHARED_OPTION_FIELDS`, inserted before the repo
filter.** The list becomes `[SIDEBAR_FILTER, SIDEBAR_CURSOR,
SIDEBAR_BOTTOM_TAB, SIDEBAR_REPO_FILTER]`; the query string, the parse
split count and the options map all derive from it, so the field order
cannot drift. `activity` / `tasks` / `git` contain no space, so the
space-split parse and the `display_message` trim are safe, and the repo
filter keeps its terminator-based last-field parse.
*Alternative:* a separate `show-options` call for the tab. Rejected: one
extra tmux process every second for a value that already has a channel.

**D3: Adoption order inside `refresh()` is retry-then-read, matching the
filter.** A rejected `set -g @sidebar_bottom_tab` sets a
`tab_save_pending` latch; `apply_all` skips adoption while the latch is
set (tmux holds the stale value the write was meant to replace), and the
retry runs at the top of the next tick, before the read. On success the
latch clears and `last_saved_bottom_tab` advances.
*Alternative:* no latch, write once. Rejected: a single transient failure
would strand the server on the old tab while the writing instance shows
the new one — every later tick would keep the two apart.

**D4: `auto_switch_tab` and the per-pane tab memory are deleted, not
disabled.** `auto_switch_tab`, `TabDecision`, `resolve_bottom_tab`,
`resolve_tab_for_focused_pane`, `save_current_tab` and `detect_new_agents`
go, along with `PaneRuntimeState.tab_pref` and `PaneRuntimeMap.seen`
(the seen set exists only to feed `detect_new_agents` and is referenced
nowhere else). The refresh tick calls nothing tab-related except applying
the shared value.
*Alternative:* keep the per-pane preference and restore it only when the
tab was never shared. Rejected: two rules for one setting, and the restore
would fight the shared value on every focus change — exactly the behavior
the spec removes.

**D5: Every tab write funnels through one `save` path.** The cycle key
(`next_bottom_tab`) and the title click (`handle_bottom_tab_click`) update
the field and then call `save_bottom_tab`; there is no other writer. The
input handlers keep their immediate `git_tab_active` store, and
`src/app.rs` keeps its post-refresh store, which covers instances that
adopt the value instead of writing it.

**D6: The startup default is Activity, from `AppState::new`.** When the
option is unset (empty expansion, dropped from the options map the same
way an unset filter is), `apply_all` adopts nothing and the instance keeps
its constructed Activity tab. No special "first instance writes its
default" step: the server adopts Activity as soon as anyone selects a tab.

## Risks / Trade-offs

- [Two instances select different tabs within one tick] → Last writer
  wins; within one tick both converge on whatever tmux holds. No revert
  loop is possible because adoption only fires on a value that differs
  from `last_saved_*`.
- [A user who relied on Activity auto-showing for a new agent loses it] →
  Accepted and pinned by spec: the tab moves only on explicit selection.
- [A rejected tab write leaves instances apart] → The D3 latch retries
  once per tick until tmux accepts, same as the filter.
- [Removing `tab_pref` / `seen` breaks a test that asserted relaunch
  detection] → Those assertions exist only for tab behavior; they are
  replaced by shared-tab tests in the same task that removes the fields.
- [`git_tab_active` goes stale in an instance that adopted Git from a
  peer] → `src/app.rs` re-stores the flag after every refresh tick, which
  is the tick that performs the adoption.

## Migration Plan

No data migration. On upgrade the option is simply unset, so every
instance shows Activity until someone selects a tab; from then on the
selection persists in `@sidebar_bottom_tab` for the life of the tmux
server. Rollback is free: an older binary ignores the unknown
`@sidebar_bottom_tab` option and returns to local tabs.

## Open Questions

None — the spec fixes the shared-ness, the focus behavior, and the
startup default; the remaining choices are all resolved in Decisions.
