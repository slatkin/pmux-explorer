# sidebar-panel Specification

## Purpose
Lets the user take the sidebar out of the window as a floating panel and put
it back, without losing the sidebar or the state it holds, while the sidebar
stays a docked column by default.

## Requirements

### Requirement: the sidebar is docked by default

The sidebar SHALL be a tiled pane in its window, beside the workspace panes,
until the user pops it out. Popping the sidebar out and back in SHALL leave
it docked again.

#### Scenario: a new window opens with a docked sidebar
- **WHEN** a window is created and the sidebar is added to it
- **THEN** the sidebar is a tiled pane of that window and the workspace panes share the remaining width

#### Scenario: the sidebar comes back docked
- **WHEN** the user pops a floating sidebar back in
- **THEN** the sidebar is a tiled pane of the same window again

### Requirement: the top row carries a pop-out control

The sidebar's top row SHALL carry a control at its right edge that pops the
sidebar out when the sidebar is docked and pops it back in when the sidebar
is floating. The control SHALL show a different glyph in each state. Clicking
the control and pressing the tmux prefix followed by `Ctrl+P` while the
sidebar has focus SHALL do the same thing. Bare `p` SHALL NOT toggle the
panel. `Ctrl+P` without the prefix SHALL keep its navigation action.

#### Scenario: the control is at the right edge
- **WHEN** the sidebar is rendered
- **THEN** the top row ends with the pop-out control, right-aligned

#### Scenario: the glyph follows the state
- **WHEN** the sidebar is docked
- **THEN** the control shows the pop-out glyph, and when the sidebar is floating it shows the pop-in glyph

#### Scenario: the keyboard shortcut
- **WHEN** the user presses the tmux prefix followed by `Ctrl+P` while the sidebar has focus
- **THEN** the sidebar pops out if it is docked and pops in if it is floating

#### Scenario: an accidental plain keypress
- **WHEN** the user presses bare `p` while the sidebar has focus
- **THEN** the sidebar stays in its current docked or floating state

#### Scenario: the shortcut outside the sidebar
- **WHEN** the user presses the tmux prefix followed by `Ctrl+P` outside the sidebar
- **THEN** tmux switches to the previous window

### Requirement: popping out floats the sidebar as a panel

Popping the sidebar out SHALL turn the sidebar pane into a floating pane in
the same window, 40 columns wide, as tall as the window, and anchored to the
right edge. The workspace panes SHALL take the full width of the window
again, because a floating pane does not take space from the layout. The
floating panel SHALL keep that width and that anchor when the window is
resized.

#### Scenario: the panel floats over the workspace
- **WHEN** the user pops the sidebar out
- **THEN** the sidebar pane is floating, 40 columns wide, as tall as the window, at the right edge, and the workspace panes use the full window width

#### Scenario: the panel keeps its own content
- **WHEN** the user pops the sidebar out
- **THEN** the same sidebar process keeps running with the same filter, selection, scroll position and bottom tab as before

#### Scenario: the window is resized while the panel floats
- **WHEN** the window is resized while the sidebar floats
- **THEN** the panel is 40 columns wide again and its right edge is flush with the right edge of the window

### Requirement: popping back in restores the docked width

Popping the sidebar back in SHALL return it to the layout and SHALL restore
the width the sidebar had before it was popped out, so that a sidebar the
user resized by hand keeps that width across the round trip.

#### Scenario: the docked width is restored
- **WHEN** the user pops a 25 column sidebar out and pops it back in
- **THEN** the sidebar is a tiled pane of 25 columns again

### Requirement: the floating panel keeps the agent list readable

In the floating panel the Activity/Git bottom panel SHALL take at most a
third of the panel height, so the agent list keeps the rest.

#### Scenario: the bottom panel is clamped
- **WHEN** the sidebar is a floating panel of 30 rows
- **THEN** the Activity/Git bottom panel is at most 10 rows tall

### Requirement: the control follows the pane, not a remembered flag

The control SHALL show the state of the sidebar pane itself, so that a
sidebar that was floated from the tmux command line shows the pop-in glyph
and can be popped back in.

#### Scenario: the pane was floated by hand
- **WHEN** the sidebar pane is floating and the user looks at the top row
- **THEN** the control shows the pop-in glyph

