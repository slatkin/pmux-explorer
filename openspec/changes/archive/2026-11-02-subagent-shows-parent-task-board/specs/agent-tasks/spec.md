## MODIFIED Requirements

### Requirement: The Tasks tab shows the focused pane todo list

The bottom panel SHALL carry a third tab named Tasks beside Activity and Git. The Tasks tab SHALL show the todo list of the focused pane, except that a pmux subagent pane with a live parent SHALL show the todo list of its ultimate parent pane. The selected tab SHALL be shared with the other sidebar instances on the same tmux server, and focusing a different pane SHALL NOT change the selected tab.

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

#### Scenario: A subagent shows its parent's list
- **WHEN** the focused pane is a pmux subagent whose parent pane is live
- **AND** the parent pane has recorded tasks
- **THEN** the Tasks tab lists the parent pane's items and description

#### Scenario: A subagent shows the list of the ultimate parent
- **WHEN** the focused pane is a pmux subagent whose parent is itself a pmux subagent with a live parent
- **THEN** the Tasks tab shows the list of the top pane of that parent chain

#### Scenario: The parent need not be nested in the sidebar
- **WHEN** the focused pane is a pmux subagent whose live parent pane is in another repo group or is hidden by the status filter
- **THEN** the Tasks tab still shows the parent pane's list

#### Scenario: An orphaned subagent shows its own list
- **WHEN** the focused pane is a pmux subagent that has no live parent pane
- **THEN** the Tasks tab shows the focused pane's own list

#### Scenario: A subagent's own writes stay in its own list
- **WHEN** a pmux subagent runs the task command in its own pane
- **THEN** the item is recorded in that pane's own list
- **AND** the parent pane's list does not change

#### Scenario: A parent chain that loops ends
- **WHEN** the parent links of the focused pane form a loop
- **THEN** the Tasks tab shows the list of one pane of that loop and does not stall
