# Spec Delta

## MODIFIED Requirements

### Requirement: the sidebar paints a base background fill at all times

The whole sidebar SHALL be painted with the base focus background colour at all times — whether its tmux pane is the active pane or not: the agent section (filter bar, secondary header, agent list, and pet band) and the tab section (rule row or strip divider, and tab content). In a column the agent section is on top and the tab section below; in a strip the agent section is on the left and the tab section on the right. The fill SHALL cover every cell of each section, including blank rows below the last pane row. Whether the sidebar pane is the active tmux pane SHALL NOT change any cell's background.

#### Scenario: the sidebar is filled
- **WHEN** the sidebar renders
- **THEN** every cell of the agent section and every cell of the tab section carries the base focus background colour

#### Scenario: the fill covers blank space
- **WHEN** the agent list ends before the bottom of the agent section
- **THEN** the empty rows below the last pane row also carry the base focus background colour

#### Scenario: the unfocused sidebar looks the same
- **WHEN** the sidebar pane is not the active tmux pane
- **THEN** the sections carry the same fills they carry while the pane is active

### Requirement: the keyboard-owning section gets a distinct fill

Of the two sections, the one that owns keyboard input SHALL be painted with the active focus background colour instead of the base one. Filter and Panes focus SHALL mean the agent section owns the keyboard; ActivityLog focus SHALL mean the tab section owns it.

#### Scenario: the agent list owns the keyboard
- **WHEN** focus is `Panes` or `Filter`
- **THEN** the agent section is painted with the active focus background colour and the tab section with the base focus background colour

#### Scenario: the bottom panel owns the keyboard
- **WHEN** focus is `ActivityLog`
- **THEN** the tab section is painted with the active focus background colour and the agent section with the base focus background colour

#### Scenario: the tab panel of a strip owns the keyboard
- **WHEN** the sidebar is a strip and focus is `ActivityLog`
- **THEN** the divider and the tab panel on the right are painted with the active focus background colour, and the filter bar and the agent panel on the left with the base focus background colour
