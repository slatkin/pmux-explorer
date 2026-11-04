# Spec Delta

## MODIFIED Requirements

### Requirement: a strip shows two panels side by side

A strip that does not show the split layout (`split-strip`) SHALL show,
from left to right: the vertical filter bar, the agent panel, the tab
divider, and the tab panel. The requirements of this capability for the
tab panel, its tab bar, its divider and `@sidebar_tab_width` SHALL apply
to such a one-pane strip only; a split strip follows `split-strip`. Each
part except the tab
bar SHALL use the full height of the strip. The agent panel SHALL show the
repo header on its first row and the agent rows below it, and SHALL
scroll vertically as the agent list does in a column. In a strip's
agent list the branch name SHALL ride the pane's status row after the
pane name instead of a dedicated branch row; that row SHALL still
render when it keeps content of its own — listening ports or the
remove `×` of a spawn-created worktree. The pet SHALL NOT show in a
strip.

#### Scenario: the parts of a strip
- **WHEN** a strip 120 columns wide and 12 rows high shows the Tools tab
- **THEN** columns 0-3 are the filter bar, the agent panel follows, then the 1-column divider line, then the tab panel to the right edge, and the tab section's top row is the tab bar

#### Scenario: more agents than rows
- **WHEN** a strip shows more agent rows than its agent panel can hold
- **THEN** the agent panel scrolls to keep the selected agent visible

#### Scenario: the pet is on
- **WHEN** `@sidebar_pet` is `on` and the sidebar is a strip
- **THEN** the strip shows no pet

#### Scenario: a split strip does not read the one-pane tab width
- **WHEN** `@sidebar_tab_width` is 50 and the strip shows the split layout
- **THEN** the panel widths come from the `split-strip` options, and a divider drag does not write `@sidebar_tab_width`

### Requirement: arrow keys follow the strip

In a strip, Left and `h` SHALL move the keyboard focus one part to the
left, and Right and `l` one part to the right, in the order filter bar,
agent panel, tab panel — or, in a split strip, filter bar, agent panel,
pane A, pane B; the focus SHALL stay at an end. Up and Down SHALL
act inside the focused part: in the filter bar they step the category, in
the agent panel they move the selection and stop at the first and last
agent, and in a tab panel they scroll. A column SHALL keep its key map.

#### Scenario: Right from the agent panel
- **WHEN** the sidebar is a strip and the agent panel has focus
- **AND** the user presses Right
- **THEN** the tab panel has focus, or pane A in a split strip

#### Scenario: Down on the last agent
- **WHEN** the sidebar is a strip and the last agent is selected
- **AND** the user presses Down
- **THEN** the last agent stays selected and the agent panel keeps focus

#### Scenario: Down in the filter bar
- **WHEN** the sidebar is a strip, the filter bar has focus and the busy category is selected
- **AND** the user presses Down
- **THEN** the waiting category is selected

#### Scenario: Left at the left end
- **WHEN** the sidebar is a strip and the filter bar has focus
- **AND** the user presses Left
- **THEN** the filter bar keeps focus and the category does not change
