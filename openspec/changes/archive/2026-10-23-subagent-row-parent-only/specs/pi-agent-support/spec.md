# Spec Delta

## ADDED Requirements

### Requirement: a subagent pane shows its parent

The pane row of a subagent SHALL show a line directly under the status row
that names the parent (or `ORPHANED`). The line SHALL NOT carry the fuzzy
task: the task names the pmux launch, and the status row's title slot already
shows the pane name pmux derives from the same launch. The line SHALL be cut
to the row width.

#### Scenario: A parent resolves

- **WHEN** a subagent pane whose parent session id resolves to a live parent pane is listed
- **THEN** the line under the status row shows the parent's pane name and no task text

#### Scenario: No parent resolves

- **WHEN** a subagent pane whose parent session id resolves to no live pane is listed
- **THEN** the line under the status row shows `ORPHANED` and no task text

#### Scenario: A narrow sidebar

- **WHEN** the line is wider than the row
- **THEN** the line is cut to the row width and the row count does not change

## REMOVED Requirements

### Requirement: a subagent pane shows its parent and task

**Reason**: the task was the only name a subagent had before the status row
showed the pane title; now that `status-row` puts the pane name in the title
slot, the task duplicates that name one row below. The stored identity
(`@pane_subagent_task`) stays; only the row stops drawing it.
