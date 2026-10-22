# Spec Delta

## Purpose

Lets any agent keep a visible todo list in the sidebar: the agent records items through one shell command, and the human reads them in a Tasks tab beside Activity and Git.

## ADDED Requirements

### Requirement: A Tasks tab shows the focused pane todo list
The bottom panel SHALL carry a third tab named Tasks beside Activity and Git. The Tasks tab SHALL show the todo list of the focused pane. The tab SHALL stay local to each sidebar instance like the other bottom tabs.

#### Scenario: A pane with tasks
- **WHEN** the focused pane has recorded tasks
- **THEN** the Tasks tab lists each item with its status

#### Scenario: A pane with no tasks
- **WHEN** the focused pane has recorded no tasks
- **THEN** the Tasks tab shows an empty state instead of a list

#### Scenario: Focus follows the pane
- **WHEN** focus moves to another pane
- **THEN** the Tasks tab shows that pane's list

#### Scenario: The tab stays local
- **WHEN** one sidebar instance switches to the Tasks tab
- **THEN** no other instance changes its bottom tab

### Requirement: Any agent writes tasks through one shell command
The sidebar SHALL provide a shell subcommand that records todo items for the calling pane. The pane SHALL be taken from `$TMUX_PANE` in the command's environment, the same way `set-status` names its pane. The command SHALL work for every agent because every agent can run a shell command, and it SHALL require no hook registration.

#### Scenario: An agent records a task
- **WHEN** an agent runs the task command with a subject in its own pane
- **THEN** that pane's todo list holds the new item as pending

#### Scenario: An agent updates a task
- **WHEN** an agent runs the task command with an existing item id and a new status
- **THEN** that pane's todo list shows the new status for that item

#### Scenario: Outside tmux nothing is recorded
- **WHEN** the command runs with no `$TMUX_PANE` in its environment
- **THEN** no todo list changes and the command exits successfully

### Requirement: The tab cycles with the other tabs
The bottom-tab cycle SHALL cover Activity, Tasks, and Git in that order. The existing toggle key and tab-header clicks SHALL reach all three tabs. Scroll input SHALL move the visible tab's own scroll offset.

#### Scenario: Keyboard cycles three tabs
- **WHEN** the user toggles the bottom tab repeatedly
- **THEN** the tab visits Activity, Tasks, and Git in order

#### Scenario: Click reaches Tasks
- **WHEN** the user clicks the Tasks title in the tab header
- **THEN** the bottom panel shows the Tasks tab

#### Scenario: Scroll stays per tab
- **WHEN** the user scrolls in the Tasks tab and switches away and back
- **THEN** the Tasks scroll offset is kept and the other tabs keep theirs

### Requirement: Reminders are conditional with common wording
The task write path SHALL carry two reminders in one common wording: record-first when a turn starts with an empty todo list, and update-before-finish when a turn ends with open items. A reminder SHALL fire only in its condition. No reminder SHALL fire per tool call or per turn when the list is already fresh.

#### Scenario: Empty list at turn start
- **WHEN** a turn starts and the pane's todo list is empty
- **THEN** the agent receives one short reminder to record tasks first

#### Scenario: Fresh list at turn start
- **WHEN** a turn starts and the pane's todo list already holds items
- **THEN** the agent receives no reminder

#### Scenario: Open items at turn end
- **WHEN** a turn ends and items are still open
- **THEN** the agent receives one short reminder to update their statuses

#### Scenario: Nothing open at turn end
- **WHEN** a turn ends and no items are open
- **THEN** the agent receives no reminder
