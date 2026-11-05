# Spec Delta

## MODIFIED Requirements

### Requirement: popping back in restores the docked width

Popping the sidebar back in SHALL return it to the layout at the edge where
it was docked and, while the width lock is off, SHALL restore the size the
sidebar had before it was popped out: the width of a column and the height
of a strip. A sidebar the user resized by hand SHALL keep that size across
the round trip. While the width lock is on and the server holds a shared
docked size, a column pop-in SHALL take the shared docked width and a strip
pop-in the shared docked height instead.

#### Scenario: the docked width is restored
- **WHEN** the width lock is off and the user pops a 25 column sidebar out and pops it back in
- **THEN** the sidebar is a tiled pane of 25 columns again

#### Scenario: the docked height is restored
- **WHEN** the width lock is off and the user pops out a bottom strip of 10 rows and pops it back in
- **THEN** the sidebar is a tiled strip of 10 rows at the bottom edge, as wide as the window
