# Spec Delta

## MODIFIED Requirements

### Requirement: the view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor's agent pane in the agent list, the selected bottom
tab, the bottom panel height, the strip tab width, and — while the width
lock is on — the sidebar's docked width and a docked strip's height. A
change to one of these settings in one instance SHALL apply to every other
instance on the same tmux server.

#### Scenario: a filter change applies to another window
- **WHEN** the user selects the waiting status filter in the sidebar of window A
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the waiting status filter

#### Scenario: a repo filter change applies to another window
- **WHEN** the user selects a repo in the repo filter of the sidebar of window A
- **THEN** the sidebar of window B shows the same repo filter

#### Scenario: a cursor move applies to another window
- **WHEN** the user moves the cursor to the third agent row in the sidebar of window A
- **THEN** the sidebar of window B puts its cursor on the same agent pane, at whatever row that pane occupies in window B's own list

#### Scenario: a bottom tab change applies to another window
- **WHEN** the user switches the bottom panel of the sidebar of window A to the Tasks tab
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the Tasks tab

#### Scenario: a docked-width change applies to another window
- **WHEN** the width lock is on and the user resizes the docked sidebar of window A
- **THEN** the docked sidebar of window B shows the same width within one refresh tick

#### Scenario: a strip tab width change applies to another window
- **WHEN** the user drags the divider in the strip of window A so that the tab panel is 50 percent wide
- **AND** the user then goes to window B, which also has a strip
- **THEN** the strip of window B shows a tab panel that is 50 percent wide

#### Scenario: a docked strip height change applies to another window
- **WHEN** the width lock is on and the user resizes the docked strip of window A
- **THEN** the docked strip of window B shows the same height within one refresh tick

### Requirement: enabling the lock seeds the shared width

When the width lock goes from off to on, the shared docked size of the
enabling instance's axis SHALL take that instance's docked size at that
moment, clamped at the axis floor: a column seeds the shared docked width,
a strip the shared docked height. An instance that is floating when it
enables the lock SHALL seed the docked size it captured on the way out, or
— a column only — the floating panel's width when no docked width was
captured; a floating strip with no captured height SHALL seed nothing. The shared docked size of the other axis SHALL be cleared, and
instances on that axis SHALL keep their sizes until one of them publishes.

#### Scenario: the enabling instance's width becomes the shared width
- **WHEN** the user enables the lock on a docked sidebar of 30 columns
- **THEN** the shared docked width is 30 columns and every other docked
  sidebar on the server adopts it within one refresh tick

#### Scenario: enabling while floating seeds the captured width
- **WHEN** the user pops a 25 column sidebar out and then enables the lock
- **THEN** the shared docked width is 25 columns and every docked sidebar
  on the server adopts it within one refresh tick

#### Scenario: enabling from a strip clears the shared width
- **WHEN** the shared docked width holds 45 columns from an earlier lock session, a docked column is 30 columns wide, and the user enables the lock from a strip
- **THEN** the shared docked width is cleared and the column stays 30 columns wide
