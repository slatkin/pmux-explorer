# Spec Delta

## Purpose

Lets a strip on a very wide window show two tab panes side by side to the
right of the agent panel, so the user can see two tabs at the same time
instead of one very wide tab panel.

## ADDED Requirements

### Requirement: a wide strip shows the split layout

A strip SHALL show the split layout while its width in columns is equal to
or more than `@sidebar_tab_split_width`. When the option is not set, empty,
or not a whole number, the threshold SHALL be 300. The value 0 SHALL turn
the split layout off. The strip SHALL check its width on each frame, so a
resize goes into or out of the split layout without a restart. A column
SHALL NOT show the split layout.

#### Scenario: a full-screen strip on an ultra-wide window
- **WHEN** `@sidebar_tab_split_width` is not set and the strip is 381 columns wide
- **THEN** the strip shows the split layout

#### Scenario: a strip below the threshold
- **WHEN** `@sidebar_tab_split_width` is not set and the strip is 250 columns wide
- **THEN** the strip shows one tab panel, as before

#### Scenario: zero turns the split layout off
- **WHEN** `@sidebar_tab_split_width` is 0 and the strip is 381 columns wide
- **THEN** the strip shows one tab panel

#### Scenario: a resize goes out of the split layout
- **WHEN** a strip shows the split layout and the window becomes 190 columns wide
- **THEN** the strip shows one tab panel on the next frame

#### Scenario: a column on a wide window
- **WHEN** the sidebar is a column and the window is 381 columns wide
- **THEN** the column shows one bottom panel, as before

### Requirement: a split strip shows three panels

A split strip SHALL show, from left to right: the vertical filter bar, the
agent panel, divider 1, pane A, divider 2 and pane B. Each divider SHALL be
one column wide and SHALL draw the same thin line as the one-pane strip's
divider. Each part except the tab bars SHALL use the full height of the
strip. The agent panel and the filter bar SHALL be the same as in a
one-pane strip.

#### Scenario: the parts of a split strip
- **WHEN** a strip is 400 columns wide, the split options are not set, and the strip shows the split layout
- **THEN** columns 0-3 are the filter bar, 4-135 the agent panel, 136 divider 1, 137-267 pane A, 268 divider 2 and 269-399 pane B

### Requirement: each pane has a fixed tab group

Pane A SHALL show only the Tools and Git tabs. Pane B SHALL show only the
Tasks and Thought tabs. Pane B SHALL show the Thought tab only while the
focused pane runs pi. A tab SHALL NOT show in both panes.

#### Scenario: the tabs for a pi pane
- **WHEN** a split strip shows and the focused pane runs pi
- **THEN** pane A offers Tools and Git, and pane B offers Tasks and Thought

#### Scenario: the tabs for a Claude pane
- **WHEN** a split strip shows and the focused pane runs Claude
- **THEN** pane A offers Tools and Git, and pane B offers Tasks only

### Requirement: a hidden Thought tab in pane B displays as Tasks

When pane B's stored tab is Thought and the focused pane does not run pi,
pane B SHALL display the Tasks tab and SHALL NOT write its stored tab.
When the focus returns to a pi pane, pane B SHALL display the Thought tab
again.

#### Scenario: the focus moves to a Claude pane
- **WHEN** pane B shows the Thought tab and the user focuses a Claude pane
- **THEN** pane B displays the Tasks tab and `@sidebar_split_tab_b` stays `thought`

#### Scenario: the focus returns to a pi pane
- **WHEN** `@sidebar_split_tab_b` is `thought`, pane B displays Tasks, and the user focuses a pi pane
- **THEN** pane B displays the Thought tab

### Requirement: each pane stores its own tab

`@sidebar_split_tab_a` SHALL store pane A's tab as `activity` or `git`,
and `@sidebar_split_tab_b` SHALL store pane B's tab as `tasks` or
`thought`. A value that is not set or that names no tab of the pane's
group SHALL select `activity` for pane A and `tasks` for pane B. A tab
selection in a split strip SHALL write only the option of its own pane.

#### Scenario: the defaults
- **WHEN** neither split tab option is set and a split strip shows
- **THEN** pane A shows Tools and pane B shows Tasks

#### Scenario: a value from the other group
- **WHEN** `@sidebar_split_tab_a` is `tasks`
- **THEN** pane A shows Tools

#### Scenario: a selection in pane A
- **WHEN** pane A shows Tools and pane B shows Tasks, and the user selects Git in pane A
- **THEN** `@sidebar_split_tab_a` is `git`, and `@sidebar_split_tab_b` and `@sidebar_bottom_tab` do not change

### Requirement: the panel widths are percents of the strip

`@sidebar_split_agents_width` SHALL give the agent panel's width, and
`@sidebar_split_tab_b_width` SHALL give pane B's width with divider 2, each
as a percent of the strip width rounded to the nearest column. A value that
is not set, empty, or not a number from 1 to 100 SHALL be 33. Pane A with
divider 1 SHALL take the columns that are left. A split strip SHALL NOT
read `@sidebar_tab_width`.

#### Scenario: the default widths
- **WHEN** neither split width option is set and a split strip is 400 columns wide
- **THEN** the agent panel takes 132 columns, pane B with divider 2 takes 132 columns, and pane A with divider 1 takes 132 columns

#### Scenario: the one-pane width does not change a split strip
- **WHEN** `@sidebar_tab_width` is 50 and a split strip is 400 columns wide
- **THEN** the agent panel still takes 132 columns

### Requirement: each split panel keeps a minimum width

