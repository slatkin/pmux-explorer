# shared-view-settings Specification

## Purpose
Defines which sidebar view settings all sidebar instances on one tmux server
share, and how fast an instance shows a change made in another instance.

## Requirements

### Requirement: the view settings are shared

All sidebar instances on one tmux server SHALL share the status filter, the
repo filter, the cursor's agent pane in the agent list, the selected bottom
tab, the bottom panel height, the strip tab width, and — while the width
lock is on — the sidebar's docked width. A change to one of these settings
in one instance SHALL apply to every other instance on the same tmux
server.

#### Scenario: a filter change applies to another window
- **WHEN** the user selects the waiting status filter in the sidebar of window A
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the waiting status filter

#### Scenario: a repo filter change applies to another window
- **WHEN** the user selects a repo in the repo filter of the sidebar of window A
- **THEN** the sidebar of window B shows the same repo filter

#### Scenario: a cursor move applies to another window
- **WHEN** the user moves the cursor to the third agent row in the sidebar of window A
- **THEN** the sidebar of window B puts its cursor on the same agent pane, at whatever row that pane occupies in window B's own list

#### Scenario: a bottom tab change applies to another window
- **WHEN** the user switches the bottom panel of the sidebar of window A to the Tasks tab
- **AND** the user then goes to window B in the same session
- **THEN** the sidebar of window B shows the Tasks tab

#### Scenario: a docked-width change applies to another window
- **WHEN** the width lock is on and the user resizes the docked sidebar of window A
- **THEN** the docked sidebar of window B shows the same width within one refresh tick

#### Scenario: a strip tab width change applies to another window
- **WHEN** the user drags the divider in the strip of window A so that the tab panel is 50 percent wide
- **AND** the user then goes to window B, which also has a strip
- **THEN** the strip of window B shows a tab panel that is 50 percent wide

### Requirement: a shared change shows within one refresh tick

A sidebar instance SHALL show a shared setting changed by another instance
within one refresh tick (1 second) of the change. This SHALL apply whether
the window of the instance is visible or not, and whether the instance is in
the same tmux session or in a different one.

#### Scenario: an instance in a different session
- **WHEN** the user changes the status filter in a sidebar in session A
- **AND** the user switches the client to session B
- **THEN** the sidebar in session B shows the new status filter within 1 second

#### Scenario: a fast window switch
- **WHEN** the user goes from window A to window B, changes the status filter there, and goes back to window A in less than 2 seconds
- **THEN** the sidebar of window A shows the new status filter within 1 second

#### Scenario: an instance that the user does not look at
- **WHEN** the user changes the status filter in window A
- **THEN** the sidebar of window B shows the new status filter within 1 second, also while window A stays the visible window

### Requirement: a local change is not reverted by a failed save

When an instance cannot write a changed shared setting to tmux, the instance
SHALL keep the value the user selected. The next refresh tick SHALL NOT
replace it with the older value that tmux still holds.

#### Scenario: the tmux write fails
- **WHEN** the user changes the status filter from error to running
- **AND** the write of the new filter to tmux fails
- **THEN** the sidebar continues to show the running filter after the next refresh tick

### Requirement: the shared cursor names a pane, not a row

The cursor SHALL be shared as the tmux pane it names, not as its row
number, because each instance's list holds its own rows for the same
panes. On every refresh tick an instance SHALL resolve the cursor's row
from the shared pane id, and SHALL NOT keep naming a pane other than the
one on its row.

#### Scenario: the peer's list differs from this instance's
- **WHEN** the cursor in the sidebar of window A is on pane `%7`, which is row 5 there
- **AND** the sidebar of window B lists that pane at row 2
- **THEN** the cursor in window B is on row 2

#### Scenario: a filter change keeps the cursor on its pane
- **WHEN** the cursor is on pane `%7`
- **AND** the user changes the status filter so that two panes above `%7` are hidden
- **THEN** the cursor moves up two rows and stays on pane `%7`

#### Scenario: the cursor's pane is not listed
- **WHEN** the cursor is on pane `%7`
- **AND** the instance's list does not hold `%7`, because a filter hides it or it has closed
- **THEN** the cursor moves to the tmux-focused pane's row when that pane is listed, and otherwise stays on its own clamped row

#### Scenario: a value that names no pane
- **WHEN** the shared cursor option holds something other than a pane id, such as a bare row number written by an older version
- **THEN** the instance ignores it and resolves its cursor from the tmux-focused pane

### Requirement: a pending cursor move is not reverted

While a cursor move in an instance waits for its delayed write to tmux, the
next refresh tick SHALL NOT move the cursor back to the older value that
tmux still holds.

#### Scenario: the user moves the cursor quickly
- **WHEN** the user moves the cursor down two rows
- **AND** a refresh tick runs before the cursor write reaches tmux
- **THEN** the cursor stays on the pane the user selected

### Requirement: the rest of the view state stays local

