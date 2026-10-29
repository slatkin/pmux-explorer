# focus-indication Specification

## Purpose

Fills the sidebar's two sections with distinct background colours at all times — the section that owns the keyboard is filled with a distinct colour over the shared base, so a glance shows where input lands. The look does not change with tmux pane focus: the sidebar has no focused/unfocused rendering.

Marks the sidebar's active state with a border colour the user can see from anywhere in the tmux window: while the sidebar pane is the active tmux pane, tmux draws that pane's border in a dedicated loud colour, so the user knows the sidebar owns the keyboard before typing into the wrong pane.

## Requirements

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

### Requirement: the selected pane row overrides the fill

The selection background on the selected pane's marker rows SHALL still take precedence over any focus fill, so the selected pane remains the most prominent row.

#### Scenario: a selected row inside a filled section
- **WHEN** focus is `Panes` and a pane row is selected
- **THEN** that row's status and branch rows show the selection background, not the focus fill

### Requirement: the two fills are configurable colours

The base and active focus background colours SHALL each be a tmux colour option — `@sidebar_color_focus_bg` and `@sidebar_color_focus_active_bg` — accepting a 256-color index or `#RRGGBB` hex, with defaults quieter than the selection background.

#### Scenario: a custom fill colour
- **WHEN** `@sidebar_color_focus_bg` is set to a valid colour
- **THEN** the base fill uses that colour

#### Scenario: an invalid fill colour
- **WHEN** `@sidebar_color_focus_bg` or `@sidebar_color_focus_active_bg` is unset, empty, or unparseable
- **THEN** the corresponding fill falls back to its default colour

### Requirement: the active sidebar pane has a loud border

While the sidebar pane is the active tmux pane, tmux SHALL draw that pane's border in the focus border colour. The signal SHALL be visible regardless of which pane the user is looking at, because it lives in tmux's own border rendering.

#### Scenario: the sidebar takes focus
- **WHEN** the sidebar pane becomes the active tmux pane
- **THEN** the sidebar pane's border is drawn in the focus border colour

#### Scenario: the sidebar loses focus
- **WHEN** another pane in the window becomes the active tmux pane
- **THEN** the sidebar pane's border is drawn with tmux's normal inactive border styling, and the previously active pane's border follows tmux's normal active styling

### Requirement: the focus border colour is configurable

The focus border colour SHALL be a tmux colour option — `@sidebar_color_focus_border` — accepting a 256-color index or `#RRGGBB` hex, with a loud default so the signal reads without configuration.

#### Scenario: a custom border colour
- **WHEN** `@sidebar_color_focus_border` is set to a valid colour
- **THEN** the sidebar pane's active border uses that colour

#### Scenario: an invalid border colour
- **WHEN** `@sidebar_color_focus_border` is unset, empty, or unparseable
- **THEN** the focus border falls back to its default colour

### Requirement: the border write is a one-time pane option

The sidebar SHALL apply the focus border by writing `pane-active-border-style` as a per-pane option on its own pane, and SHALL NOT track focus transitions to update it. A pane option written this way SHALL take effect for as long as the pane lives, without per-tick tmux traffic.

#### Scenario: no per-tick writes
- **WHEN** the sidebar runs and focus moves between panes any number of times
- **THEN** the sidebar writes the pane option once at startup and issues no further border-related tmux commands

#### Scenario: the sidebar process dies
- **WHEN** the sidebar process exits while its border option is set
- **THEN** the pane's options and its border colour end with the pane, leaving no stale styling behind
