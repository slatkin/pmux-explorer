# Design

## Context

See proposal.md - Why for the problem. The facts that shape the fix:

- `GlobalState` (`src/state/global.rs`) already writes the three settings to
  `@sidebar_filter`, `@sidebar_cursor` and `@sidebar_repo_filter` with
  `set -g`. The write side works. Only the read side is gated.
- `GlobalState::apply_all` already has the guard the spec needs: it changes a
  field only when the tmux value differs from the matching `last_saved_*`
  value. A failed save leaves `last_saved_*` at the old value, which is equal
  to what tmux holds, so the old value does not come back. A pending cursor
  save (120 ms debounce) works the same way: tmux still holds the value that
  is equal to `last_saved_cursor`.
- The read happens in `src/app.rs` only when `is_window_active` is true and
  `window_inactive_count >= 2`. `is_window_active` comes from
  `#{window_active}`, which tmux keeps per session.
- Each tick, `AppState::refresh` (`src/state/refresh.rs`) already runs
  `display-message -t <sidebar pane>` through `get_sidebar_pane_info`, and
  then calls `apply_session_snapshot`, which calls `rebuild_row_targets`.

## Goals / Non-Goals

**Goals:**
- Read the three shared settings on every refresh tick, with no extra tmux
  process.
- Keep the `last_saved_*` guard as the only protection against reverting a
  local change.

**Non-Goals:**
- A daemon, a socket, or a shared cache of the pane snapshot.
- Push updates. The sidebar keeps its 1 second poll, plus the SIGUSR1
  refresh it already has.
- A new SIGUSR1 hook for `client-session-changed`. The spec allows up to one
  tick of delay, and the tick already covers it.
- Changes to which settings are shared.

## Decisions

### Read the options in the existing per-tick `display-message`

Extend the format of the sidebar pane query to also expand
`#{@sidebar_filter}`, `#{@sidebar_cursor}` and `#{@sidebar_repo_filter}`.
tmux resolves a user option on a pane target through pane, window, session
and then global scope, so the global value comes back in the same call.

The repo filter is a directory name and can contain spaces, so it is the last
field, and the parse takes the rest of the line for it. A field that expands
to an empty string means "option not set". It is left out of the map given to
`apply_all`, which keeps the current behavior for an unset option (no
change).

Alternatives:
- Call `get_all_global_options` (`show -g`) every tick. This adds one tmux
  process per tick for each instance. Rejected.
- Add the options to the `list-panes -a` format. This repeats the same three
  values on every pane line of the server. Rejected.

### Apply the options inside `AppState::refresh`, before the rows rebuild

`refresh` applies the shared settings before `apply_session_snapshot`. The
rows are then rebuilt in the same tick by the `rebuild_row_targets` call that
`apply_session_snapshot` already makes. No "changed" flag or second rebuild
is needed.

### Remove the window-activation gate

`window_inactive_count`, the `load_from_tmux` call and the extra
`rebuild_row_targets` call go away from `src/app.rs`. The gate was added to
stop a short `window_active` flicker during hook processing from causing a
false sync. With the `last_saved_*` guard, a read changes a field only when
another instance wrote a new value, so a flicker cannot cause a false sync.
`is_window_active` stays in use for `size_guard::enforce`.

`load_from_tmux` stays for the startup read in `src/app/setup.rs`.

## Risks / Trade-offs

- [Two instances change a setting in the same tick] → The last write to tmux
  wins in every instance. The user works in one pane at a time, so this needs
  a write from code, not from the user. Accepted.
- [A transient empty pane list resets the repo filter for every instance] →
  `rebuild_row_targets` resets a repo filter whose repo is gone and saves
  "all" to tmux. If `list-panes` fails for one tick, the list is empty and the
  reset happens. This is how the code works today. The change only makes the
  reset reach the other instances faster. It is not fixed here, and it is
  noted for a separate change.
- [The cursor is a row index] → The same index shows the same agent only when
  the instances show the same rows. All instances read the same server-wide
  pane list and, after this change, the same filters, so the rows are equal.

## Migration Plan

No data or config migration. A sidebar that was started before the upgrade
keeps the old behavior until it restarts.
