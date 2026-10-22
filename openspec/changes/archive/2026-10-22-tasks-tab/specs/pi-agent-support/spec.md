# Spec Delta

## MODIFIED Requirements

### Requirement: the gaps stay gaps

The sidebar SHALL NOT report a permission mode for a pi pane, because pi
reports none. The pi adapter SHALL report the default permission mode. A pi
pane CAN report task progress through the common task command into its own per-pane store, while every
other agent keeps its existing task reporting unchanged. The sidebar SHALL
NOT report an in-process subagent tree for a pi pane; a pmux subagent runs
in its own pane and is marked on that pane instead.

#### Scenario: A pi pane with a finished turn
- **WHEN** a pi turn ends
- **THEN** the pane shows the idle status and no permission mode badge

#### Scenario: The permission mode of a pi pane
- **WHEN** a pi pane is listed
- **THEN** the pane carries no permission mode badge

#### Scenario: A pi pane with per-pane tasks
- **WHEN** a pi pane records tasks through the common task command
- **THEN** the Tasks tab shows that pane's list

#### Scenario: No in-process subagent tree for pi
- **WHEN** a pi pane is listed
- **THEN** the pane shows no in-process subagent rows
