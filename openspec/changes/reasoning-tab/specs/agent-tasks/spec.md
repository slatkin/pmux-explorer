# Spec Delta

## MODIFIED Requirements

### Requirement: The tab cycles with the other tabs
The bottom-tab cycle SHALL cover the Activity tab (drawn `Tools`), then `Thought` while the focused pane runs pi, then `Tasks` and `Git`, in that order. The cycle SHALL skip Thought while its label is hidden. The existing toggle key and tab-header clicks SHALL reach every shown tab. Scroll input SHALL move the visible tab's own scroll offset.

#### Scenario: Keyboard cycles three tabs
- **WHEN** the focused pane does not run pi
- **AND** the user toggles the bottom tab repeatedly
- **THEN** the tab visits Tools, Tasks, and Git in order

#### Scenario: Keyboard cycles four tabs for a pi pane
- **WHEN** the focused pane runs pi
- **AND** the user toggles the bottom tab repeatedly
- **THEN** the tab visits Tools, Thought, Tasks, and Git in order

#### Scenario: Click reaches Tasks
- **WHEN** the user clicks the Tasks title in the tab header
- **THEN** the bottom panel shows the Tasks tab

#### Scenario: Scroll stays per tab
- **WHEN** the user scrolls in the Tasks tab and switches away and back
- **THEN** the Tasks scroll offset is kept and the other tabs keep theirs
