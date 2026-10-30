# Spec Delta

## MODIFIED Requirements

### Requirement: the parent is resolved at each refresh

At each refresh, the sidebar SHALL take as parent the agent pane whose
reported session id equals the recorded parent session id, and SHALL keep
that pane's id. When no pane has that session id, the subagent SHALL be
orphaned. The resolution SHALL run over every live pane, before any filter,
so a parent row that a filter hides still resolves.

#### Scenario: The parent is live and named
- **WHEN** a subagent records parent session id `P1` and pane `%0` reports session id `P1` with the title `orchestrator`
- **THEN** the subagent's parent is pane `%0`

#### Scenario: The parent has no title
- **WHEN** the parent pane `%0` has an empty title
- **THEN** the subagent's parent is still pane `%0`, because the title plays no part in the link

#### Scenario: The parent is gone
- **WHEN** no pane reports session id `P1`
- **THEN** the subagent is orphaned

#### Scenario: The parent comes back
- **WHEN** a pane that reports session id `P1` appears after the subagent was orphaned
- **THEN** the next refresh resolves that pane as the parent

#### Scenario: The parent is hidden by a filter
- **WHEN** the parent pane is live but the sidebar's repository filter hides its row
- **THEN** the subagent's parent is still that pane

## REMOVED Requirements

### Requirement: a subagent pane shows its parent

**Reason**: The line under the status row names the parent in a row of its own. Nesting the subagent under its parent shows the same fact by position and saves that row.
**Migration**: See "a subagent pane is nested under its ultimate parent" for the parent link, and "an orphaned subagent carries a tag" for the `ORPHANED` case.

## ADDED Requirements

### Requirement: a subagent pane is nested under its ultimate parent

A subagent pane SHALL be drawn directly after its ultimate parent, indented by
2 columns, with no row that names the parent. The ultimate parent is the top
of the parent chain that is reachable among the panes of the same repo group.
All depths SHALL use the same indent. Children of one ultimate parent SHALL
keep the order tmux lists them in.

#### Scenario: A subagent nests under its parent
- **WHEN** a repo group holds pane `%0` and a subagent `%1` whose parent is `%0`
- **THEN** `%1` is drawn directly after `%0`, indented by 2 columns, with no parent line

#### Scenario: A subagent of a subagent
- **WHEN** `%2` is a subagent of `%1`, and `%1` is a subagent of `%0`, all in one repo group
- **THEN** `%1` and `%2` are drawn after `%0` with the same indent of 2 columns

#### Scenario: Children keep tmux order
- **WHEN** subagents `%3` and `%1` share the ultimate parent `%0` and tmux lists `%3` before `%1`
- **THEN** `%3` is drawn before `%1`

#### Scenario: Selection follows the drawn order
- **WHEN** the sidebar selection moves down from a parent that has two children
- **THEN** it visits the first child, then the second child, then the next top-level pane

#### Scenario: The parent is in another repo group
- **WHEN** the parent pane is live but belongs to a different repo group than the subagent
- **THEN** the subagent is a top-level row of its own group, with no indent and no tag

#### Scenario: The status filter hides the ultimate parent
- **WHEN** the status filter hides the ultimate parent but shows the subagent
- **THEN** the subagent is a top-level row, with no indent and no tag

#### Scenario: A narrow sidebar
- **WHEN** the sidebar is narrow
- **THEN** every row of the nested subagent is cut to the width left after the indent, and the row count does not change

### Requirement: an orphaned subagent carries a tag

The status row of an orphaned subagent SHALL show the word `ORPHANED` in the
theme's error color, after the subagent badge. An orphaned subagent SHALL be a
top-level row. A subagent that is not nested for any other reason SHALL carry
no tag.

#### Scenario: An orphaned subagent
- **WHEN** a subagent pane whose parent session id resolves to no live pane is listed
- **THEN** its status row shows `ORPHANED` in the error color after the badge, and the pane is a top-level row

#### Scenario: Children of an orphaned subagent
- **WHEN** an orphaned subagent has its own subagents in the same repo group
- **THEN** those subagents are nested under it, and only the orphaned subagent carries the tag

#### Scenario: A resolved subagent
- **WHEN** a subagent pane whose parent resolves to a live pane is listed
- **THEN** its status row shows no `ORPHANED` tag
