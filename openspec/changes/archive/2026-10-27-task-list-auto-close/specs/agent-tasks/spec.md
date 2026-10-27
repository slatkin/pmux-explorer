# Spec Delta

## ADDED Requirements

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
