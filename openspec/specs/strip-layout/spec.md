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

A strip that does not show the split layout (`split-strip`) SHALL show,
from left to right: the vertical filter bar, the agent panel, the tab
divider, and the tab panel. The requirements of this capability for the
tab panel, its tab bar, its divider and `@sidebar_tab_width` SHALL apply
to such a one-pane strip only; a split strip follows `split-strip`. Each
part except the tab
bar SHALL use the full height of the strip. The agent panel SHALL show the
repo header on its first row and the agent rows below it, and SHALL
scroll vertically as the agent list does in a column. In a strip's
agent list the branch name SHALL ride the pane's status row after the
pane name instead of a dedicated branch row; that row SHALL still
render when it keeps content of its own — listening ports or the
remove `×` of a spawn-created worktree. The pet SHALL NOT show in a
strip.

#### Scenario: the parts of a strip
- **WHEN** a strip 120 columns wide and 12 rows high shows the Tools tab
- **THEN** columns 0-3 are the filter bar, the agent panel follows, then the 1-column divider line, then the tab panel to the right edge, and the tab section's top row is the tab bar

#### Scenario: more agents than rows
- **WHEN** a strip shows more agent rows than its agent panel can hold
- **THEN** the agent panel scrolls to keep the selected agent visible

#### Scenario: the pet is on
- **WHEN** `@sidebar_pet` is `on` and the sidebar is a strip
- **THEN** the strip shows no pet

#### Scenario: a split strip does not read the one-pane tab width
- **WHEN** `@sidebar_tab_width` is 50 and the strip shows the split layout
- **THEN** the panel widths come from the `split-strip` options, and a divider drag does not write `@sidebar_tab_width`

### Requirement: the strip filter bar is a vertical column

In a strip, the status filter SHALL be a column 4 columns wide at the left
edge. It SHALL show one chip on each row, in the order all, busy, waiting,
idle, error, from the first row. Its last row SHALL hold the side-switch
control, the pop-out control and the lock control. A chip that does not
fit above the control row SHALL be dropped whole, together with each chip
after it. A click on a chip SHALL select its category. A click on the lock
control SHALL toggle the lock the same way as pressing `w`.

#### Scenario: all chips fit
- **WHEN** a strip is 12 rows high
- **THEN** rows 0-4 show the all, busy, waiting, idle and error chips, and row 11 shows the side-switch, pop-out and lock controls

#### Scenario: a short strip
- **WHEN** a strip is 4 rows high
- **THEN** rows 0-2 show the all, busy and waiting chips, row 3 shows the controls, and the idle and error chips are not drawn

#### Scenario: a click on a chip
- **WHEN** the user clicks the waiting chip in the strip filter bar
- **THEN** the waiting category is selected and the agent panel shows only waiting agents

#### Scenario: the lock control shows the lock state
- **WHEN** a strip is rendered
- **THEN** the control row shows the lock control after the pop-out control, with a different glyph for locked and unlocked

#### Scenario: a click on the lock control
- **WHEN** the user clicks the lock control in the strip filter bar
- **THEN** the lock toggles on if it was off and off if it was on, for every sidebar instance on the server

### Requirement: the strip's tab bar is a header row

In a strip, the tab bar SHALL be the top row of the tab section — the
divider column included — a horizontal band in the filter bar's
background colour. It SHALL draw the shown tab glyphs in a
right-aligned horizontal run, in the same order and with the same
styles as the column's rule — the displayed tab in the accent colour —
and a run wider than the band SHALL be truncated. The run SHALL keep
one column free at the band's right edge, so the glyphs' two-cell
rendering never has its last glyph clipped by the sidebar's edge. The
divider between the agent panel
and the tab panel SHALL be a single column drawing the left
one-quarter block glyph on every row over the sidebar's own fill: a
thin vertical line that carries no band and holds no tabs.

#### Scenario: the run is right-aligned on the tab bar
- **WHEN** the strip shows `R` tab glyphs
- **THEN** the tab bar draws them right-aligned horizontally on the tab section's top row, with one free column between the last glyph and the band's right edge

#### Scenario: the tabs for a Claude pane
- **WHEN** a strip shows the tab section and the focused pane runs Claude
- **THEN** the tab bar draws the Tools, Tasks and Git glyphs in that order

#### Scenario: the tabs for a pi pane
- **WHEN** a strip shows the tab section and the focused pane runs pi
- **THEN** the tab bar draws the Tools, Thought, Tasks and Git glyphs in that order

