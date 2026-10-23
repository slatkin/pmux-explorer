# Spec Delta

## Purpose

Defines which sidebar view settings all sidebar instances on one tmux server
share, and how fast an instance shows a change made in another instance.

## RENAMED Requirements

- FROM: `### Requirement: three view settings are shared`
- TO: `### Requirement: four view settings are shared`

## MODIFIED Requirements

### Requirement: four view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor position in the agent list, and the selected bottom
tab. A change to one of these settings in one instance SHALL apply to every
other instance on the same tmux server.

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

## REMOVED Requirements

### Requirement: other view state stays local

- **Reason**: the bottom tab is now one of the shared view settings, so it
  no longer belongs to the list of state each instance keeps for itself.
- **Migration**: the remaining local state — focus, scroll position, open
  popups and git data — is re-stated by the requirement "the rest of the
  view state stays local" added below, without the bottom tab.

## ADDED Requirements

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
