# Proposal

## Why

The sidebar can dock only on the left or the right edge of a window, as a
tall column. On a wide screen, a column takes width that the workspace
panes need. A user who wants the sidebar as a short strip along the top or
the bottom edge has no way to get it. A column layout in a short strip
would also not work: the agent list would get only 2 or 3 rows above the
tab panel.

## What Changes

- `@sidebar_position` accepts two new values, `top` and `bottom`. A
  sidebar made with one of these values is a full-width **strip** at that
  edge. `left` and `right` make a **column**, as today. The position
  selects the layout when the sidebar is made; the shape of the pane does
  not.
- A new option `@sidebar_height` gives the height of a strip, as a row
  count or a percent of the window height (default `30%`).
  `@sidebar_width` controls only a column.
- A strip has two panels side by side: the agent list on the left and the
  tab panel (Tools, Thought, Tasks, Git) on the right.
- In a strip, the tab bar is the divider between the two panels. It is a
  vertical column, 2 columns wide, with the tab glyphs stacked and
  centered on it.
- In a strip, the status filter bar is a vertical column, 4 columns wide,
  at the left edge. It has one chip for each row and the side-switch and
  pop-out controls on its last row. The repo header stays the first row
  of the agent list.
- The user resizes the two panels when they drag the divider in x. The
  drag saves a new shared option, `@sidebar_tab_width`: the percent of the
  strip width that the tab panel takes (default `40`, `0` hides the tab
  panel). Each panel keeps at least 20 columns.
- In a strip, the side switch (control and prefix + `Ctrl+O`) moves the
  sidebar between top and bottom. Its glyph is `⇅`. It never changes a
  strip into a column, or a column into a strip.
- In a strip, pop-out floats a full-width strip at the docked edge, with
  the docked height. Pop-in restores that height. The one-third clamp of
  the floating column does not apply.
- In a strip, Left and Right move the keyboard focus between the filter
  bar, the agent list and the tab panel. Up and Down act inside the
  focused part: they step the chips, select an agent, or scroll a tab.
- The pet does not show in a strip.

Out of scope:

- A live change between a strip and a column. The user changes
  `@sidebar_position` and toggles the sidebar off and on.
- An agent list that scrolls horizontally, or a grid of agent cards.
- A height guard for the strip like the width guard of the column.

## Capabilities

### New Capabilities

- `strip-layout`: the top and bottom positions, the strip height, the two
  panels, the vertical filter bar, the vertical divider with the tabs and
  its drag, the minimum panel widths, and the keyboard focus in a strip.

### Modified Capabilities

- `sidebar-panel`: docking now also means a strip at the top or the bottom
  edge; the side switch stays on the axis of the sidebar; pop-out and
  pop-in of a strip use the full width and the docked height; the
  one-third clamp applies only to a floating column.
- `shared-view-settings`: the strip split width (`@sidebar_tab_width`)
  becomes a sixth shared view setting, and a divider drag in progress is
  not reverted by a refresh tick.
- `bottom-panel`: the panel under the agent list is now the layout of a
  column only; a strip uses the `strip-layout` divider.
- `focus-indication`: the two filled sections are named the agent section
  and the tab section, so that they also describe the left and right
  panels of a strip.

## Impact

- `src/cli/toggle.rs`: `SidebarPosition` gets `Top` and `Bottom`, the
  `split-window -v` flags, the `@sidebar_height` size, the edge target by
  `pane_top`, and the layout given to the new process (`split-window -e`)
  and to the pane (a per-pane option for the side-switch sweep).
- `src/tmux/options.rs`, `src/tmux/panes.rs`, `src/state/global.rs`: the
  new options and the shared `@sidebar_tab_width` field.
- `src/ui/mod.rs`, `src/ui/bottom.rs`, `src/ui/panes.rs`,
  `src/ui/panes/filter_bar.rs`: the horizontal split, the vertical divider
  and the vertical filter bar.
- `src/state/layout.rs`, `src/app/input.rs`, `src/state/tab.rs`: the
  layout functions, the hit tests, the divider drag and the strip key map.
- `src/state/side_switch.rs`, `src/tmux/commands.rs`: the vertical
  `join-pane` sweep.
- `src/state/popout.rs`, `src/app/panel_guard.rs`, `src/app/size_guard.rs`:
  the floating strip geometry and its guard; the width guard does not act
  on a strip.
- `pmux-explorer.conf`, `website/src/content/docs/reference/tmux-options.md`,
  `website/src/content/docs/reference/keybindings.md`,
  `docs/state-management.md`: the new values, options and keys.
- Tests: new inline snapshots for the strip; the column snapshots do not
  change.
