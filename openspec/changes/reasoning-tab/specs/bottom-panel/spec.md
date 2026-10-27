# Spec Delta

## MODIFIED Requirements

### Requirement: the rule holds the tabs

The rule SHALL show the `Activity`, `Tasks` and `Git` tab labels near its
left end, followed by a `Thought` label only while the focused pane runs
pi, with the displayed tab in the accent color. The rule SHALL use
the accent color when the bottom panel has focus, and the inactive border
color when it does not. When the sidebar is too narrow for all labels,
the labels SHALL truncate and the rule SHALL stay exactly as wide as the
sidebar.

#### Scenario: the rule shows the tabs
- **WHEN** the sidebar is 30 columns wide and the Activity tab is selected
- **AND** the focused pane does not run pi
- **THEN** the rule reads `─ Activity│Tasks│Git ` followed by `─` to the right edge
- **AND** `Activity` is in the accent color

#### Scenario: the rule shows the Thought label for a pi pane
- **WHEN** the sidebar is 40 columns wide and the focused pane runs pi
- **THEN** the rule reads `─ Activity│Tasks│Git│Thought ` followed by `─` to the right edge

#### Scenario: the Thought label hides for another agent
- **WHEN** the focused pane runs Claude
- **THEN** the rule shows no `Thought` label

#### Scenario: the rule follows the focus
- **WHEN** the user moves the focus to the bottom panel
- **THEN** the rule changes to the accent color

#### Scenario: a narrow sidebar
- **WHEN** the sidebar is 20 columns wide
- **THEN** the tab labels truncate and the rule is 20 columns wide

### Requirement: a click on the rule selects a tab

When the user presses and releases the left mouse button on the rule
without a move to a different row, the press SHALL be a click. A click on
a tab label SHALL select that tab on release. A click on the rule outside
the tab labels SHALL change nothing.

#### Scenario: a click on a tab label
- **WHEN** the Activity tab is selected
- **AND** the user presses and releases on the `Git` label
- **THEN** the Git tab is selected after the release, and the split height does not change

#### Scenario: a click on the Thought label
- **WHEN** the focused pane runs pi and the Activity tab is selected
- **AND** the user presses and releases on the `Thought` label
- **THEN** the Thought tab is selected after the release

#### Scenario: a click where a hidden label would be
- **WHEN** the focused pane does not run pi
- **AND** the user presses and releases on the `─` fill where the `Thought` label shows for a pi pane
- **THEN** the tab and the split height do not change

#### Scenario: a click on the dashes
- **WHEN** the user presses and releases on the `─` fill to the right of the labels
- **THEN** the tab and the split height do not change

#### Scenario: a click on the left end of the rule
- **WHEN** the Tasks tab is selected
- **AND** the user presses and releases on the `─ ` to the left of the `Activity` label
- **THEN** the Tasks tab stays selected