Each sidebar instance SHALL keep its own focus, scroll position, open
popups and git data, and — while the width lock is off — its own docked
width. A change to these in one instance SHALL NOT change them in another
instance.

#### Scenario: a scroll change stays local
- **WHEN** the user scrolls the activity log down in the sidebar of window A
- **THEN** the sidebar of window B keeps its own activity log scroll position

#### Scenario: a width change stays local while unlocked
- **WHEN** the width lock is off and the user resizes the docked sidebar of window A
- **THEN** the sidebar of window B keeps its own width

### Requirement: focusing a different agent does not change the bottom tab

The bottom tab SHALL change only when the user selects a tab — with the
cycle key, with a click on a tab title, or in another sidebar instance
whose selection this instance adopts. Focusing a different pane or a
different agent SHALL leave the selected tab alone.

#### Scenario: clicking a different agent keeps the tab
- **WHEN** the sidebar shows the Tasks tab
- **AND** the user focuses another agent pane in the same window
- **THEN** the sidebar still shows the Tasks tab

#### Scenario: an agent starting in the focused pane keeps the tab
- **WHEN** the sidebar shows the Git tab
- **AND** an agent starts in the currently focused pane
- **THEN** the sidebar still shows the Git tab

#### Scenario: a shell pane keeps the tab
- **WHEN** the sidebar shows the Tasks tab
- **AND** the user focuses a pane with no agent running
- **THEN** the sidebar still shows the Tasks tab

### Requirement: an instance without a shared tab shows Activity

A sidebar instance SHALL show the Activity tab when the tmux server holds
no bottom tab value — on a server where no instance has selected a tab yet.

#### Scenario: a first instance on a fresh server
- **WHEN** the tmux server holds no bottom tab value
- **AND** a sidebar instance starts
- **THEN** it shows the Activity tab

### Requirement: the width lock is shared

All sidebar instances on one tmux server SHALL share one width-lock state,
on or off. Toggling the lock in one instance SHALL apply to every other
instance on the same server within one refresh tick. A tmux server that
holds no width-lock value SHALL behave as unlocked.

#### Scenario: locking in one window locks all windows
- **WHEN** the user enables the width lock in the sidebar of window A
- **THEN** the sidebar of window B treats the lock as on within one refresh tick

#### Scenario: a first instance on a fresh server
- **WHEN** the tmux server holds no width-lock value
- **AND** a sidebar instance starts
- **THEN** it runs with the lock off

### Requirement: enabling the lock seeds the shared width

When the width lock goes from off to on, the shared docked width SHALL
take the enabling instance's docked width at that moment, clamped at the
minimum-width floor. An instance that is floating when it enables the
lock SHALL seed the docked width it captured on the way out, or the
floating panel's width when no docked width was captured.

#### Scenario: the enabling instance's width becomes the shared width
- **WHEN** the user enables the lock on a docked sidebar of 30 columns
- **THEN** the shared docked width is 30 columns and every other docked
  sidebar on the server adopts it within one refresh tick

#### Scenario: enabling while floating seeds the captured width
- **WHEN** the user pops a 25 column sidebar out and then enables the lock
- **THEN** the shared docked width is 25 columns and every docked sidebar
  on the server adopts it within one refresh tick

### Requirement: unlocking keeps every sidebar at its current width

When the width lock goes from on to off, each sidebar SHALL keep the width
it has at that moment. A later width change in one instance SHALL NOT
change another instance's width.

#### Scenario: a resize stays local after unlocking
- **WHEN** the lock is turned off and the user later resizes the docked sidebar of window A
- **THEN** the sidebar of window B keeps the width it had

### Requirement: a failed width publish is not reverted

When an instance cannot write its docked width to the shared option, the
next refresh tick SHALL NOT resize its pane back to the stale shared
value, and the write SHALL be retried.

#### Scenario: the tmux write fails
- **WHEN** the lock is on, the user resizes the docked sidebar to 45 columns, and the write of the new shared width to tmux fails
- **THEN** the sidebar stays at 45 columns after the next refresh tick and the write is retried until it lands

### Requirement: the split height is shared

All sidebar instances on one tmux server SHALL share the bottom panel
height (`@sidebar_bottom_height`). A height that the user sets with a
drag in one instance SHALL apply to every other instance on the same
tmux server, within the refresh time of the other shared settings.

#### Scenario: a drag applies to another window
- **WHEN** the user drags the rule in the sidebar of window A so that the bottom panel is 50 percent high
- **AND** the user then goes to window B
- **THEN** the sidebar of window B shows a bottom panel that is 50 percent high

#### Scenario: a different sidebar height
- **WHEN** the sidebar of window A is 40 rows high and the sidebar of window B is 60 rows high
- **AND** the user sets 50 percent with a drag in window A
- **THEN** the bottom panel is 20 rows high in window A and 30 rows high in window B

### Requirement: a drag in progress is not reverted

