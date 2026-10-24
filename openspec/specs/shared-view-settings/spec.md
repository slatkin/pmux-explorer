# shared-view-settings Specification

## Purpose
Defines which sidebar view settings all sidebar instances on one tmux server
share, and how fast an instance shows a change made in another instance.

## Requirements

### Requirement: five view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor position in the agent list, the selected bottom
tab, and the bottom panel height. A change to one of these settings in one
instance SHALL apply to every other instance on the same tmux server.

#### Scenario: a filter change applies to another window
- **WHEN** the user selects the waiting status filter in the sidebar of window A
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the waiting status filter

#### Scenario: a repo filter change applies to another window
- **WHEN** the user selects a repo in the repo filter of the sidebar of window A
- **THEN** the sidebar of window B shows the same repo filter

#### Scenario: a cursor move applies to another window
- **WHEN** the user moves the cursor to the third agent row in the sidebar of window A
- **THEN** the sidebar of window B selects the third agent row

#### Scenario: a bottom tab change applies to another window
- **WHEN** the user switches the bottom panel of the sidebar of window A to the Tasks tab
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the Tasks tab

### Requirement: a shared change shows within one refresh tick

A sidebar instance SHALL show a shared setting changed by another instance
within one refresh tick (1 second) of the change. This SHALL apply whether
the window of the instance is visible or not, and whether the instance is in
the same tmux session or in a different one.

#### Scenario: an instance in a different session
- **WHEN** the user changes the status filter in a sidebar in session A
- **AND** the user switches the client to session B
- **THEN** the sidebar in session B shows the new status filter within 1 second

#### Scenario: a fast window switch
- **WHEN** the user goes from window A to window B, changes the status filter there, and goes back to window A in less than 2 seconds
- **THEN** the sidebar of window A shows the new status filter within 1 second

#### Scenario: an instance that the user does not look at
- **WHEN** the user changes the status filter in window A
- **THEN** the sidebar of window B shows the new status filter within 1 second, also while window A stays the visible window

### Requirement: a local change is not reverted by a failed save

When an instance cannot write a changed shared setting to tmux, the instance
SHALL keep the value the user selected. The next refresh tick SHALL NOT
replace it with the older value that tmux still holds.

#### Scenario: the tmux write fails
- **WHEN** the user changes the status filter from error to running
- **AND** the write of the new filter to tmux fails
- **THEN** the sidebar continues to show the running filter after the next refresh tick

### Requirement: a pending cursor move is not reverted

While a cursor move in an instance waits for its delayed write to tmux, the
next refresh tick SHALL NOT move the cursor back to the older value that
tmux still holds.

#### Scenario: the user moves the cursor quickly
- **WHEN** the user moves the cursor down two rows
- **AND** a refresh tick runs before the cursor write reaches tmux
- **THEN** the cursor stays on the row that the user selected

### Requirement: the rest of the view state stays local

Each sidebar instance SHALL keep its own focus, scroll position, open
popups and git data. A change to these in one instance SHALL NOT change
them in another instance.

#### Scenario: a scroll change stays local
- **WHEN** the user scrolls the activity log down in the sidebar of window A
- **THEN** the sidebar of window B keeps its own activity log scroll position

### Requirement: focusing a different agent does not change the bottom tab

The bottom tab SHALL change only when the user selects a tab — with the
cycle key, with a click on a tab title, or in another sidebar instance
whose selection this instance adopts. Focusing a different pane or a
different agent SHALL leave the selected tab alone.

#### Scenario: clicking a different agent keeps the tab
- **WHEN** the sidebar shows the Tasks tab
- **AND** the user focuses another agent pane in the same window
- **THEN** the sidebar still shows the Tasks tab

#### Scenario: an agent starting in the focused pane keeps the tab
- **WHEN** the sidebar shows the Git tab
- **AND** an agent starts in the currently focused pane
- **THEN** the sidebar still shows the Git tab

#### Scenario: a shell pane keeps the tab
- **WHEN** the sidebar shows the Tasks tab
- **AND** the user focuses a pane with no agent running
- **THEN** the sidebar still shows the Tasks tab

### Requirement: an instance without a shared tab shows Activity

A sidebar instance SHALL show the Activity tab when the tmux server holds
no bottom tab value — on a server where no instance has selected a tab yet.

#### Scenario: a first instance on a fresh server
- **WHEN** the tmux server holds no bottom tab value
- **AND** a sidebar instance starts
- **THEN** it shows the Activity tab

### Requirement: the split height is shared

All sidebar instances on one tmux server SHALL share the bottom panel
height (`@sidebar_bottom_height`). A height that the user sets with a
drag in one instance SHALL apply to every other instance on the same
tmux server, within the refresh time of the other shared settings.

#### Scenario: a drag applies to another window
- **WHEN** the user drags the rule in the sidebar of window A so that the bottom panel is 50 percent high
- **AND** the user then goes to window B
- **THEN** the sidebar of window B shows a bottom panel that is 50 percent high

#### Scenario: a different sidebar height
- **WHEN** the sidebar of window A is 40 rows high and the sidebar of window B is 60 rows high
- **AND** the user sets 50 percent with a drag in window A
- **THEN** the bottom panel is 20 rows high in window A and 30 rows high in window B

### Requirement: a drag in progress is not reverted

While the user drags the rule, a refresh tick SHALL NOT move the rule to
the height that tmux holds. After release, the instance SHALL adopt the
shared height again on the refresh ticks that follow.

#### Scenario: a refresh tick during a drag
- **WHEN** the user presses on the rule and drags it for 3 seconds
- **THEN** the rule stays on the mouse row through every refresh tick of the drag

#### Scenario: another instance changes the height after release
- **WHEN** the user released a drag in window A
- **AND** a later drag in window B sets a different height
- **THEN** the sidebar of window A shows the height from window B within 1 second
