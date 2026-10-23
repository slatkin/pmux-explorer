# Proposal

## Why

Each tmux window runs its own sidebar instance, and each sidebar pane keeps
its own width. Resizing the sidebar in one window changes nothing anywhere
else, so switching windows jumps between whatever width each pane happened
to get — at creation from `@sidebar_width`, after a layout squeeze grew it
back to `@sidebar_min_width`, or after a manual resize. The user expects
one docked width for the whole tmux server when they ask for it: a lock
that makes every sidebar the same width, while leaving per-window widths
as the default for users who want them.

## What Changes

- A width lock joins the shared view settings: a new tmux global option
  (`@sidebar_width_lock`) holds the on/off state, shared by every sidebar
  instance on the server, default off.
- While the lock is on, a second global option (`@sidebar_docked_width`)
  holds the shared docked width in columns. An instance that sees its own
  docked pane width change publishes the new width; an instance whose pane
  width is stable adopts the shared width with `resize-pane -x`, within
  one refresh tick, whether its window is visible or not.
- Enabling the lock seeds the shared width from the enabling instance's
  current docked width, clamped at the `@sidebar_min_width` floor — a
  floating instance seeds its captured docked width instead. Disabling
  it leaves every sidebar at the width it already has; later resizes
  stay local again.
- A resize below the `@sidebar_min_width` floor is treated as a layout
  squeeze, not a user choice: it is never published and the pane is grown
  back toward the shared width (the floor is folded into the lock's
  enforcement while the lock is on).
- The adopted width is capped so the sidebar never takes more than half
  the window's columns, so a narrow window does not lose its workspace to
  a width synced from a wider one.
- A floating sidebar is outside the lock: the popped-out panel stays at
  its fixed 40 columns, neither publishes nor adopts, and popping back in
  restores the shared width while the lock is on.
- The sidebar's top row gains a lock control next to the pop-out control,
  showing the lock state and toggling it on click or on a `w` keypress;
  the change applies to every instance on the server within one tick.
- A sidebar created while the lock is on (the `toggle` subcommand) starts
  at the shared width instead of `@sidebar_width`, so a new window does
  not flash at a different width before its first tick.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-view-settings`: the docked width becomes a shared setting while
  the width lock is on, the lock state itself is shared, and the failed
  save and one-tick adoption rules gain width-specific requirements.
- `sidebar-panel`: the docked sidebar gains the publish/adopt behavior,
  the squeeze floor, the half-window cap, the floating-panel exclusion,
  the shared-width pop-in restore, and the lock control in the top row.

## Impact

- `src/tmux/options.rs`: new `SIDEBAR_WIDTH_LOCK` (`@sidebar_width_lock`)
  and `SIDEBAR_DOCKED_WIDTH` (`@sidebar_docked_width`) global option
  constants.
- `src/tmux/panes.rs`: both options added to `SHARED_OPTION_FIELDS` before
  the repo filter, riding the existing per-tick `display-message` read
  with no extra tmux process; `parse_sidebar_pane_reading` gains the two
  positional fields (both are space-free values). `@sidebar_min_width`
  rides the same read, since the lock needs the floor on every tick in
  every window, not only while the window is active.
- `src/state/global.rs`: `width_lock` and `shared_docked_width` join
  `GlobalState` with `last_saved_*` guards and save/pending-retry latches,
  following the status filter and bottom tab pattern.
- `src/app/`: a width-lock step next to `size_guard`/`panel_guard` in the
  refresh block — publish on an observed local width change, adopt on a
  shared value change, running every tick regardless of window focus so
  hidden windows converge before the user switches to them.
- `src/app/size_guard.rs`: while locked, the floor logic is subsumed by
  the shared-width enforcement (grow toward the shared width, which the
  publish rule keeps at or above the floor).
- `src/app/input.rs`: `w` key toggles the lock through the shared-option
  write path.
- `src/ui/panes/filter_bar.rs` and `src/state/layout.rs`: lock glyph
  beside the pop-out control in the filter bar's control column,
  clickable through the same `filter_bar_layout` / `handle_filter_click`
  hit-test as the pop-out control.
- `src/state/popout.rs`: `apply_pop_in` restores the shared width,
  floor-and-cap clamped, instead of the captured docked width while the
  lock is on.
- `src/cli/toggle.rs`: `cmd_toggle` prefers `@sidebar_docked_width` over
  `@sidebar_width` for `split-window -l` while the lock is on.
- `openspec/specs/shared-view-settings/spec.md` and
  `openspec/specs/sidebar-panel/spec.md`: requirement changes recorded as
  deltas in this change.
- `docs/state-management.md`: `width_lock`/`docked_width` join the Global
  scope table with their tmux variables and update triggers.
- `agent-sidebar.conf` and
  `website/src/content/docs/reference/tmux-options.md`: the two new
  options documented.