While the user drags the rule of a column or the divider of a strip, a
refresh tick SHALL NOT move the rule or the divider to the size that tmux
holds. After release, the instance SHALL adopt the shared size again on the
refresh ticks that follow.

#### Scenario: a refresh tick during a drag
- **WHEN** the user presses on the rule and drags it for 3 seconds
- **THEN** the rule stays on the mouse row through every refresh tick of the drag

#### Scenario: a refresh tick during a divider drag
- **WHEN** the user presses on the divider of a strip and drags it for 3 seconds
- **THEN** the divider stays on the mouse column through every refresh tick of the drag

#### Scenario: another instance changes the height after release
- **WHEN** the user released a drag in window A
- **AND** a later drag in window B sets a different height
- **THEN** the sidebar of window A shows the height from window B within 1 second

### Requirement: a hidden Thought tab displays as Activity

When the selected bottom tab is Thought and the focused pane does not run pi, the sidebar SHALL display the Activity tab — drawn as `Tools` — and SHALL NOT write the shared bottom tab value. When the focus returns to a pi pane, the sidebar SHALL display the Thought tab again. The cycle key and a tab click SHALL start from the displayed tab.

#### Scenario: focus moves to a Claude pane
- **WHEN** the Thought tab is selected and the user focuses a Claude pane
- **THEN** the sidebar displays the Activity tab, whose label reads `Tools` in the accent color
- **AND** `@sidebar_bottom_tab` still holds `thought`

#### Scenario: focus returns to a pi pane
- **WHEN** the Thought tab is selected, the sidebar displays Activity for a Claude pane, and the user focuses a pi pane
- **THEN** the sidebar displays the Thought tab

#### Scenario: another window keeps Thought
- **WHEN** the Thought tab is selected, the sidebar of window A focuses a Claude pane, and the sidebar of window B focuses a pi pane
- **THEN** the sidebar of window B displays the Thought tab

#### Scenario: the cycle key from the fallback
- **WHEN** the Thought tab is selected and the sidebar displays Activity (the `Tools` label) for a Claude pane
- **AND** the user presses the cycle key
- **THEN** the Tasks tab is selected and written to `@sidebar_bottom_tab`

### Requirement: the shared value names the Thought tab

The shared bottom tab value `thought` SHALL select the Thought tab. A value that names no tab SHALL select Activity, drawn as `Tools`, as before.

#### Scenario: a peer selects Thought
- **WHEN** another sidebar instance writes `thought` to `@sidebar_bottom_tab`
- **THEN** this instance selects the Thought tab within one refresh tick

### Requirement: the strip tab width is shared apart from the column height

All strips on one tmux server SHALL share `@sidebar_tab_width`. A divider
drag in a strip SHALL NOT change `@sidebar_bottom_height`, and a rule drag
in a column SHALL NOT change `@sidebar_tab_width`, so a column and a strip
on the same server keep their own split.

#### Scenario: a strip drag leaves a column alone
- **WHEN** window A has a strip and window B has a column with a bottom panel 30 percent high
- **AND** the user drags the divider in window A to 60 percent
- **THEN** the bottom panel of window B stays 30 percent high

#### Scenario: a column drag leaves a strip alone
- **WHEN** window A has a column and window B has a strip with a tab panel 40 percent wide
- **AND** the user drags the rule in window A
- **THEN** the tab panel of window B stays 40 percent wide

### Requirement: the split strip settings are shared

All sidebar instances on one tmux server SHALL share `@sidebar_split_tab_a`,
`@sidebar_split_tab_b`, `@sidebar_split_agents_width` and
`@sidebar_split_tab_b_width`. A change to one of them in one split strip
SHALL apply to every other split strip on the same server within one
refresh tick, with the same failed-save and drag-in-progress rules as the
other shared settings.

#### Scenario: a pane B tab change applies to another window
- **WHEN** window A and window B each have a split strip
- **AND** the user selects the Thought tab in pane B of window A
- **THEN** pane B of window B shows the Thought tab when its focused pane runs pi

#### Scenario: a divider drag applies to another window
- **WHEN** the user drags divider 1 in the split strip of window A until `@sidebar_split_agents_width` is 25
- **THEN** the split strip of window B shows an agent panel 25 percent wide

### Requirement: a split strip and a one-pane sidebar keep their own settings

A tab selection or a divider drag in a split strip SHALL NOT change
`@sidebar_bottom_tab`, `@sidebar_tab_width` or `@sidebar_bottom_height`.
A tab selection or a drag in a column or a one-pane strip SHALL NOT change
the split strip settings.

#### Scenario: a split selection leaves a column alone
- **WHEN** window A has a split strip and window B has a column that shows the Tools tab
- **AND** the user selects Git in pane A of window A
- **THEN** the column of window B still shows the Tools tab

#### Scenario: a column selection leaves a split strip alone
- **WHEN** window A has a column and window B has a split strip whose pane B shows Tasks
- **AND** the user selects the Git tab in window A
- **THEN** pane A and pane B of window B do not change
