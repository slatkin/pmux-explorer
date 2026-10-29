# strip-layout Specification

## Purpose
Lets the user dock the sidebar as a short, full-width strip at the top or
the bottom edge of a window, with the agent list and the tab panel side by
side, a vertical filter bar, and a vertical tab divider that resizes the
two panels.

## Requirements

### Requirement: top and bottom positions make a strip

`@sidebar_position` SHALL accept `top` and `bottom` in addition to `left`
and `right`. A sidebar made while the value is `top` or `bottom` SHALL be a
tiled pane that spans the full window width at that edge (a strip). A
sidebar made while the value is `left` or `right` SHALL be a column, as
before. Any other value SHALL make a left column.

#### Scenario: a bottom strip
- **WHEN** `@sidebar_position` is `bottom` and the user toggles the sidebar on
- **THEN** the sidebar is a tiled pane at the bottom edge, as wide as the window, and the workspace panes share the rows above it

#### Scenario: a top strip
- **WHEN** `@sidebar_position` is `top` and the user toggles the sidebar on
- **THEN** the sidebar is a tiled pane at the top edge, as wide as the window

#### Scenario: an unknown value
- **WHEN** `@sidebar_position` is `middle` and the user toggles the sidebar on
- **THEN** the sidebar is a column at the left edge

### Requirement: the layout is fixed when the sidebar is made

A sidebar SHALL keep the layout (strip or column) that its position gave
when it was made, for the life of its pane. A later change of
`@sidebar_position`, a resize of the pane, a side switch, or a pop-out SHALL
NOT change a strip into a column or a column into a strip.

#### Scenario: the option changes while a column runs
- **WHEN** a left column runs and the user sets `@sidebar_position` to `top`
- **THEN** that sidebar stays a column, and a sidebar made after the change is a strip

#### Scenario: a strip that the user makes taller
- **WHEN** the user drags the tmux border of a strip until the strip is taller than it is wide
- **THEN** the sidebar keeps the strip layout

### Requirement: the strip height is a separate option

`@sidebar_height` SHALL give the height of a new strip, as a row count or
as a percent of the window height (a number followed by `%`). When the
option is not set or is empty, the height SHALL be `30%`.
`@sidebar_width` SHALL NOT change the size of a strip, and
`@sidebar_height` SHALL NOT change the size of a column.

#### Scenario: the default height
- **WHEN** `@sidebar_height` is not set, `@sidebar_position` is `bottom`, and the window is 40 rows high
- **THEN** the new strip is 12 rows high

#### Scenario: a row count
- **WHEN** `@sidebar_height` is `10` and a strip is made
- **THEN** the strip is 10 rows high

#### Scenario: the width option does not size a strip
- **WHEN** `@sidebar_width` is `50%` and a strip is made
- **THEN** the strip is as wide as the window

### Requirement: a strip shows two panels side by side

A strip SHALL show, from left to right: the vertical filter bar, the agent
panel, the tab divider, and the tab panel. Each part except the footer
SHALL use the full height of the strip. The agent panel SHALL show the
repo header on its first row and the agent rows below it, and SHALL
scroll vertically as the agent list does in a column. In a strip's
agent list the branch name SHALL ride the pane's status row after the
pane name instead of a dedicated branch row; that row SHALL still
render when it keeps content of its own — listening ports or the
remove `×` of a spawn-created worktree. The pet SHALL NOT show in a
strip.

#### Scenario: the parts of a strip
- **WHEN** a strip 120 columns wide and 12 rows high shows the Tools tab
- **THEN** columns 0-3 are the filter bar, the agent panel follows, then the 1-column divider line, then the tab panel to the right edge, and the tab section's bottom row is the footer tab bar

#### Scenario: more agents than rows
- **WHEN** a strip shows more agent rows than its agent panel can hold
- **THEN** the agent panel scrolls to keep the selected agent visible

#### Scenario: the pet is on
- **WHEN** `@sidebar_pet` is `on` and the sidebar is a strip
- **THEN** the strip shows no pet

### Requirement: the strip filter bar is a vertical column

In a strip, the status filter SHALL be a column 4 columns wide at the left
edge. It SHALL show one chip on each row, in the order all, busy, waiting,
idle, error, from the first row. Its last row SHALL hold the side-switch
control and the pop-out control. A chip that does not fit above the
control row SHALL be dropped whole, together with each chip after it. A
click on a chip SHALL select its category.

#### Scenario: all chips fit
- **WHEN** a strip is 12 rows high
- **THEN** rows 0-4 show the all, busy, waiting, idle and error chips, and row 11 shows the side-switch and pop-out controls

#### Scenario: a short strip
- **WHEN** a strip is 4 rows high
- **THEN** rows 0-2 show the all, busy and waiting chips, row 3 shows the controls, and the idle and error chips are not drawn

#### Scenario: a click on a chip
- **WHEN** the user clicks the waiting chip in the strip filter bar
- **THEN** the waiting category is selected and the agent panel shows only waiting agents

### Requirement: the strip's tab bar is a footer row

In a strip, the tab bar SHALL be the bottom row of the tab section — the
divider column included — a horizontal band in the filter bar's
background colour. It SHALL draw the shown tab glyphs in a centred
horizontal run, in the same order and with the same styles as the
column's rule — the displayed tab in the accent colour — and a run wider
than the band SHALL be truncated. The divider between the agent panel
and the tab panel SHALL be a single column drawing the left
one-quarter block glyph on every row over the sidebar's own fill: a
thin vertical line that carries no band and holds no tabs.