### Requirement: a refused pop-out is reported

When tmux refuses to float the sidebar pane, the sidebar SHALL tell the user
that the pop-out did not happen, and SHALL stay docked.

#### Scenario: the window is zoomed
- **WHEN** the user pops the sidebar out while the window is zoomed
- **THEN** the sidebar stays docked and the user is told that the pop-out was refused

### Requirement: the top row carries a side-switch control

The sidebar's top row SHALL carry a side-switch control next to the pop-out control. Clicking it, and pressing the tmux prefix followed by `Ctrl+O` while the sidebar has focus, SHALL move the docked sidebar to the opposite edge of the window.

#### Scenario: the control sits next to the pop-out control
- **WHEN** the sidebar is rendered
- **THEN** the top row shows the side-switch control directly left of the pop-out control

#### Scenario: the control moves a left sidebar to the right
- **WHEN** the sidebar is docked on the left and the user clicks the control
- **THEN** the sidebar is a full-height tiled pane at the right edge and the workspace panes keep their order

#### Scenario: the control moves a right sidebar to the left
- **WHEN** the sidebar is docked on the right and the user presses the tmux prefix followed by `Ctrl+O`
- **THEN** the sidebar is a full-height tiled pane at the left edge

#### Scenario: the shortcut outside the sidebar
- **WHEN** the user presses the tmux prefix followed by `Ctrl+O` outside the sidebar
- **THEN** tmux rotates the window as it does by default

### Requirement: the side switch applies to every window

The side switch SHALL move the sidebar of every window to the new side, and SHALL set the global `@sidebar_position` to that side so sidebars created later follow. It SHALL NOT change the user's tmux config.

#### Scenario: all windows follow
- **WHEN** a session has three windows with docked left sidebars and the user switches the side
- **THEN** all three sidebars are docked at the right edge

#### Scenario: a new window follows the switch
- **WHEN** the user switches the side to right and then opens a new window
- **THEN** the new window's sidebar is created on the right

### Requirement: windows that cannot be moved are skipped

The side switch SHALL skip a window that tmux refuses to rearrange, such as a zoomed window, and a window whose sidebar is floating. A skipped window SHALL keep its side. The switch SHALL still move the other windows.

#### Scenario: a zoomed window
- **WHEN** one of four windows is zoomed and the user switches the side
- **THEN** the other three sidebars move and the zoomed window keeps its sidebar on the old side

#### Scenario: a floating sidebar in another window
- **WHEN** the sidebar of one window is floating and the user switches the side from another window
- **THEN** the floating sidebar is left as it is and the docked sidebars move

### Requirement: the side switch reports its outcome

After a switch that moved or skipped at least one window, the sidebar SHALL tell the user how many windows moved and how many were skipped.

#### Scenario: some windows skipped
- **WHEN** the switch moves three windows and skips one
- **THEN** the sidebar shows a notice that three windows moved and one was skipped

#### Scenario: every window moved
- **WHEN** the switch moves all windows and skips none
- **THEN** the sidebar shows a notice that the windows moved, with no skip count

### Requirement: the side switch is unavailable while floating

While the sidebar is floating, the side-switch control SHALL be drawn disabled and SHALL NOT act. The floating panel SHALL keep its right-edge anchor.

#### Scenario: clicking the disabled control
- **WHEN** the sidebar is floating and the user clicks the side-switch control
- **THEN** nothing moves and the global side does not change

### Requirement: the rule draws the tabs as centered icons

The bottom panel's rule SHALL draw each shown tab as a single Nerd Font glyph — Tools , Tasks , Git 󰊢, Thought 󰟶 — instead of a text label, and SHALL center the tab run on the rule. Clicking a drawn glyph SHALL select its tab, and the click hitboxes SHALL be the drawn columns.

#### Scenario: a glyph per tab
- **WHEN** the bottom panel's rule renders
- **THEN** each shown tab draws its glyph and no tab draws a text label

#### Scenario: the run centers on the rule
- **WHEN** the rule is `W` columns wide and the tab run is `R` columns
- **THEN** the run starts at column `floor((W - R) / 2)` and the rule still spans exactly `W` columns

#### Scenario: a click on a glyph selects its tab
- **WHEN** the user clicks a drawn glyph
- **THEN** the rule selects that tab
