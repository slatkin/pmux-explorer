# Spec Delta

## ADDED Requirements

### Requirement: the strip height is shared while the lock is on

While the sidebar lock is on, every docked strip SHALL converge on one
shared docked height, held in `@sidebar_docked_height` as absolute rows.
The lock is the same server-wide lock the width lock uses: toggling it
from any sidebar instance SHALL govern both axes, a column syncing its
width and a strip its height. Enabling the lock from a strip SHALL seed
the shared docked height with the strip's live height, at least the
minimum strip height floor.

#### Scenario: the lock is one flag for both axes
- **WHEN** the user presses `w` while a strip has focus
- **THEN** the same server-wide lock toggles, and strips in every window begin converging on one shared height

#### Scenario: enabling from a strip seeds the shared height
- **WHEN** the user enables the lock while a docked strip is 12 rows high
- **THEN** `@sidebar_docked_height` becomes 12 rows

#### Scenario: enabling from a squeezed strip seeds the floor
- **WHEN** the user enables the lock while a docked strip is 2 rows high
- **THEN** `@sidebar_docked_height` becomes 4 rows, the minimum strip height floor

### Requirement: a docked strip publishes its height while locked

While the lock is on, a docked strip whose pane height changed SHALL
publish the new height as the shared docked height, unless the change was
the strip's own adoption of the shared value. A pane height below the
minimum strip height floor SHALL NOT be published.

#### Scenario: a manual resize becomes the shared height
- **WHEN** the lock is on and the user resizes the docked strip of window A to 14 rows
- **THEN** the shared docked height becomes 14 rows

#### Scenario: a squeeze below the floor is not published
- **WHEN** the lock is on and a horizontal split squeezes the strip below 4 rows
- **THEN** the shared docked height is unchanged

### Requirement: a docked strip adopts the shared height while locked

While the lock is on, a docked strip whose height differs from the shared
docked height SHALL resize toward it, taller or shorter. The adopted
height SHALL be capped at half the window's rows and SHALL never go below
4 rows; where the two cannot both hold, the floor wins. The cap binds
adopted heights only: a publishing strip MAY stay taller than half its
window. Strips in hidden windows SHALL converge within one refresh tick.

#### Scenario: a taller shared height grows the strip
- **WHEN** the lock is on and the shared docked height becomes 14 rows
- **THEN** a docked strip of 10 rows resizes to 14 rows within one refresh tick

#### Scenario: a shorter shared height shrinks the strip
- **WHEN** the lock is on and the shared docked height becomes 8 rows
- **THEN** a docked strip of 14 rows resizes to 8 rows within one refresh tick

#### Scenario: a short window caps the adopted height
- **WHEN** the lock is on, the shared height is 16 rows, and the strip's window is 30 rows high
- **THEN** the strip adopts 15 rows and the shared docked height stays 16 rows

#### Scenario: a shared height below the floor adopts at the floor
- **WHEN** the lock is on, the shared docked height is 2 rows, and the strip's window is 40 rows high
- **THEN** the strip adopts 4 rows

#### Scenario: the publisher's own resize is not capped
- **WHEN** the lock is on and the user resizes the docked strip of a 40-row window to 14 rows
- **THEN** the shared docked height becomes 14 rows, that strip stays at 14 rows, and a peer on a 20-row window adopts 10 rows

#### Scenario: a strip in a hidden window converges
- **WHEN** the lock is on, the shared docked height becomes 14 rows, and the strip's window is not active
- **THEN** the strip adopts 14 rows within one refresh tick without the window becoming active

### Requirement: a floating strip is outside the lock

While the lock is on, a floating strip SHALL keep its panel size, SHALL
NOT publish its height, and SHALL NOT adopt the shared height. Popping it
back in SHALL dock it at the shared docked height instead of the height
it had before it was popped out.

#### Scenario: the floating strip keeps its size
- **WHEN** the lock is on, a strip floats as a panel, and the shared docked height changes
- **THEN** the floating strip keeps its size

#### Scenario: popping back in takes the shared height
- **WHEN** the lock is on with a shared docked height of 14 rows and the user pops a floating strip back in
- **THEN** the strip docks at 14 rows

### Requirement: a new strip starts at the shared height while locked

While the lock is on and the server holds a shared docked height, adding
the sidebar to a window as a strip SHALL create the pane at the shared
docked height instead of resolving `@sidebar_height`. While the lock is
off, `@sidebar_height` SHALL keep governing a new strip as before, a
percent resolving against that window's height.

#### Scenario: a strip opened under the lock
- **WHEN** the lock is on with a shared docked height of 14 rows and the user opens the sidebar in a new window
- **THEN** the new strip pane is 14 rows high

#### Scenario: a strip opened unlocked keeps the option rule
- **WHEN** the lock is off, `@sidebar_height` is `30%`, the window is 40 rows high, and the user opens the sidebar in that window
- **THEN** the new strip pane is 12 rows high