### Requirement: the strip's tab bar names the focused pane

The pad the right-aligned tab run leaves on the tab bar's left SHALL
carry the focused pane's identity: its pane id, then its own name when
the pane has one, one column in from the band's left edge and one
column short of the run. The id SHALL come first and SHALL never be
cut: a pad that cannot hold it whole draws no label at all, and a pad
that holds it but not the whole name SHALL truncate the name with an
ellipsis. A sidebar with no focused pane — or one whose focused pane
no longer resolves — SHALL draw no label. The label SHALL wear the
band's bright text colour and SHALL NOT be a tab click target.

#### Scenario: a wide tab bar names the pane
- **WHEN** the strip's tab bar leaves the run more pad than the focused pane's id and name need
- **THEN** the bar draws the id and the whole name at its left end and the rest of the pad stays empty

#### Scenario: a narrow tab bar cuts the name
- **WHEN** the pad holds the focused pane's id and part of its name
- **THEN** the bar draws the id whole and the name with an ellipsis, and the tab run keeps its columns

#### Scenario: a tab bar too narrow for the id
- **WHEN** the pad cannot hold the focused pane's id
- **THEN** the bar draws no label at all

#### Scenario: an unnamed pane
- **WHEN** the focused pane carries no name of its own
- **THEN** the label is the pane id alone

### Requirement: a click on the tab bar selects a tab

When the user presses and releases the left mouse button on the tab
bar without a move to a different column, a press on a glyph's drawn
columns SHALL select that tab on release. A press on a tab-bar column
with no glyph, or on the thin divider line, SHALL change nothing. The
hitboxes SHALL be the columns the tab bar draws.

#### Scenario: a click on a glyph
- **WHEN** the Tools tab is selected and the user presses and releases on the Git glyph's columns of the tab bar
- **THEN** the Git tab is selected and the panel widths do not change

#### Scenario: a click on the band
- **WHEN** the user presses and releases on a tab-bar column that has no glyph
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
tab bar and moves the mouse to a different column before release, the
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
agent panel, tab panel — or, in a split strip, filter bar, agent panel,
pane A, pane B; the focus SHALL stay at an end. Up and Down SHALL
act inside the focused part: in the filter bar they step the category, in
the agent panel they move the selection and stop at the first and last
agent, and in a tab panel they scroll. A column SHALL keep its key map.

#### Scenario: Right from the agent panel
- **WHEN** the sidebar is a strip and the agent panel has focus
- **AND** the user presses Right
- **THEN** the tab panel has focus, or pane A in a split strip

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

### Requirement: the strip height is shared while the lock is on

While the sidebar lock is on, every docked strip SHALL converge on one
shared docked height, held in `@sidebar_docked_height` as absolute rows.
The lock is the same server-wide lock the width lock uses: toggling it
from any sidebar instance SHALL govern both axes, a column syncing its
width and a strip its height.

#### Scenario: the lock is one flag for both axes
- **WHEN** the user presses `w`, or clicks the strip's lock control, while a strip has focus
- **THEN** the same server-wide lock toggles, and strips in every window begin converging on one shared height

### Requirement: enabling the lock from a strip seeds the shared height

Enabling the lock from a docked strip SHALL seed the shared docked height
with the strip's live height, at least the minimum strip height floor. A
floating strip SHALL seed the docked height it captured on the way out, at
least the floor, or SHALL enable the lock without a seed when no docked
height was captured.

#### Scenario: enabling from a strip seeds the shared height
- **WHEN** the user enables the lock while a docked strip is 12 rows high
- **THEN** `@sidebar_docked_height` becomes 12 rows

#### Scenario: enabling from a squeezed strip seeds the floor
- **WHEN** the user enables the lock while a docked strip is 2 rows high
- **THEN** `@sidebar_docked_height` becomes 4 rows, the minimum strip height floor

#### Scenario: enabling from a floating strip seeds the captured height
- **WHEN** the user pops out a docked strip of 10 rows and then enables the lock
- **THEN** `@sidebar_docked_height` becomes 10 rows, not the floating panel's height

#### Scenario: enabling from a floating strip with no capture seeds nothing
- **WHEN** the strip was floated before the sidebar started, so no docked height was captured, and the user enables the lock
- **THEN** the lock turns on and `@sidebar_docked_height` is not written

### Requirement: a docked strip publishes its height while locked

