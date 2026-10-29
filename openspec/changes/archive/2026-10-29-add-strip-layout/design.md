# Design

## Context

See `proposal.md` for the motivation and `specs/` for the behavior.

The current code that this change touches:

- `cli/toggle.rs` makes the sidebar with `split-window -hf[b] -l <width>`
  against the outermost pane (`target_pane_for_position`, which reads
  `#{pane_left} #{pane_width} #{pane_id}`). It then writes
  `@pane_role sidebar` on the new pane. `SidebarPosition` is `Left |
  Right`, with `from_setting`, `from_pane_left`, `as_setting` and
  `opposite`.
- `ui::draw` (`src/ui/mod.rs`) splits the frame vertically into the agent
  part and `split_rows(bottom_percent, height)` rows for the bottom
  panel. It writes the drawn height to `FrameLayout::bottom_panel_height`,
  and the mouse handlers hit-test that value.
- `filter_bar_layout(counts, width)` (`state/layout.rs`) is the one pure
  function that both the top row's drawing and its click test use.
  `top_line_border` (`ui/bottom.rs`) draws the centered tab run and writes
  `FrameLayout::bottom_tab_targets`.
- `app/input.rs` sends `Down`, `Drag` and `Up` of the left button to
  `handle_terminal_click`, `handle_split_drag_move` and `end_split_drag`,
  all in rows. `handle_mouse_scroll` routes a wheel event by row.
- The focus walk is vertical: `pane_nav_down` and `pane_nav_up` move
  between `Filter`, `Panes` and `ActivityLog`; `handle_filter_keys` steps
  the chips with Left and Right.
- `state/popout.rs` floats the pane with `panel_geometry` (40 columns,
  full height, right edge), captures `docked_width`, and docks with a bare
  `join-pane -s <pane>` plus `resize-pane -x`. `app/panel_guard.rs` sets
  the height, the width and the right anchor again after a window resize.
  `app/size_guard.rs` holds the pane width between `@sidebar_min_width`
  (default 35) and `@sidebar_max_width`.
- `state/side_switch.rs` and `tmux::switch_sidebar_sides` move every
  sidebar with `join-pane -h -f -d [-b]`, from one `list-panes -a`
  listing.
- The shared options are read in one `display-message` per tick
  (`SHARED_OPTION_FIELDS` in `tmux/panes.rs`) and applied by
  `GlobalState::apply_all`, with a save latch for each written value.

## Goals / Non-Goals

**Goals:**

- One value, read once, says whether a sidebar is a strip. Every code
  path that differs (layout, hit tests, keys, pop-out, side switch,
  guards) branches on that value and not on the pane shape.
- Each new geometry has one pure function that the drawing, the hit test
  and the drag all read, as `split_rows` and `filter_bar_layout` do today.
- The column paths and their snapshots do not change.

**Non-Goals:**

- No change between a strip and a column at run time.
- No height guard for a strip.
- No change to the renderers of the agent rows or of the tab contents.
  They draw into the `Rect` that they get.

## Decisions

### 1. `toggle` gives the layout to the process and to the pane

`split-window` starts the TUI before it returns, and `toggle` writes the
pane options only after that. So the TUI cannot read its layout from a
pane option at startup: it could read the option before `toggle` writes
it. `toggle` therefore gives the layout in two places:

- To the process: `split-window -e PMUX_SIDEBAR_LAYOUT=strip|column`.
  The TUI reads this variable once at startup into
  `AppState::layout_kind: LayoutKind { Column, Strip }`. A missing or
  unknown value means `Column`, so a sidebar that started before this
  change keeps its behavior.
- To the pane: `@pane_sidebar_layout` (`column` or `strip`), next to
  `@pane_role`. Only the side-switch sweep reads it, per pane, in its
  `list-panes -a` format. The sweep runs long after the pane is made, so
  the write is always done by then.

Alternative: read `@pane_sidebar_layout` at startup. Rejected: the TUI
can start before `toggle` writes it, and a lost race makes a strip a
column for the life of the pane. Alternative: an argv flag on the TUI
binary. Possible, but the TUI mode has no flags today, and `-e` keeps the
CLI parse unchanged. Alternative: read the global `@sidebar_position` at
startup. Rejected: the sweep must know the axis of the sidebar in every
window, and a global value cannot say that for a mixed server.
Alternative: derive the layout from the pane shape. Rejected by the user
(the layout is position-driven, `strip-layout` spec).

### 2. `SidebarPosition` gets two values and an axis

`SidebarPosition` becomes `Left | Right | Top | Bottom`, with
`axis() -> LayoutKind`. `from_setting` accepts `top` and `bottom`; any
other value still gives `Left`. `opposite()` stays on the axis (`Top` and
`Bottom` swap). `from_pane_left` keeps its job for a column; a new
`from_pane_at_top` reads `#{pane_at_top}` for a strip.

`split_window_flags` returns `-vfb` for `Top` and `-vf` for `Bottom`.
`target_pane_for_position` gets a strip twin that reads `#{pane_top}
#{pane_height} #{pane_id}`: the smallest top for `Top`, the largest bottom
edge for `Bottom`.

