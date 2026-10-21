# Design

## Context

See proposal.md - Why for the motivation, and the spec deltas under
`specs/` for the required behaviour.

What shapes the approach:

- The sidebar pane is created by the binary, not by the plugin.
  `src/cli/toggle.rs` runs `split-window -hfb -l <width>` and marks the new
  pane with the pane option `@pane_role=sidebar`. The plugin configuration
  file only seeds options and hooks.
- The sidebar already talks to tmux about its own pane. `src/app/size_guard.rs`
  calls `resize-pane` on the pane id the app got from `TMUX_PANE`, and
  `src/tmux/commands.rs` already has `run_tmux_capture`, which returns
  tmux's stderr on failure.
- The top row is drawn by `render_filter_bar` in
  `src/ui/panes/filter_bar.rs`, which today takes no width and returns a bare
  `Line`. The secondary header on row 1 is the existing model for a row that
  right-aligns something: it takes the width, computes the gap, and returns
  the column of its button so the click handler can hit-test it.
- Click targets live in `FrameLayout` (`src/state/layout.rs`) and are written
  during the render, then read by the click handler before the next render.
- The status categories live in `StatusFilter` (`src/state/filter.rs`) and
  are persisted in the global tmux option `@sidebar_filter`.
- The pane status itself (`PaneStatus` in `src/tmux/types.rs`) has five
  values plus unknown, and the hook handlers in
  `src/cli/hook/handlers/status_priority.rs` decide which one a pane gets.
  `running` and `background` already share a colour in
  `src/ui/colors.rs` and already share `is_active()`.
- tmux 3.8 is available here. Measured on this machine with tmux 3.8:

```text
  break-pane -W -s <pane> -x 40 -y 18 -X 75 -Y 3
    -> the pane becomes floating, 38x16 of content, at (76, 4)
    -> the other panes in the window take the full width
    -> the pane's process keeps running in the same pane id

  join-pane -s <pane>
    -> the pane returns to the layout at its old position
    -> but at a width the layout algorithm chose (25 columns came back as 60)

  break-pane -W on a zoomed window
    -> "can't float a pane while window is zoomed"

  a floating pane and a window resize
    -> the floating pane keeps its size (18x6 stayed 18x6 while the tiled
       pane grew from 80x23 to 120x39)
```

  `break-pane -W` and the floating-pane behaviour of `join-pane` are tmux
  3.8 features. Neither exists in 3.7, where floating panes could only be
  moved and resized with the mouse.

## Goals / Non-Goals

**Goals:**

- One control, in the top row of the sidebar, that switches the sidebar
  between docked and floating and back.
- No loss of sidebar state across the switch, and no loss of the docked
  width.
- A top row that still fits at the minimum sidebar width of 20 columns.
- A status filter row with one category fewer, and with the busy count
  readable at a glance.

**Non-Goals:**

- A second sidebar instance, a separate tmux window, or a panel that can be
  dragged to another monitor.
- A tmux version fallback. The sidebar targets tmux 3.8 for the floating
  mode; there is no path for 3.7 and older.
- Changing the pane status vocabulary. `running` and `background` stay
  distinct in the pane option and in the row rendering.
- Options for the panel size and position. The values are constants in one
  place, and an option can be added later without changing the approach.

## Decisions

### The sidebar floats its own pane instead of starting a second instance

`break-pane -W` lifts the sidebar's own pane out of the tiled layout and
makes it floating, in place, in the same window. The process keeps running,
so the filter, the selection, the scroll position, the bottom tab and the
background workers all survive.

The alternative is to create a floating pane with `new-pane`, run a second
sidebar in it, and kill the docked one. That loses the sidebar state and
needs the state to be handed over, and it leaves the plugin's pane-role
marking and the `pane-exited` auto-close hook to reason about two panes.

### The mode is read from the pane, not remembered

`#{pane_floating_flag}` on the sidebar's own pane is the truth. The control
renders from it, and the click handler decides what to do from it.

The alternative is a boolean in `AppState`. It would drift from reality
whenever the pane is floated from the tmux command line, or when a pop-out
is refused. The sidebar already queries its own pane on every refresh, so
the flag comes for free with the existing query.

### The docked width is captured on the way out and restored on the way in

