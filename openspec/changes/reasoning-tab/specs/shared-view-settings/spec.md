# Spec Delta

## ADDED Requirements

### Requirement: a hidden Thought tab displays as Activity

When the selected bottom tab is Thought and the focused pane does not run pi, the sidebar SHALL display the Activity tab and SHALL NOT write the shared bottom tab value. When the focus returns to a pi pane, the sidebar SHALL display the Thought tab again. The cycle key and a tab click SHALL start from the displayed tab.

#### Scenario: focus moves to a Claude pane
- **WHEN** the Thought tab is selected and the user focuses a Claude pane
- **THEN** the sidebar displays the Activity tab with `Activity` in the accent color
- **AND** `@sidebar_bottom_tab` still holds `thought`

#### Scenario: focus returns to a pi pane
- **WHEN** the Thought tab is selected, the sidebar displays Activity for a Claude pane, and the user focuses a pi pane
- **THEN** the sidebar displays the Thought tab

#### Scenario: another window keeps Thought
- **WHEN** the Thought tab is selected, the sidebar of window A focuses a Claude pane, and the sidebar of window B focuses a pi pane
- **THEN** the sidebar of window B displays the Thought tab

#### Scenario: the cycle key from the fallback
- **WHEN** the Thought tab is selected and the sidebar displays Activity for a Claude pane
- **AND** the user presses the cycle key
- **THEN** the Tasks tab is selected and written to `@sidebar_bottom_tab`

### Requirement: the shared value names the Thought tab

The shared bottom tab value `thought` SHALL select the Thought tab. A value that names no tab SHALL select Activity, as before.

#### Scenario: a peer selects Thought
- **WHEN** another sidebar instance writes `thought` to `@sidebar_bottom_tab`
- **THEN** this instance selects the Thought tab within one refresh tick
