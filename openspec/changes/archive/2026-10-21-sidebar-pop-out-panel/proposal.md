# Proposal

## Why

The sidebar is useful as a permanent column, but a docked column always costs
the workspace its width, and a new pane in the window can squeeze the sidebar
down to a few columns. A user who wants to watch an agent closely wants the
sidebar out of the way without closing it, and a user who wants the sidebar
always visible wants the opposite. Today the sidebar can only be one or the
other: the plugin toggle kills the pane and creates it again.

The status filter row on the same top row has the opposite problem: it is
always there and it is noisy. It carries six chips where the user reads five
things, because "running" and "background" are both "the agent is busy", and
the busy chip is greyed out whenever it is not the active filter, which hides
the one number worth noticing.

## What Changes

- The sidebar's top row gains a right-aligned control that pops the sidebar
  out of the window into a floating tmux panel, and pops it back in. The
  sidebar stays docked by default; the control only changes the mode.
- The floating panel is 40 columns wide, as tall as the window, and anchored
  to the right edge. The Activity/Git bottom panel is clamped to a third of
  the panel height so the agent list stays readable.
- Popping the sidebar back in restores the docked width the sidebar had
  before it was popped out, so a manually resized sidebar survives the round
  trip.
- The status filter row merges the "running" and "background" chips into one
  "busy" chip that counts both, so the row carries five chips.
- The busy chip icon is drawn in the running colour whether or not it is the
  active filter. The active chip is marked by its count, which is drawn in
  the accent colour.
- The chips are separated by a single space instead of two, so the row fits
  the control at the minimum sidebar width.
- The keyboard shortcut `p` toggles the pop-out state.

## Capabilities

### New Capabilities
- `sidebar-panel`: the sidebar pane as a user-facing surface — its docked
  default, the floating panel mode, the control that switches between them,
  the geometry of each, and the state that survives the switch.
- `status-filter`: the status filter row of the sidebar — the categories it
  offers, how a category is counted, how the active category is marked, and
  how a category is selected.

### Modified Capabilities
<!-- No existing capability changes. `pi-agent-support` covers the pi agent
     protocol and is untouched by this change. -->

## Impact

- **Code**: `src/ui/panes/filter_bar.rs` (chip row, the new control, the
  click columns), `src/state/layout.rs` (click targets, the chip hit
  test, and `filter_bar_width`), `src/state/filter.rs` (the
  `StatusFilter` categories and their matching), `src/state/popout.rs`
  (the pop-out/pop-in action, the panel geometry, the bottom-panel
  clamp), `src/app/input.rs` (the `p` shortcut), `src/app/panel_guard.rs`
  (the floating-panel height guard), `src/tmux/commands.rs` (the tmux
  calls that float and dock the pane), `src/tmux/panes.rs` (the pane
  query that reports the floating flag and the sidebar's own pane
  state), and small wiring in `src/state.rs`, `src/state/refresh.rs`,
  `src/app.rs`, `src/ui/mod.rs` and `src/ui/panes.rs`.
- **Website**: the new feature page
  `website/src/content/docs/features/floating-panel.mdx` (wired into the
  sidebar list in `website/astro.config.mjs`), the `p` shortcut in
  `website/src/content/docs/reference/keybindings.md`, and the matching
  section in `README.md`.
- **tmux**: the floating panel needs tmux 3.8. `break-pane -W` lifts a tiled
  pane out of the layout, and `join-pane` puts a floating pane back into the
  layout; both exist in 3.8 and neither exists in 3.7.
- **Plugin**: the plugin configuration file keeps its role of seeding
  options and hooks. The pane is created and moved by the binary, so no
  plugin change is needed for the mode itself.
- **Compatibility**: the persisted status filter value changes shape, so a
  session that holds the old "background" value must map onto the merged
  category instead of falling back to "all".
