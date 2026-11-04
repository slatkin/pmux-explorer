# Spec Delta

## ADDED Requirements

### Requirement: the split strip settings are shared

All sidebar instances on one tmux server SHALL share `@sidebar_split_tab_a`,
`@sidebar_split_tab_b`, `@sidebar_split_agents_width` and
`@sidebar_split_tab_b_width`. A change to one of them in one split strip
SHALL apply to every other split strip on the same server within one
refresh tick, with the same failed-save and drag-in-progress rules as the
other shared settings.

#### Scenario: a pane B tab change applies to another window
- **WHEN** window A and window B each have a split strip
- **AND** the user selects the Thought tab in pane B of window A
- **THEN** pane B of window B shows the Thought tab when its focused pane runs pi

#### Scenario: a divider drag applies to another window
- **WHEN** the user drags divider 1 in the split strip of window A until `@sidebar_split_agents_width` is 25
- **THEN** the split strip of window B shows an agent panel 25 percent wide

### Requirement: a split strip and a one-pane sidebar keep their own settings

A tab selection or a divider drag in a split strip SHALL NOT change
`@sidebar_bottom_tab`, `@sidebar_tab_width` or `@sidebar_bottom_height`.
A tab selection or a drag in a column or a one-pane strip SHALL NOT change
the split strip settings.

#### Scenario: a split selection leaves a column alone
- **WHEN** window A has a split strip and window B has a column that shows the Tools tab
- **AND** the user selects Git in pane A of window A
- **THEN** the column of window B still shows the Tools tab

#### Scenario: a column selection leaves a split strip alone
- **WHEN** window A has a column and window B has a split strip whose pane B shows Tasks
- **AND** the user selects the Git tab in window A
- **THEN** pane A and pane B of window B do not change
