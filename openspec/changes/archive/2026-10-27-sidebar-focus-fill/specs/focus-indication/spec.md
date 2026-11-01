# Spec Delta

## Purpose

Makes the sidebar's focus state visible: when its tmux pane is active the whole sidebar is filled with a base colour, and the section that owns the keyboard is filled with a distinct colour, so a glance shows both that input goes to the sidebar and where it lands.

## ADDED Requirements

### Requirement: the focused sidebar paints a base background fill

When the sidebar pane is the active tmux pane, the whole sidebar SHALL be painted with the base focus background colour: the top section (filter bar, secondary header, agent list, and pet band) and the bottom section (rule row and tab content). The fill SHALL cover every cell of each section, including blank rows below the last pane row.

#### Scenario: the active sidebar is filled
- **WHEN** the sidebar pane is the active tmux pane
- **THEN** every cell of the top section and every cell of the bottom section carries the base focus background colour

#### Scenario: the fill covers blank space
- **WHEN** the sidebar pane is the active tmux pane and the agent list ends before the bottom of the top section
- **THEN** the empty rows below the last pane row also carry the base focus background colour

### Requirement: the unfocused sidebar paints no focus fill

When the sidebar pane is not the active tmux pane, neither section SHALL carry a focus fill. The sidebar SHALL render exactly as it did before this feature.

#### Scenario: another pane is active
- **WHEN** the sidebar pane is not the active tmux pane
- **THEN** no cell in either section carries a focus background colour

### Requirement: the keyboard-owning section gets a distinct fill

Of the two sections, the one that owns keyboard input SHALL be painted with the active focus background colour instead of the base one. Filter and Panes focus SHALL mean the top section owns the keyboard; ActivityLog focus SHALL mean the bottom section owns it.

#### Scenario: the agent list owns the keyboard
- **WHEN** the sidebar pane is active and focus is `Panes` or `Filter`
- **THEN** the top section is painted with the active focus background colour and the bottom section with the base focus background colour

#### Scenario: the bottom panel owns the keyboard
- **WHEN** the sidebar pane is active and focus is `ActivityLog`
- **THEN** the bottom section is painted with the active focus background colour and the top section with the base focus background colour

### Requirement: the selected pane row overrides the fill

The selection background on the selected pane's marker rows SHALL still take precedence over any focus fill, so the selected pane remains the most prominent row.

#### Scenario: a selected row inside a filled section
- **WHEN** the sidebar pane is active, focus is `Panes`, and a pane row is selected
- **THEN** that row's status and branch rows show the selection background, not the focus fill

### Requirement: the two fills are configurable colours

The base and active focus background colours SHALL each be a tmux colour option — `@sidebar_color_focus_bg` and `@sidebar_color_focus_active_bg` — accepting a 256-color index or `#RRGGBB` hex, with defaults quieter than the selection background.

#### Scenario: a custom fill colour
- **WHEN** `@sidebar_color_focus_bg` is set to a valid colour
- **THEN** the base fill uses that colour

#### Scenario: an invalid fill colour
- **WHEN** `@sidebar_color_focus_bg` or `@sidebar_color_focus_active_bg` is unset, empty, or unparseable
- **THEN** the corresponding fill falls back to its default colour