While the lock is on, a docked strip whose pane height changed SHALL
publish the new height as the shared docked height, unless the change was
the strip's own adoption of the shared value. A pane height below the
minimum strip height floor SHALL NOT be published.

#### Scenario: a manual resize becomes the shared height
- **WHEN** the lock is on and the user resizes the docked strip of window A to 14 rows
- **THEN** the shared docked height becomes 14 rows

#### Scenario: a squeeze below the floor is not published
- **WHEN** the lock is on and a horizontal split squeezes the strip below 4 rows
- **THEN** the shared docked height is unchanged

### Requirement: a docked strip adopts the shared height while locked

While the lock is on, a docked strip whose height differs from the shared
docked height SHALL resize toward it, taller or shorter. The adopted
height SHALL be capped at half the window's rows and SHALL never go below
4 rows; where the two cannot both hold, the floor wins. The cap binds
adopted heights only: a publishing strip MAY stay taller than half its
window. Strips in hidden windows SHALL converge within one refresh tick.

#### Scenario: a taller shared height grows the strip
- **WHEN** the lock is on and the shared docked height becomes 14 rows
- **THEN** a docked strip of 10 rows resizes to 14 rows within one refresh tick

#### Scenario: a shorter shared height shrinks the strip
- **WHEN** the lock is on and the shared docked height becomes 8 rows
- **THEN** a docked strip of 14 rows resizes to 8 rows within one refresh tick

#### Scenario: a short window caps the adopted height
- **WHEN** the lock is on, the shared height is 16 rows, and the strip's window is 30 rows high
- **THEN** the strip adopts 15 rows and the shared docked height stays 16 rows

#### Scenario: a shared height below the floor adopts at the floor
- **WHEN** the lock is on, the shared docked height is 2 rows, and the strip's window is 40 rows high
- **THEN** the strip adopts 4 rows

#### Scenario: the publisher's own resize is not capped
- **WHEN** the lock is on and the user resizes the docked strip of a 40-row window to 14 rows
- **THEN** the shared docked height becomes 14 rows, that strip stays at 14 rows, and a peer on a 20-row window adopts 10 rows

#### Scenario: a strip in a hidden window converges
- **WHEN** the lock is on, the shared docked height becomes 14 rows, and the strip's window is not active
- **THEN** the strip adopts 14 rows within one refresh tick without the window becoming active

### Requirement: a floating strip is outside the lock

While the lock is on, a floating strip SHALL keep its panel size, SHALL
NOT publish its height, and SHALL NOT adopt the shared height. Popping it
back in SHALL dock it at the shared docked height instead of the height
it had before it was popped out.

#### Scenario: the floating strip keeps its size
- **WHEN** the lock is on, a strip floats as a panel, and the shared docked height changes
- **THEN** the floating strip keeps its size

#### Scenario: popping back in takes the shared height
- **WHEN** the lock is on with a shared docked height of 14 rows and the user pops a floating strip back in
- **THEN** the strip docks at 14 rows

### Requirement: a new strip starts at the shared height while locked

While the lock is on and the server holds a shared docked height, adding
the sidebar to a window as a strip SHALL create the pane at the shared
docked height instead of resolving `@sidebar_height`. While the lock is
off, `@sidebar_height` SHALL keep governing a new strip as before, a
percent resolving against that window's height.

#### Scenario: a strip opened under the lock
- **WHEN** the lock is on with a shared docked height of 14 rows and the user opens the sidebar in a new window
- **THEN** the new strip pane is 14 rows high

#### Scenario: a strip opened unlocked keeps the option rule
- **WHEN** the lock is off, `@sidebar_height` is `30%`, the window is 40 rows high, and the user opens the sidebar in that window
- **THEN** the new strip pane is 12 rows high

### Requirement: enabling the lock from a column clears the shared height

Enabling the lock from a column SHALL clear `@sidebar_docked_height`.
While no shared docked height exists, a docked strip SHALL keep its height
and SHALL NOT resize; the first strip whose height then changes publishes
a new shared height.

#### Scenario: a stale shared height is not adopted
- **WHEN** `@sidebar_docked_height` holds 14 rows from an earlier lock session, a docked strip is 10 rows high, and the user enables the lock from a column
- **THEN** the shared docked height is cleared and the strip stays 10 rows high

#### Scenario: a resize after the clear publishes
- **WHEN** the lock was enabled from a column and the user then resizes the docked strip of window A to 12 rows
- **THEN** the shared docked height becomes 12 rows and the other docked strips adopt it within one refresh tick
