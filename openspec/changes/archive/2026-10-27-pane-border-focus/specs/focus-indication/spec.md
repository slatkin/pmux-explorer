# Spec Delta

## Purpose

Marks the sidebar's active state with a border colour the user can see from anywhere in the tmux window: while the sidebar pane is the active tmux pane, tmux draws that pane's border in a dedicated loud colour, so the user knows the sidebar owns the keyboard before typing into the wrong pane.

## ADDED Requirements

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
