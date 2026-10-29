# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: five view settings are shared`
- TO: `### Requirement: six view settings are shared`

## MODIFIED Requirements

### Requirement: six view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor position in the agent list, the selected bottom
tab, the bottom panel height, and the strip tab width. A change to one of
these settings in one instance SHALL apply to every other instance on the
same tmux server.

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

#### Scenario: a strip tab width change applies to another window
- **WHEN** the user drags the divider in the strip of window A so that the tab panel is 50 percent wide
- **AND** the user then goes to window B, which also has a strip
- **THEN** the strip of window B shows a tab panel that is 50 percent wide

### Requirement: a drag in progress is not reverted

While the user drags the rule of a column or the divider of a strip, a
refresh tick SHALL NOT move the rule or the divider to the size that tmux
holds. After release, the instance SHALL adopt the shared size again on the
refresh ticks that follow.

#### Scenario: a refresh tick during a drag
- **WHEN** the user presses on the rule and drags it for 3 seconds
- **THEN** the rule stays on the mouse row through every refresh tick of the drag

#### Scenario: a refresh tick during a divider drag
- **WHEN** the user presses on the divider of a strip and drags it for 3 seconds
- **THEN** the divider stays on the mouse column through every refresh tick of the drag

#### Scenario: another instance changes the height after release
- **WHEN** the user released a drag in window A
- **AND** a later drag in window B sets a different height
- **THEN** the sidebar of window A shows the height from window B within 1 second

## ADDED Requirements

### Requirement: the strip tab width is shared apart from the column height

All strips on one tmux server SHALL share `@sidebar_tab_width`. A divider
drag in a strip SHALL NOT change `@sidebar_bottom_height`, and a rule drag
in a column SHALL NOT change `@sidebar_tab_width`, so a column and a strip
on the same server keep their own split.

#### Scenario: a strip drag leaves a column alone
- **WHEN** window A has a strip and window B has a column with a bottom panel 30 percent high
- **AND** the user drags the divider in window A to 60 percent
- **THEN** the bottom panel of window B stays 30 percent high

#### Scenario: a column drag leaves a strip alone
- **WHEN** window A has a column and window B has a strip with a tab panel 40 percent wide
- **AND** the user drags the rule in window A
- **THEN** the tab panel of window B stays 40 percent wide
