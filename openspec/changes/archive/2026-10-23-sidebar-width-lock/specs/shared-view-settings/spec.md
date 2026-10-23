# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: four view settings are shared`
- TO: `### Requirement: the view settings are shared`

## MODIFIED Requirements

### Requirement: the view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor position in the agent list, the selected bottom
tab, and — while the width lock is on — the sidebar's docked width. A
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
- **THEN** the sidebar of window B selects the third agent row

#### Scenario: a bottom tab change applies to another window
- **WHEN** the user switches the bottom panel of the sidebar of window A to the Tasks tab
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the Tasks tab

#### Scenario: a docked-width change applies to another window
- **WHEN** the width lock is on and the user resizes the docked sidebar of window A
- **THEN** the docked sidebar of window B shows the same width within one refresh tick

### Requirement: the rest of the view state stays local

Each sidebar instance SHALL keep its own focus, scroll position, open
popups and git data, and — while the width lock is off — its own docked
width. A change to these in one instance SHALL NOT change them in another
instance.

#### Scenario: a scroll change stays local
- **WHEN** the user scrolls the activity log down in the sidebar of window A
- **THEN** the sidebar of window B keeps its own activity log scroll position

#### Scenario: a width change stays local while unlocked
- **WHEN** the width lock is off and the user resizes the docked sidebar of window A
- **THEN** the sidebar of window B keeps its own width

## ADDED Requirements

### Requirement: the width lock is shared

All sidebar instances on one tmux server SHALL share one width-lock state,
on or off. Toggling the lock in one instance SHALL apply to every other
instance on the same server within one refresh tick. A tmux server that
holds no width-lock value SHALL behave as unlocked.

#### Scenario: locking in one window locks all windows
- **WHEN** the user enables the width lock in the sidebar of window A
- **THEN** the sidebar of window B treats the lock as on within one refresh tick

#### Scenario: a first instance on a fresh server
- **WHEN** the tmux server holds no width-lock value
- **AND** a sidebar instance starts
- **THEN** it runs with the lock off

### Requirement: enabling the lock seeds the shared width

When the width lock goes from off to on, the shared docked width SHALL
take the enabling instance's docked width at that moment, clamped at the
minimum-width floor. An instance that is floating when it enables the
lock SHALL seed the docked width it captured on the way out, or the
floating panel's width when no docked width was captured.

#### Scenario: the enabling instance's width becomes the shared width
- **WHEN** the user enables the lock on a docked sidebar of 30 columns
- **THEN** the shared docked width is 30 columns and every other docked
  sidebar on the server adopts it within one refresh tick

#### Scenario: enabling while floating seeds the captured width
- **WHEN** the user pops a 25 column sidebar out and then enables the lock
- **THEN** the shared docked width is 25 columns and every docked sidebar
  on the server adopts it within one refresh tick

### Requirement: unlocking keeps every sidebar at its current width

When the width lock goes from on to off, each sidebar SHALL keep the width
it has at that moment. A later width change in one instance SHALL NOT
change another instance's width.

#### Scenario: a resize stays local after unlocking
- **WHEN** the lock is turned off and the user later resizes the docked sidebar of window A
- **THEN** the sidebar of window B keeps the width it had

### Requirement: a failed width publish is not reverted

When an instance cannot write its docked width to the shared option, the
next refresh tick SHALL NOT resize its pane back to the stale shared
value, and the write SHALL be retried.

#### Scenario: the tmux write fails
- **WHEN** the lock is on, the user resizes the docked sidebar to 45 columns, and the write of the new shared width to tmux fails
- **THEN** the sidebar stays at 45 columns after the next refresh tick and the write is retried until it lands
