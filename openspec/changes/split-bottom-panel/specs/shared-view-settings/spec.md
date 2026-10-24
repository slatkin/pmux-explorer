# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: four view settings are shared`
- TO: `### Requirement: five view settings are shared`

## MODIFIED Requirements

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

## ADDED Requirements

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
