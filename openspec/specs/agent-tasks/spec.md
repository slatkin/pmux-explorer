# agent-tasks Specification

## Purpose
Lets any agent keep a visible todo list in the sidebar: the agent records items through one shell command, and the human reads them in a Tasks tab beside Activity and Git.

## Requirements

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

### Requirement: A finished task list closes after ninety seconds
When every item of a pane's todo list is completed and the list is unchanged for 90 seconds, the sidebar SHALL empty the list — items and pane-wide description — and the Tasks tab SHALL show the empty state. The rule SHALL apply to every pane's list, focused or not.

#### Scenario: A finished list closes after the window
- **WHEN** every item of a pane's todo list is completed and the list is unchanged for 90 seconds
- **THEN** the sidebar empties the list and the Tasks tab shows the empty state

#### Scenario: A finished list stays during the window
- **WHEN** every item is completed but the list has been unchanged for less than 90 seconds
- **THEN** the Tasks tab still shows the list

#### Scenario: The close takes the description with it
- **WHEN** the list closes and the pane had a pane-wide description
- **THEN** the Tasks tab shows the empty state with no description

#### Scenario: An unfocused pane's list closes on time
- **WHEN** a finished list belongs to a pane that is not focused
- **THEN** it closes after the same 90 seconds

### Requirement: Added or reopened work keeps the task list open
While the 90-second window runs on a finished list, a task added to the list or a completed task reopened SHALL keep the list open. When every item is completed again, the list SHALL close 90 seconds after that change.

#### Scenario: A new task keeps the list open
- **WHEN** a task is added to a finished list inside the window
- **THEN** the list is not closed and shows the new item

#### Scenario: A reopened task keeps the list open
- **WHEN** a completed task is reopened inside the window
- **THEN** the list is not closed and shows the reopened item as open

#### Scenario: A refinished list gets a fresh window
- **WHEN** every item is completed again after such a change
- **THEN** the list closes 90 seconds after that change, not earlier
