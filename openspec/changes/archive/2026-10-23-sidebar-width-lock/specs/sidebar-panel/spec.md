# Spec Delta

## MODIFIED Requirements

### Requirement: popping back in restores the docked width

Popping the sidebar back in SHALL return it to the layout and, while the
width lock is off, SHALL restore the width the sidebar had before it was
popped out, so that a sidebar the user resized by hand keeps that width
across the round trip.

#### Scenario: the docked width is restored
- **WHEN** the width lock is off and the user pops a 25 column sidebar out and pops it back in
- **THEN** the sidebar is a tiled pane of 25 columns again

## ADDED Requirements

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
