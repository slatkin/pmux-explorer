# Proposal

## Why

Each tmux window runs its own sidebar instance. The status filter, the repo
filter and the cursor are meant to be shared through tmux global options, but
an instance reads them again only when its window becomes active after two or
more inactive refresh ticks. That check uses `#{window_active}`, which tmux
keeps per session. A sidebar in another session never becomes "inactive", so
it never reads the shared settings again. A fast window switch (under two
ticks) also skips the read. The user then sees one instance with the "all"
view and another with a narrow view, and it looks as if the agents are gone.

## What Changes

- Every sidebar instance reads the shared status filter, repo filter and
  cursor on each refresh tick, not only on window activation.
- The read is part of the tmux query that each tick already runs for the
  sidebar pane, so the change adds no new tmux process per tick.
- The window-activation gate (`window_inactive_count`) and its two-tick
  debounce are removed.
- A change made in one instance is visible in every other instance within one
  refresh tick (1 second), in the same session or in a different session.
- The settings that are not shared do not change: focus, scroll, tab, popups
  and the git data of the focused pane stay local to each instance.
- No daemon is introduced. The tmux server stays the single shared store.

## Capabilities

### New Capabilities
- `shared-view-settings`: which sidebar view settings all instances on one
  tmux server share, and when an instance picks up a change from another
  instance.

### Modified Capabilities

None.

## Impact

- `src/app.rs`: the refresh loop drops the activation gate.
- `src/state/refresh.rs` and `src/tmux/panes.rs`: the per-tick pane query
  also returns the three shared options, and the refresh applies them before
  it rebuilds the rows.
- `tests/state_tests.rs`: the window-activation sync tests are replaced by
  tests for the per-tick behavior.
- `docs/state-management.md`: the sync timing of the global options is
  updated.
