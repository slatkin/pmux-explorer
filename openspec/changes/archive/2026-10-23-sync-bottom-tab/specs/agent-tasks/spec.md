# Spec Delta

## Purpose

Lets any agent keep a visible todo list in the sidebar: the agent records items through one shell command, and the human reads them in a Tasks tab beside Activity and Git.

## REMOVED Requirements

### Requirement: A Tasks tab shows the focused pane todo list

- **Reason**: the requirement states the tab SHALL stay local to each
  sidebar instance, which the shared bottom tab replaces; the rest of its
  behavior is kept in the requirement added below.
- **Migration**: the pane-list behavior (which pane's tasks are shown,
  empty state, focus following the pane) moves unchanged to "The Tasks tab
  shows the focused pane todo list" added below; the "stays local" clause
  and its scenario are replaced by the two shared-tab scenarios there.

## ADDED Requirements

### Requirement: The Tasks tab shows the focused pane todo list

The bottom panel SHALL carry a third tab named Tasks beside Activity and Git. The Tasks tab SHALL show the todo list of the focused pane. The selected tab SHALL be shared with the other sidebar instances on the same tmux server, and focusing a different pane SHALL NOT change the selected tab.

#### Scenario: A pane with tasks
- **WHEN** the focused pane has recorded tasks
- **THEN** the Tasks tab lists each item with its status

#### Scenario: A pane with no tasks
- **WHEN** the focused pane has recorded no tasks
- **THEN** the Tasks tab shows an empty state instead of a list

#### Scenario: Focus follows the pane
- **WHEN** focus moves to another pane
- **THEN** the Tasks tab shows that pane's list

#### Scenario: The tab applies to every instance
- **WHEN** one sidebar instance switches to the Tasks tab
- **THEN** every other sidebar instance on the same tmux server shows the Tasks tab within one refresh tick

#### Scenario: The tab survives a focus change
- **WHEN** the sidebar shows the Tasks tab and the user focuses another pane
- **THEN** the sidebar still shows the Tasks tab
