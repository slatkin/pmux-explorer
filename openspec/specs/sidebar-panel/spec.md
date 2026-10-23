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

Popping the sidebar back in SHALL return it to the layout and, while the
width lock is off, SHALL restore the width the sidebar had before it was
popped out, so that a sidebar the user resized by hand keeps that width
across the round trip.

#### Scenario: the docked width is restored
- **WHEN** the width lock is off and the user pops a 25 column sidebar out and pops it back in
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

### Requirement: the top row carries a lock control

The sidebar's top row SHALL carry a control beside the pop-out control
that shows whether the width lock is on. Clicking the control and pressing
the `w` key SHALL each toggle the lock for every sidebar instance on the
server.

#### Scenario: the control shows the lock state
- **WHEN** the sidebar is rendered
- **THEN** the top row shows the lock control beside the pop-out control, with a different glyph for locked and unlocked

#### Scenario: the keyboard shortcut
- **WHEN** the user presses `w` while the sidebar has focus
- **THEN** the width lock toggles on if it was off and off if it was on

#### Scenario: the control is clickable
- **WHEN** the user clicks the lock control
- **THEN** the width lock toggles the same way as pressing `w`

### Requirement: a docked sidebar publishes its width while locked

While the width lock is on, a docked sidebar whose pane width changed
SHALL publish the new width as the shared docked width, unless the change
was the sidebar's own adoption of the shared value. A pane width below
the minimum-width floor SHALL NOT be published.

#### Scenario: a manual resize becomes the shared width
- **WHEN** the lock is on and the user resizes the docked sidebar of window A to 45 columns
- **THEN** the shared docked width becomes 45 columns

#### Scenario: a squeeze below the floor is not published
- **WHEN** the lock is on and a split in the window squeezes the sidebar below the minimum-width floor
- **THEN** the shared docked width is unchanged

### Requirement: a docked sidebar adopts the shared width while locked

While the width lock is on, a docked sidebar whose width differs from the
shared docked width SHALL resize toward it, wider or narrower. The
adopted width SHALL be capped at half the window's columns and SHALL
never go below the minimum-width floor; where the two cannot both hold,
the floor wins. The cap binds adopted widths only: a publishing sidebar
MAY stay wider than half its window.

#### Scenario: a wider shared width grows the pane
- **WHEN** the lock is on and the shared docked width becomes 45 columns
- **THEN** a docked sidebar of 30 columns resizes to 45 columns within one refresh tick

#### Scenario: a narrower shared width shrinks the pane
- **WHEN** the lock is on and the shared docked width becomes 25 columns
- **THEN** a docked sidebar of 45 columns resizes to 25 columns within one refresh tick

#### Scenario: a narrow window caps the adopted width
- **WHEN** the lock is on, the shared width is 80 columns, and the sidebar's window is 100 columns wide
- **THEN** the sidebar adopts 50 columns and the shared docked width stays 80 columns

#### Scenario: a shared width below the floor adopts at the floor
- **WHEN** the lock is on, the shared docked width is 8 columns, and the minimum-width floor is 20
- **THEN** the sidebar adopts 20 columns

#### Scenario: a squeezed sidebar grows back toward the shared width
- **WHEN** the lock is on and a new pane squeezes the sidebar below the shared width
- **THEN** the sidebar grows back toward the shared width within one refresh tick

#### Scenario: the publisher's own resize is not capped
- **WHEN** the lock is on and the user resizes the docked sidebar of a 100-column window to 70 columns
- **THEN** the shared docked width becomes 70 columns, that sidebar stays at 70 columns, and a peer on a 100-column window adopts 50 columns

#### Scenario: a squeeze with no shared width still recovers
- **WHEN** the lock is on, no shared docked width exists, and a split squeezes the sidebar below the minimum-width floor on the active window
- **THEN** the sidebar grows back to the floor the same way as an unlocked sidebar

### Requirement: a floating sidebar is outside the width lock

While the width lock is on, a floating sidebar SHALL keep its fixed panel
width, SHALL NOT publish its width, and SHALL NOT adopt the shared width.
Popping it back in SHALL restore the shared docked width.

#### Scenario: the floating panel keeps its width
- **WHEN** the lock is on and the sidebar floats as a panel
- **THEN** the panel stays 40 columns wide and does not change when the shared docked width changes

#### Scenario: popping back in restores the shared width
- **WHEN** the lock is on with a shared docked width of 45 columns and the user pops a floating sidebar back in
- **THEN** the sidebar docks at 45 columns

### Requirement: a new sidebar starts at the shared width while locked

While the width lock is on and the server holds a shared docked width,
adding the sidebar to a window SHALL create the pane at the shared width
instead of the configured sidebar width.

#### Scenario: a sidebar opened under the lock
- **WHEN** the lock is on with a shared docked width of 45 columns and the user opens the sidebar in a new window
- **THEN** the new sidebar pane is 45 columns wide
