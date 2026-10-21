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
the control and pressing the `p` key SHALL do the same thing.

#### Scenario: the control is at the right edge
- **WHEN** the sidebar is rendered
- **THEN** the top row ends with the pop-out control, right-aligned

#### Scenario: the glyph follows the state
- **WHEN** the sidebar is docked
- **THEN** the control shows the pop-out glyph, and when the sidebar is floating it shows the pop-in glyph

#### Scenario: the keyboard shortcut
- **WHEN** the user presses `p` while the sidebar has focus
- **THEN** the sidebar pops out if it is docked and pops in if it is floating

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