The size argument comes from `@sidebar_height` for a strip, with the same
parse as `@sidebar_width` (a row count, or a percent of
`#{window_height}`). `DEFAULT_HEIGHT_PERCENT` is `30`, and
`pmux-explorer.conf` seeds `@sidebar_height 30%`, with a test that keeps
the seed and the constant equal, like the width.

### 3. One function for the strip geometry

A new pure function in `state/layout.rs`:

```
strip_layout(width, height, tab_percent) -> StripLayout {
    filter:  Rect,          // x 0..4, full height
    agents:  Rect,          // x 4.., full height
    divider: Option<Rect>,  // 2 columns, full height
    tabs:    Option<Rect>,  // right of the divider
}
```

The tab width is `round(width * tab_percent / 100)`, divider included,
clamped so that `agents.width >= 20` and `divider + tabs >= 20`. When
`width < 4 + 20 + 20` or `tab_percent == 0`, `divider` and `tabs` are
`None` and the agent panel takes the rest. A `cols_for_percent` inverse,
like `percent_for_rows`, gives the percent for a drag release.

`ui::draw` branches on `layout_kind`. The `Column` branch is the current
code. The `Strip` branch calls `strip_layout`, paints the two fills (the
filter and the agents get one fill; the divider and the tabs get the
other), calls a new `draw_strip_filter_bar`, then `panes::draw_agents`
with the agent `Rect`, a new `draw_strip_divider`, and
`bottom::draw_bottom_content` with the tab `Rect`. The pet is not drawn.
The drawn `StripLayout` goes into `FrameLayout::strip` for the hit tests.

`panes::draw_agents` today draws the filter row and the header itself.
The strip branch needs it without the filter row. A `with_filter_row:
bool` parameter (or a split into the header-and-list part) does that; the
column call passes `true` and its snapshots stay the same.

`draw_bottom` today draws the rule and the content. It splits into the
rule (column only) and `draw_bottom_content(frame, state, area)`, which
both layouts call.

### 4. The vertical filter bar and the divider have their own layout functions

`filter_column_layout(counts, height) -> (Vec<FilterChip>, Option<u16>)`
returns the chips that fit above the control row, as rows, and the
control row (`height - 1`). The side switch is at column 0 and the
pop-out at column 2 of the control row. A chip is drawn as icon, space,
count, in 4 columns; a count of 100 or more is drawn as `99`, so that the
chip always fits (the count is a hint here, the agent list shows the
truth). `FilterChip` gets the row in its `start` field; the type stays
the same, and the meaning of `start` is documented per layout.

`divider_tab_rows(tabs, height) -> Vec<BottomTabTarget>` gives the row of
each shown tab glyph, centered (`floor((height - run) / 2)`), cut at the
bottom. `BottomTabTarget` holds a start and an end; for the divider they
are rows. The drawing writes the result to
`FrameLayout::bottom_tab_targets`, as `top_line_border` does now.

### 5. The divider drag reuses `SplitDrag` with an axis

`SplitDrag` gets `axis: LayoutKind` and its `rows` field becomes `size`
(rows for a column, tab columns for a strip). `handle_terminal_click`
branches on `layout_kind`: in a strip, a press inside `FrameLayout::strip
.divider` starts the drag; a press in the filter `Rect` goes to a new
`handle_filter_column_click(row, col)`; a press in the agent `Rect` goes
to a strip twin of `handle_mouse_click`.

The column `handle_mouse_click` cannot be reused with an offset. It
treats row 0 as the filter row, row 1 as the repo header and row 2 as the
first list row, while the spawn and remove targets are stored in absolute
frame coordinates and the repo button column is relative to the header
area. In a strip the agent panel has no filter row. The strip twin routes
by the drawn agent `Rect` (`agents`):

- Popups (notices, spawn input, remove confirm, repo popup) and the `+`
  spawn and `×` remove targets are tested first, in absolute coordinates,
  as today.
- `row == agents.y` is the repo header:
  `handle_secondary_header_click(col - agents.x)`.
- A row below it is a list row: `line_to_row` at
  `(row - agents.y - 1) + scrolls.panes.offset`.
`handle_split_drag_move` takes the mouse column for a strip, and
`end_split_drag` saves `@sidebar_tab_width` for a strip and
`@sidebar_bottom_height` for a column. An unmoved release selects the tab
whose row the press row names.

`handle_mouse_scroll` routes by the strip `Rect`s for a strip (agents or
tabs by column), and by row for a column as today. `input.rs` passes the
mouse column, which it has already.

Alternative: a second drag type. Rejected: the stale-release handling and
the "moved" rule are the same, and one type keeps one copy of them.

### 6. `@sidebar_tab_width` copies the bottom-percent sync