`join-pane` restores the position in the layout tree, not the size. The
width the sidebar had is read before the pop-out and applied with
`resize-pane -x` after the pop-in.

The alternative is to recompute the configured width from `@sidebar_width`,
the way `src/cli/toggle.rs` does when it creates the pane. That loses a
sidebar the user resized by hand, which is the case worth protecting.

### The pop-out runs the tmux command that can fail, and reports the failure

`break-pane -W` refuses to float a pane in a zoomed window. The call goes
through `run_tmux_capture`, and a failure is shown with `AppState::set_flash`
(`src/state/activity.rs`), the transient one-line banner that the spawn and
remove flows already use, so the user is told that the sidebar stayed docked.
The `ⓘ` notices popup is not used: it holds only typed hook, version and
plugin data.

The alternative is to unzoom the window first and float afterwards. That
changes the user's zoom state without asking, so the failure is reported
instead.

### The floating panel is re-sized when the window changes

A floating pane keeps its size when the window resizes, so the panel would
stay the height it had. Its position keeps no right-edge anchor either:
when the window grows, tmux leaves the floating pane's left edge where it
was, so a panel that was flush with the right edge drifts away from the
new right edge; when the window shrinks, tmux pulls the pane back into
the window and clamps it, so the panel stays inside on its own. The
geometry guard that already runs on every refresh tick, next to
`size_guard::enforce` in `src/app.rs`, re-asserts the panel height when
the window height changed, and re-asserts the panel width and its
right-edge anchor when the window width changed.

Re-asserting on every tick would fight a user who drags the panel edge, so
the guard only acts when the window size itself changed.

### The control lives on the top row and the row becomes width-aware

The top row currently draws the status chips with no knowledge of the width.
`render_filter_bar` takes the width, like `render_secondary_header` already
does, builds the chip run, computes the gap, and returns the column of the
control so the click handler can hit-test it.

The chips are separated by a single space instead of two. The chip run is 23
columns with two-space separators and 18 with single-space separators, and
the control needs two more columns, so the row fits the 20 column minimum
only with the tighter separator.

### The chips keep a single active category, and the count marks it

Selection was signalled by the icon colour alone. The busy chip now always
wears the running colour, so the selection cue moves to the count, which is
drawn in the accent colour for the active chip. The accent colour is the
theme's colour for "the active thing" already, and the `ⓘ` notices button
is the existing precedent for a status-coloured control that never dims.

The alternative is to keep the selection cue on the icon and let the busy
chip be grey unless it is active, which is what the code does today and
what the user asked to change.

### The busy merge happens in the filter, not in the pane status

`StatusFilter` loses the `background` category and gains `busy`, which
matches both the running and the background pane statuses. The pane status
itself keeps its five values, so the row still tells a live turn from a
finished turn with a live background command, and the hook handlers and
`src/state/refresh.rs` are untouched.

The alternative is one pane status for both, which would make the spinner
pulse for a turn that has already ended, and would leave every live tmux
session holding an old `@pane_status` value that nothing maps.

### The persisted filter value maps forward

`@sidebar_filter` is global tmux state and may hold `running` or
`background` from an older sidebar. Both map onto `busy`, so a session that
already had a filter keeps a filter instead of falling back to all.

## Risks / Trade-offs

- The floating mode needs tmux 3.8, and a sidebar on tmux 3.7 or older has
  no way to float. -> The control is the only entry point to the mode, so
  the older tmux simply never floats the sidebar. The docked behaviour is
  unchanged on every tmux version.
- The floating panel covers the right edge of the workspace, and a click on
  the covered area goes to the panel. -> The panel is narrow and anchored to
  the right edge, and the user pops it back in with one key press.
- The control glyph `⤢` and `⤡` may be missing from a terminal font.
  -> The control uses those two glyphs, and the app cannot detect which
  glyphs a terminal font has, so there is no ASCII fallback. A font
  without them shows a placeholder for the control, and the `p` key
  still pops the sidebar out and in.
- The panel height and width are re-asserted when the window changes, so a
  panel the user deliberately resized by hand is snapped back on the next
  window resize. -> The guard only acts on a window size change, and the
  panel keeps its own size otherwise.
- Merging the two busy categories removes the way to list only the panes
  with a live background command. -> The row still shows the command on a
  background pane, so they can be spotted by reading.
