# status-filter Specification

## Purpose
Defines the status filter row of the sidebar: the categories it offers, how
each category is counted, how the active category is marked, and how a
category is selected.

## Requirements

### Requirement: the row offers five categories

The status filter row SHALL offer the categories all, busy, waiting, idle
and error, in that order. The busy category SHALL cover the panes that are
running and the panes that are in the background, because both mean the
agent is busy and neither needs the user.

#### Scenario: the categories of the row
- **WHEN** the status filter row is rendered
- **THEN** it shows the chips all, busy, waiting, idle and error, in that order

#### Scenario: a background pane is busy
- **WHEN** the user selects the busy category
- **THEN** the list shows the running panes and the background panes

### Requirement: the busy count covers both busy states

The busy chip SHALL count the running panes and the background panes
together. The all chip SHALL count every pane the status filter can show.

#### Scenario: the busy count
- **WHEN** one pane is running and one pane is in the background
- **THEN** the busy chip shows 2

### Requirement: the busy icon keeps the running colour

The busy chip icon SHALL be drawn in the running colour whether or not the
busy chip is the active category. The other chips SHALL take their own
status colour when they are the active category and the inactive chip colour
when they are not.

#### Scenario: the busy chip is not the active category
- **WHEN** another category is active and the busy chip is rendered
- **THEN** the busy chip icon is drawn in the running colour

### Requirement: the active category is marked by its count

The count of the active category SHALL be drawn in the accent colour, so
that the active category stays visible when its icon colour does not change.

#### Scenario: the active chip
- **WHEN** the busy category is the active one
- **THEN** the busy chip count is drawn in the accent colour

#### Scenario: the chip counts
- **WHEN** the status filter row is rendered
- **THEN** the count of the active chip is drawn in the accent colour, a
  non-zero count on an inactive chip is drawn in the ordinary text colour,
  and a zero count on an inactive chip is drawn dimmed in the inactive
  chip colour

### Requirement: the chips are separated by a single space

The chips SHALL be separated by a single space, so that the row and the
pop-out control fit the minimum sidebar width of 20 columns.

#### Scenario: the row fits the minimum width
- **WHEN** the sidebar is 20 columns wide and every count is a single digit
- **THEN** the chips and the pop-out control fit on the row without anything being cut off

### Requirement: selecting a category filters the list and persists

Selecting a category SHALL show only the panes of that category in the agent
list and SHALL persist the selection, so that a new sidebar instance starts
with the same category. The persisted value of a category that no longer
exists SHALL map onto the busy category rather than onto all.

#### Scenario: the selection persists
- **WHEN** the user selects the busy category and the sidebar is restarted
- **THEN** the sidebar starts with the busy category selected

#### Scenario: an old persisted value
- **WHEN** the persisted filter value is `background`
- **THEN** the busy category is selected

### Requirement: the keyboard cycles the categories

The keyboard SHALL cycle the active category through the categories in the
order the row shows them, and the cycle SHALL wrap around.

#### Scenario: the cycle order
- **WHEN** the user cycles forward from the all category
- **THEN** the busy category is active, and cycling forward from the error category returns to the all category
