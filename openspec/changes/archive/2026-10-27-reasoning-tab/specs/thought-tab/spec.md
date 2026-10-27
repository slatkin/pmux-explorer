# Spec Delta

## Purpose

Defines the Thought tab of the bottom panel: where the sidebar finds the thinking feed of a pi pane, how the feed records make the last thought, and what the tab shows.

## ADDED Requirements

### Requirement: the feed of a pi pane is found by its session id

The sidebar SHALL read the thinking feed of the focused pi pane at `<dir>/<session id>.jsonl`, where `<session id>` is the pane's `@pane_session_id` and `<dir>` is `$XDG_RUNTIME_DIR/pmux-thinking`, or `/tmp/pmux-thinking-<uid>` when `XDG_RUNTIME_DIR` is unset or empty. A session id that contains `/` or is `.` or `..` SHALL give no feed.

#### Scenario: the runtime directory is set
- **WHEN** `XDG_RUNTIME_DIR` is `/run/user/1000` and the focused pi pane has session id `abc`
- **THEN** the sidebar reads `/run/user/1000/pmux-thinking/abc.jsonl`

#### Scenario: the runtime directory is unset
- **WHEN** `XDG_RUNTIME_DIR` is unset, the uid is 1000, and the session id is `abc`
- **THEN** the sidebar reads `/tmp/pmux-thinking-1000/abc.jsonl`

#### Scenario: an unsafe session id
- **WHEN** the session id is `../x`
- **THEN** the sidebar reads no file and the tab shows the empty notice

### Requirement: the records make the last thought

Each feed line SHALL be one JSON record `{kind, ts, producer, text?}`. A `start` record SHALL begin a new last thought with no text. A `delta` record SHALL append its text. An `end` record with text SHALL replace the text with its text; an `end` with no text SHALL keep the text. After an `end`, the thought SHALL stay until the next `start`. A line that is not a valid record SHALL be ignored.

#### Scenario: a thought grows
- **WHEN** the feed holds `start`, then `delta` "I should", then `delta` " read the file"
- **THEN** the tab shows "I should read the file"

#### Scenario: the end replaces the text
- **WHEN** the feed holds `start`, `delta` "draft", and `end` with text "final"
- **THEN** the tab shows "final"

#### Scenario: a finished thought stays
- **WHEN** the newest block ended and no later `start` is in the feed
- **THEN** the tab shows that block's text

#### Scenario: a new block clears the old one
- **WHEN** a block ended with "first" and a later `start` arrives
- **THEN** the tab no longer shows "first"

#### Scenario: a delta without a start
- **WHEN** the feed's newest records are `delta` records with no `start` before them
- **THEN** the tab shows their text as the last thought

#### Scenario: a broken line
- **WHEN** a line of the feed is not valid JSON
- **THEN** the line is ignored and the other records apply

### Requirement: the last thought is bounded

While a thought grows, the sidebar SHALL keep at most its last 16000 characters. The text of an `end` record SHALL be kept whole.

#### Scenario: a long streaming thought
- **WHEN** the deltas of one open block hold 40000 characters
- **THEN** the tab holds the end of the text and no more than 16000 characters of it

### Requirement: the thought is shown as plain text

The tab SHALL remove control characters and terminal escape sequences from the thought before it draws it, and SHALL keep line breaks. The tab SHALL wrap the text to the width of the block it draws. Above the text, the tab SHALL show the `HH:mm:ss` local time of the record that began the thought, in the timestamp color — `@sidebar_color_thought_timestamp`, which defaults to the 256-color index 220 (yellow). The thought text itself SHALL keep the normal text color.

#### Scenario: an escape sequence in the thought
- **WHEN** a delta holds `\x1b[31mred`
- **THEN** the tab shows `red` with no escape sequence

#### Scenario: the start time
- **WHEN** the newest `start` record has `ts` `2026-10-27T08:00:07Z` and the local time zone is UTC
- **THEN** the tab shows `08:00:07` above the thought, in yellow

#### Scenario: the timestamp color is overridden
- **WHEN** `@sidebar_color_thought_timestamp` is set to an index or a hex color
- **THEN** the time line draws in that color and the thought text keeps the normal text color

### Requirement: the thinking block is indented and starts at the top

The thinking block — the `HH:mm:ss` line when drawn plus the wrapped thought rows — SHALL start at the first content row of the panel and SHALL be drawn one column in from each edge, with the thought wrapped at the panel width less those two columns, so the last column stays blank. A block shorter than the panel SHALL leave the rows under it blank. A taller block SHALL fill the panel and follow its end.

#### Scenario: the block is indented
- **WHEN** the panel content area is 28 columns wide and the thought does not fit one row
- **THEN** the block draws starting at the second column, each row holds at most 26 characters, and the last column is blank

#### Scenario: a block that fits starts at the top
- **WHEN** the panel shows 6 content rows and the block is 3 rows
- **THEN** the block occupies the first three content rows and the three under it are blank

#### Scenario: the block is taller than the panel
- **WHEN** the block is 31 rows and the panel shows 10 content rows
- **THEN** the tab fills the panel from its first content row and shows the block's last 10 rows — thought rows 21 to 30

### Requirement: the tab shows the end of a long thought

When the thought is taller than the panel, the tab SHALL show its last rows. The user SHALL be able to scroll up and down inside that one thought. A thought that grows SHALL keep the view at its last rows unless the user scrolled up.

#### Scenario: a tall thought
- **WHEN** the wrapped thought is 30 rows and the panel shows 10 content rows
- **THEN** the tab shows rows 21 to 30

#### Scenario: the user scrolled up
- **WHEN** the user scrolled up 5 rows and a delta adds text
- **THEN** the view does not move to the last rows

### Requirement: the tab has an empty notice

When the focused pi pane has no session id, no feed file, or no thought in its feed, the tab SHALL show the centered muted notice `No thinking yet`.

#### Scenario: a session that never thought
- **WHEN** the feed file of the focused pi pane is empty
- **THEN** the tab shows `No thinking yet`

### Requirement: the feed is read on the refresh tick

The sidebar SHALL read the feed only on its refresh tick (1 second, or a forced refresh), and only while the Thought tab is drawn. A read SHALL process only the bytes added since the previous read of the same feed. The first read of a feed SHALL begin at its newest `start` record, not at the start of the file.

#### Scenario: another tab is drawn
- **WHEN** the Tasks tab is drawn
- **THEN** the sidebar does not read any thinking feed

#### Scenario: a large feed
- **WHEN** the feed of the focused pane holds 5 MB of earlier blocks and the user selects the Thought tab
- **THEN** the sidebar reads only the newest block, not the whole file

#### Scenario: a thought appears within one tick
- **WHEN** pmux appends a delta while the Thought tab is drawn
- **THEN** the tab shows the new text within 1 second

### Requirement: a dead pane's feed is deleted

When the sidebar clears the state of a pi pane whose process has exited, it SHALL delete that pane's thinking feed file through the same path rule and session-id checks as a read. The session id SHALL be taken from the pane's `@pane_session_id` before the pane's state is wiped. A feed file that is absent SHALL NOT fail or delay the clearing of the pane's state.

#### Scenario: the agent exits
- **WHEN** a pi pane's process exits and its feed file exists
- **THEN** the feed file is removed when the pane's state is cleared

#### Scenario: no feed was written
- **WHEN** a pi pane's process exits and no feed file exists for its session
- **THEN** the pane's state is still cleared

#### Scenario: an unsafe session id
- **WHEN** a pi pane that carries session id `../x` exits
- **THEN** no file is removed and the pane's state is still cleared
