# bottom-panel Specification

## Purpose
Defines the layout of the Activity/Tasks/Git panel under the agent list:
the rule that separates the two parts and holds the tabs, the height of
the panel, and how the user resizes the split with the mouse.

## Requirements

### Requirement: the bottom panel is a split part under the agent list

The bottom panel SHALL be directly under the agent list, with no blank
row between them. One horizontal rule SHALL be the top row of the bottom
panel and SHALL separate the two parts. The bottom panel SHALL have no
side border and no bottom border, so its content uses the full width of
the sidebar.

#### Scenario: the panel has only the rule
- **WHEN** the sidebar shows the bottom panel
- **THEN** the row directly above the panel content is a horizontal rule across the full sidebar width
- **AND** no border character is drawn at the left edge, the right edge, or the bottom row of the panel

#### Scenario: the content uses the full width
- **WHEN** the sidebar is 40 columns wide and shows the Activity tab
- **THEN** each activity line has 40 columns available

### Requirement: the rule holds the tabs

The rule SHALL show the `Activity`, `Tasks` and `Git` tab labels near its
left end, with the selected tab in the accent color. The rule SHALL use
the accent color when the bottom panel has focus, and the inactive border
color when it does not. When the sidebar is too narrow for all labels,
the labels SHALL truncate and the rule SHALL stay exactly as wide as the
sidebar.

#### Scenario: the rule shows the tabs
- **WHEN** the sidebar is 30 columns wide and the Activity tab is selected
- **THEN** the rule reads `─ Activity│Tasks│Git ` followed by `─` to the right edge
- **AND** `Activity` is in the accent color

#### Scenario: the rule follows the focus
- **WHEN** the user moves the focus to the bottom panel
- **THEN** the rule changes to the accent color

#### Scenario: a narrow sidebar
- **WHEN** the sidebar is 20 columns wide
- **THEN** the tab labels truncate and the rule is 20 columns wide

### Requirement: the panel height is a percent of the sidebar height

The `@sidebar_bottom_height` option SHALL give the height of the bottom
panel, rule included, as a percent of the sidebar height. The row count
SHALL be the percent of the sidebar height, rounded to the nearest row.
When the option is not set, empty, or not a number from 0 to 100, the
height SHALL be 30 percent. The value 0 SHALL hide the bottom panel.

#### Scenario: the default height
- **WHEN** `@sidebar_bottom_height` is not set and the sidebar is 40 rows high
- **THEN** the bottom panel is 12 rows high, rule included, and the agent list has the other 28 rows

#### Scenario: the height follows the sidebar height
- **WHEN** `@sidebar_bottom_height` is 30 and the sidebar height changes from 40 to 60 rows
- **THEN** the bottom panel height changes from 12 to 18 rows

#### Scenario: zero hides the panel
- **WHEN** `@sidebar_bottom_height` is 0
- **THEN** the sidebar shows no rule and no bottom panel, and the agent list has all rows

#### Scenario: an invalid value
- **WHEN** `@sidebar_bottom_height` is `abc` or `150`
- **THEN** the bottom panel is 30 percent of the sidebar height

### Requirement: each part keeps a minimum height

The bottom panel SHALL be at least 3 rows high (the rule and 2 content
rows), and the agent list SHALL keep at least 3 rows. A height outside
these limits SHALL be clamped to them. When the sidebar is less than 6
rows high, the bottom panel SHALL not show.

#### Scenario: a small percent
- **WHEN** `@sidebar_bottom_height` is 1 and the sidebar is 40 rows high
- **THEN** the bottom panel is 3 rows high

#### Scenario: a large percent
- **WHEN** `@sidebar_bottom_height` is 100 and the sidebar is 40 rows high
- **THEN** the bottom panel is 37 rows high and the agent list keeps 3 rows

#### Scenario: a very short sidebar
- **WHEN** the sidebar is 5 rows high
- **THEN** the sidebar shows no bottom panel

### Requirement: the user resizes the split by dragging the rule

When the user presses the left mouse button on the rule and moves the
mouse to a different row before release, the rule SHALL follow the mouse
row on each move, within the minimum heights. On release, the sidebar
SHALL keep the new height and save it as a percent of the sidebar height,
rounded to the nearest percent. The saved percent SHALL be inside the
minimum heights.

#### Scenario: a drag up
- **WHEN** the sidebar is 40 rows high and the rule is on row 28
- **AND** the user presses on the rule and drags it to row 20
- **THEN** the rule is on row 20 during the drag and after release
- **AND** `@sidebar_bottom_height` is 50 after release

#### Scenario: a drag past the limit
- **WHEN** the sidebar is 40 rows high
- **AND** the user drags the rule to row 1
- **THEN** the rule stops on row 3, so the agent list keeps 3 rows
- **AND** the saved percent gives the same 37-row panel

#### Scenario: a drag that starts on a tab label
- **WHEN** the user presses on the `Tasks` label and drags to a different row
- **THEN** the split resizes and the selected tab does not change

#### Scenario: no tmux write during the drag
- **WHEN** the user drags the rule across several rows
- **THEN** `@sidebar_bottom_height` changes only once, on release

### Requirement: a click on the rule selects a tab

When the user presses and releases the left mouse button on the rule
without a move to a different row, the press SHALL be a click. A click on
a tab label SHALL select that tab on release. A click on the rule outside
the tab labels SHALL change nothing.

#### Scenario: a click on a tab label
- **WHEN** the Activity tab is selected
- **AND** the user presses and releases on the `Git` label
- **THEN** the Git tab is selected after the release, and the split height does not change

#### Scenario: a click on the dashes
- **WHEN** the user presses and releases on the `─` fill to the right of the labels
- **THEN** the tab and the split height do not change

#### Scenario: a click on the left end of the rule
- **WHEN** the Tasks tab is selected
- **AND** the user presses and releases on the `─ ` to the left of the `Activity` label
- **THEN** the Tasks tab stays selected
