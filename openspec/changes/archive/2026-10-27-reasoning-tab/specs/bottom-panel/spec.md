# Spec Delta

## MODIFIED Requirements

### Requirement: the rule holds the tabs

The rule SHALL show the `Tools` label first, then `Thought` while the
focused pane runs pi, then `Tasks` and `Git`, with the displayed tab in
the accent color. The tabs keep their code names — the `Tools` label is
the `Activity` variant, and the shared value stays `activity` — so only
the drawn label changed. The rule SHALL use the accent color when the
bottom panel has focus, and the inactive border color when it does not.
When the sidebar is too narrow for all labels, the labels SHALL truncate
and the rule SHALL stay exactly as wide as the sidebar.

#### Scenario: the rule shows the tabs
- **WHEN** the sidebar is 30 columns wide and the Activity tab is selected
- **AND** the focused pane does not run pi
- **THEN** the rule reads `─ Tools│Tasks│Git ` followed by `─` to the right edge
- **AND** `Tools` is in the accent color

#### Scenario: the rule shows the Thought label for a pi pane
- **WHEN** the sidebar is 40 columns wide and the focused pane runs pi
- **THEN** the rule reads `─ Tools│Thought│Tasks│Git ` followed by `─` to the right edge

#### Scenario: the labels sit at their columns
- **WHEN** the focused pane does not run pi
- **THEN** `─ ` occupies columns 0-1, `Tools` 2-6, `Tasks` 8-12 and `Git` 14-16
- **AND** one separator column sits between two labels

#### Scenario: the pi labels sit at their columns
- **WHEN** the focused pane runs pi
- **THEN** `─ ` occupies columns 0-1, `Tools` 2-6, `Thought` 8-14, `Tasks` 16-20 and `Git` 22-24

#### Scenario: the Thought label hides for another agent
- **WHEN** the focused pane runs Claude
- **THEN** the rule shows no `Thought` label, and `Tasks` and `Git` follow `Tools` directly

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
the tab labels SHALL change nothing. The hitboxes SHALL come from the same
shown-tab list and label strings the rule draws, so a hidden label has no
hitbox.

#### Scenario: a click on a tab label
- **WHEN** the Activity tab is selected
- **AND** the user presses and releases on the `Git` label (columns 14-16 for a pane that does not run pi)
- **THEN** the Git tab is selected after the release, and the split height does not change

#### Scenario: a click on the Tools label
- **WHEN** the Tasks tab is selected
- **AND** the user presses and releases on the `Tools` label (columns 2-6)
- **THEN** the Activity tab is selected after the release

#### Scenario: a click on the Tasks label
- **WHEN** the focused pane does not run pi
- **AND** the Activity tab is selected
- **AND** the user presses and releases on the `Tasks` label (columns 8-12, where `Thought` sits for a pi pane)
- **THEN** the Tasks tab is selected after the release

#### Scenario: a click on the Thought label
- **WHEN** the focused pane runs pi and the Activity tab is selected
- **AND** the user presses and releases on the `Thought` label (columns 8-14)
- **THEN** the Thought tab is selected after the release

#### Scenario: a non-pi pane has no Thought hitbox
- **WHEN** the focused pane does not run pi and the Tasks tab is selected
- **AND** the user presses and releases on columns 8-14, where the `Thought` label sits for a pi pane
- **THEN** the click lands on the `Tasks` label and the tab and the split height do not change

#### Scenario: a click on the dashes
- **WHEN** the user presses and releases on the `─` fill to the right of the labels (columns 18 and later for a pane that does not run pi)
- **THEN** the tab and the split height do not change

#### Scenario: a click on the left end of the rule
- **WHEN** the Tasks tab is selected
- **AND** the user presses and releases on the `─ ` to the left of the `Tools` label
- **THEN** the Tasks tab stays selected