In a split strip, the agent panel, pane A with divider 1, and pane B with
divider 2 SHALL each keep at least 20 columns. The agent panel width SHALL
be clamped first, then pane B's width. When the strip cannot give the
filter bar and each of the three panels its minimum, the strip SHALL show
one tab panel, also above the threshold.

#### Scenario: a large agent percent
- **WHEN** `@sidebar_split_agents_width` is 95 and a split strip is 400 columns wide
- **THEN** the agent panel takes 356 columns, and pane A and pane B each take 20 columns with their dividers

#### Scenario: a low threshold on a narrow strip
- **WHEN** `@sidebar_tab_split_width` is 50 and the strip is 60 columns wide
- **THEN** the strip shows one tab panel

### Requirement: each pane has its own tab bar

In a split strip, the top row of pane A with divider 1 SHALL be pane A's
tab bar, and the top row of pane B with divider 2 SHALL be pane B's tab
bar. Each bar SHALL be the band the one-pane strip's tab bar draws, with
the pane's shown tab glyphs right-aligned and its displayed tab in the
accent colour. Pane A's bar SHALL draw Tools then Git, and pane B's
bar SHALL draw Tasks then Thought. Only pane A's bar SHALL carry the focused pane's id and
name; pane B's bar SHALL draw no label.

#### Scenario: the bars for a pi pane
- **WHEN** a split strip shows and the focused pane runs pi
- **THEN** pane A's bar draws the pane label and the Tools and Git glyphs, and pane B's bar draws the Tasks and Thought glyphs and no label

### Requirement: a click on a pane's tab bar selects a tab in that pane

When the user presses and releases the left mouse button on a pane's tab
bar without a move to a different column, a press on a glyph's drawn
columns SHALL select that tab in that pane on release. A press on a column
with no glyph SHALL change nothing.

#### Scenario: a click on pane B's Thought glyph
- **WHEN** the focused pane runs pi, pane B shows Tasks, and the user clicks the Thought glyph on pane B's bar
- **THEN** pane B shows Thought and pane A does not change

### Requirement: the user resizes a split strip by dragging a divider

A press on divider 1 or on pane A's tab bar that moves to a different
column SHALL drag divider 1: the agent panel and pane A trade columns and
pane B does not change. A press on divider 2 or on pane B's tab bar that
moves SHALL drag divider 2: pane A and pane B trade columns and the agent
panel does not change. The divider SHALL follow the mouse column within the
minimum widths.

#### Scenario: a drag stops at a minimum
- **WHEN** a split strip is 400 columns wide with the default widths
- **AND** the user drags divider 2 to column 140
- **THEN** divider 2 stops so that pane A with divider 1 keeps 20 columns

#### Scenario: a divider 1 drag stops at pane A's minimum
- **WHEN** a split strip is 400 columns wide with the default widths
- **AND** the user drags divider 1 to column 350
- **THEN** divider 1 stops at column 248, so the agent panel takes 244 columns, pane A with divider 1 keeps 20 columns, and pane B does not change

### Requirement: a divider drag saves its width on release

On the release of a moved divider drag, the sidebar SHALL write the new
width once, as a percent of the strip width rounded to the nearest
percent: divider 1 to `@sidebar_split_agents_width`, divider 2 to
`@sidebar_split_tab_b_width`.

#### Scenario: a drag of divider 1
- **WHEN** a split strip is 400 columns wide with the default widths
- **AND** the user drags divider 1 from column 136 to column 104
- **THEN** the agent panel takes 100 columns, pane B does not change, and `@sidebar_split_agents_width` is 25 after release

#### Scenario: a drag of divider 2
- **WHEN** a split strip is 400 columns wide with the default widths
- **AND** the user drags divider 2 from column 268 to column 300
- **THEN** pane B with divider 2 takes 100 columns, the agent panel does not change, and `@sidebar_split_tab_b_width` is 25 after release

#### Scenario: no tmux write during the drag
- **WHEN** the user drags a divider across several columns
- **THEN** the divider's option changes only once, on release, and `@sidebar_tab_width` does not change

### Requirement: the keys follow the split strip

In a split strip, Left and Right (and `h` and `l`) SHALL move the keyboard
focus in the order filter bar, agent panel, pane A, pane B, and the focus
SHALL stay at an end. Up and Down SHALL scroll the focused tab pane. The
tab cycle key SHALL cycle the tabs of the focused tab pane, and of pane A
when no tab pane has focus. A mouse wheel event SHALL scroll the tab pane
under the mouse.

#### Scenario: Right from pane A
- **WHEN** a split strip shows and pane A has focus
- **AND** the user presses Right
- **THEN** pane B has focus

#### Scenario: Right at the right end
- **WHEN** pane B has focus and the user presses Right
- **THEN** pane B keeps focus

#### Scenario: the cycle key in pane B
- **WHEN** the focused pane runs pi, pane B has focus and shows Tasks
- **AND** the user presses the tab cycle key
- **THEN** pane B shows Thought and pane A does not change

#### Scenario: the split layout ends while pane B has focus
- **WHEN** pane B has focus and the strip becomes narrower than the threshold
- **THEN** the one tab panel has focus

### Requirement: the keyboard-owning panel gets the active fill

In a split strip, the agent section (filter bar and agent panel), pane A,
and pane B SHALL each be one section for the focus fill. The section that
owns the keyboard SHALL get the active focus background colour, and the
other two SHALL get the base one.

#### Scenario: pane B owns the keyboard
- **WHEN** a split strip shows and pane B has focus
- **THEN** pane B is painted with the active focus background colour, and the agent section and pane A with the base one