`GlobalState` gets `tab_percent: u8`, `last_saved_tab_percent` and
`tab_percent_save_pending`, the same as `bottom_percent`.
`SIDEBAR_TAB_WIDTH` goes into `SHARED_OPTION_FIELDS` before
`SIDEBAR_REPO_FILTER`, which must stay last. `apply_all` does not adopt it
while a drag is active. The parse is the bottom-percent parse with the
default `DEFAULT_TAB_PERCENT = 40`. `pmux-explorer.conf` seeds
`@sidebar_tab_width 40`, with a seed test.

### 7. A second key map for a strip

`handle_key_event` branches on `layout_kind` before the navigation keys.
For a strip: Left/`h` and Right/`l` step `Focus` through `Filter`,
`Panes`, `ActivityLog` and stop at the ends; Up/Down call the existing
per-part actions (`StatusFilter::prev/next` with the save, the pane
selection without the jump to another part, `scroll_bottom`). Tab,
BackTab, Esc and the other keys stay the same in both layouts. The column
path calls the current `handle_filter_keys`, `pane_nav_down` and
`pane_nav_up` without change.

### 8. Pop-out and pop-in of a strip

`SidebarPaneInfo` gets `height` (`#{pane_height}`) and `at_top`
(`#{pane_at_top}`), added to the pane fields before the shared options.
`panel_geometry` takes the layout kind and the docked edge:

```
Column              -> 40 x window_height at (window_width - 40, 0)  (today)
Strip, top edge     -> window_width x h at (0, 0)
Strip, bottom edge  -> window_width x h at (0, window_height - h)
```

`h` is the docked height that the pop-out captures, the same way
`docked_width` is captured (the field becomes `docked_size`). The docked
edge is captured too, because a floating pane is no longer at an edge.

Pop-in of a strip does not use the bare `join-pane -s`: that command has no
target edge, so the docked pane is not sure to be full width. It uses
the edge join of the side switch, `join-pane -v -f -d [-b] -s <pane> -t
<edge pane>`, then `resize-pane -y <h>`. The column pop-in stays as it is.

`panel_guard` gets the layout kind and the captured edge. For a strip it
sets the width to the window width and moves the pane to its anchor when
the window size changes; the height is left as the user set it.
`clamp_bottom_height` is not applied in a strip.

### 9. The side switch keeps today's rule on the acting axis

Today the sweep moves every sidebar to one target side, so a window that
an earlier switch skipped comes back into line with the others. That
rule stays for the axis of the sidebar that runs the action:

- The target edge is the opposite of the acting sidebar's edge.
- Each docked sidebar on the acting axis moves to that target edge. A
  sidebar that is already there is left as it is, as today.
- Each docked sidebar on the other axis flips to its own opposite edge.
  That axis has no common target, because the action names only one
  edge.

A column moves with `join-pane -h -f -d [-b]` (today), a strip with
`join-pane -v -f -d [-b]` against the strip edge target. The sweep's
`list-panes -a` format gets `#{@pane_sidebar_layout}` and
`#{pane_at_top}`. The new global `@sidebar_position` is the target edge.
On a server with columns only, the result is the same as today. The
control glyph is `⇄` for a column and `⇅` for a strip.

Alternative: each sidebar flips to its own opposite edge on both axes.
Rejected: on a column-only server, a window that an earlier switch skipped
would move away from the others, which changes the column behavior.
Alternative: leave the other axis alone. Rejected: the user accepted that
a switch moves the sidebar of every window.

### 10. The width guard does not act on a strip

`size_guard::enforce` returns at once when `layout_kind` is `Strip`. A
strip is as wide as the window, so a `@sidebar_max_width` would shrink it
into a narrow strip in the corner. There is no height guard (out of
scope).

## Risks / Trade-offs

- [The bare `join-pane -s` that the column pop-in uses lands at an unknown
  place] → A strip uses the edge join. A recorded run in a scratch tmux
  3.8 server (task 8.6) checks that a popped-in strip is full width at its
  edge.
- [`#{pane_at_top}` is true for a single-pane window where the strip is
  the only pane] → The sweep skips a window with no workspace pane, as
  the edge target is then `None`; a pop-in with no edge target falls back
  to the bare join.
- [The 4-column filter bar cuts a count of 100 or more to `99`] → Accepted.
  The agent list shows the real list.
- [A strip of fewer than 2 rows cannot draw a chip and the control row] →
  `filter_column_layout` returns no chips and no control for a height
  below 1, and only the control row for a height of 1.
- [The `draw_agents` split could change the column snapshots] → The column
  call keeps the same arguments; the column snapshot suite must pass with
  no accepted change.
- [A sidebar that was made before this change] → Its process has no
  `PMUX_SIDEBAR_LAYOUT` and its pane has no `@pane_sidebar_layout`, so
  the TUI and the sweep both treat it as a column, which is correct.
- [Two layouts double the input paths to test] → The strip key map is a
  separate function with its own unit tests; the column tests stay as
  they are.

## Migration Plan

No data migration. The new options have defaults, the old values keep
their meaning, and a running sidebar keeps its layout. Rollback is a
revert; a strip that exists at that time becomes a full-width column
layout until the user toggles it.