#### Scenario: the run is centred on the footer
- **WHEN** the strip shows `R` tab glyphs
- **THEN** the footer draws them centred horizontally on the tab section's bottom row

#### Scenario: the tabs for a Claude pane
- **WHEN** a strip shows the tab section and the focused pane runs Claude
- **THEN** the footer draws the Tools, Tasks and Git glyphs in that order

#### Scenario: the tabs for a pi pane
- **WHEN** a strip shows the tab section and the focused pane runs pi
- **THEN** the footer draws the Tools, Thought, Tasks and Git glyphs in that order

### Requirement: a click on the footer selects a tab

When the user presses and releases the left mouse button on the footer
without a move to a different column, a press on a glyph's drawn columns
SHALL select that tab on release. A press on a footer column with no
glyph, or on the thin divider line, SHALL change nothing. The hitboxes
SHALL be the columns the footer draws.

#### Scenario: a click on a glyph
- **WHEN** the Tools tab is selected and the user presses and releases on the Git glyph's columns of the footer
- **THEN** the Git tab is selected and the panel widths do not change

#### Scenario: a click on the band
- **WHEN** the user presses and releases on a footer column that has no glyph
- **THEN** the tab and the panel widths do not change

### Requirement: the strip tab width is a percent of the strip width

`@sidebar_tab_width` SHALL give the width of the tab panel, divider
included, as a percent of the strip width, rounded to the nearest column.
When the option is not set, empty, or not a number from 0 to 100, the
width SHALL be 40 percent. The value 0 SHALL hide the divider and the tab
panel. `@sidebar_bottom_height` SHALL NOT change a strip.

#### Scenario: the default width
- **WHEN** `@sidebar_tab_width` is not set and the strip is 120 columns wide
- **THEN** the divider and the tab panel take 48 columns

#### Scenario: zero hides the tab panel
- **WHEN** `@sidebar_tab_width` is 0
- **THEN** the strip shows no divider and no tab panel, and the agent panel takes the width that the filter bar leaves

#### Scenario: the column option does not change a strip
- **WHEN** `@sidebar_bottom_height` is 80 and the sidebar is a strip
- **THEN** the tab panel width still comes from `@sidebar_tab_width`

### Requirement: each strip panel keeps a minimum width

In a strip, the agent panel SHALL keep at least 20 columns, and the tab
panel with its divider SHALL keep at least 20 columns. A width outside
these limits SHALL be clamped to them. When the strip is too narrow for
the filter bar and both minimums, the divider and the tab panel SHALL NOT
show.

#### Scenario: a small percent
- **WHEN** `@sidebar_tab_width` is 1 and the strip is 120 columns wide
- **THEN** the divider and the tab panel take 20 columns

#### Scenario: a large percent
- **WHEN** `@sidebar_tab_width` is 100 and the strip is 120 columns wide
- **THEN** the agent panel keeps 20 columns and the divider and the tab panel take the rest

#### Scenario: a very narrow strip
- **WHEN** the strip is 40 columns wide
- **THEN** the strip shows no divider and no tab panel

### Requirement: the user resizes the strip by dragging the divider

When the user presses the left mouse button on the divider line or the
footer and moves the mouse to a different column before release, the
divider SHALL follow the mouse column on each move, within the minimum
widths. On release, the sidebar SHALL keep the new width and save it to
`@sidebar_tab_width` as a percent of the strip width, rounded to the
nearest percent. A drag that starts on a glyph SHALL resize and SHALL
NOT change the tab.

#### Scenario: a drag to the left
- **WHEN** a strip is 120 columns wide and the divider starts at column 72
- **AND** the user presses on the divider and drags it to column 60
- **THEN** the divider starts at column 60 during the drag and after release
- **AND** `@sidebar_tab_width` is 50 after release

#### Scenario: no tmux write during the drag
- **WHEN** the user drags the divider across several columns
- **THEN** `@sidebar_tab_width` changes only once, on release

### Requirement: arrow keys follow the strip

In a strip, Left and `h` SHALL move the keyboard focus one part to the
left, and Right and `l` one part to the right, in the order filter bar,
agent panel, tab panel; the focus SHALL stay at an end. Up and Down SHALL
act inside the focused part: in the filter bar they step the category, in
the agent panel they move the selection and stop at the first and last
agent, and in the tab panel they scroll. A column SHALL keep its key map.

#### Scenario: Right from the agent panel
- **WHEN** the sidebar is a strip and the agent panel has focus
- **AND** the user presses Right
- **THEN** the tab panel has focus

#### Scenario: Down on the last agent
- **WHEN** the sidebar is a strip and the last agent is selected
- **AND** the user presses Down
- **THEN** the last agent stays selected and the agent panel keeps focus

#### Scenario: Down in the filter bar
- **WHEN** the sidebar is a strip, the filter bar has focus and the busy category is selected
- **AND** the user presses Down
- **THEN** the waiting category is selected

#### Scenario: Left at the left end
- **WHEN** the sidebar is a strip and the filter bar has focus
- **AND** the user presses Left
- **THEN** the filter bar keeps focus and the category does not change
