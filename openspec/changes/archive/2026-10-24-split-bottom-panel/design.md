# Design

## Context

See `proposal.md` for the motivation and `specs/` for the behavior.

The current code that this change touches:

- `ui::draw` (`src/ui/mod.rs`) splits the frame into `Min(1)` for the
  agent list, a divider of 1 row (5 rows when the pet is on), and
  `Length(bot_h)` for the bottom panel. `bot_h` is
  `AppState::bottom_panel_height`, a line count that `app/setup.rs` reads
  once at startup from `@sidebar_bottom_height`. While the sidebar
  floats, `clamp_bottom_height` limits it to a third of the frame height.
  The drawn height goes to `FrameLayout::bottom_panel_height`, and the
  mouse handlers hit-test that value.
- `ui::bottom::draw_bottom` draws a `Borders::ALL` block, then draws over
  its top row with `top_line_border` (`╭ Activity│Tasks│Git ─╮`) and over
  its bottom row with `╰─╯`. The tab renderers get `block.inner(area)`.
- `AppState::handle_terminal_click` (`src/state/layout.rs`) sends a press
  on the first row of the bottom panel to `handle_bottom_tab_click`
  (`src/state/tab.rs`). That handler subtracts 1 for the `╭` and maps the
  remaining column to a label.
- `app/input.rs` handles only `Down(Left)`, `ScrollUp` and `ScrollDown`.
  `main.rs` enables `EnableMouseCapture`, which asks the terminal for
  button-motion events, so tmux sends `Drag(Left)` and `Up(Left)` to the
  pane with its default mouse bindings.
- The shared view settings live in `GlobalState` (`src/state/global.rs`).
  Each refresh tick reads them in one `display-message` call
  (`SHARED_OPTION_FIELDS` in `src/tmux/panes.rs`) and applies them with
  `GlobalState::apply_all`. A write that tmux refuses sets a latch that
  blocks adoption and retries on the next tick (`filter_save_pending`,
  `tab_save_pending`). A pending cursor move blocks adoption of the
  cursor in the same way.

## Goals / Non-Goals

**Goals:**

- One pure function gives the drawn row count from the percent and the
  frame height. The renderer, the hit test and the drag all use it, so
  the drawn rule and the clickable rule cannot be in different rows.
- The split height uses the same sync and latch pattern as the bottom
  tab, so it needs no new mechanism.

**Non-Goals:**

- No change to the floating-panel code or its limit.
- No change to the pet code. Only its divider band moves.
- No keyboard control for the split height.

## Decisions

### 1. The percent is a shared field of `GlobalState`

`AppState::bottom_panel_height` (a line count read at startup) goes. A
new `GlobalState::bottom_percent: u8` takes its place, with a
`last_saved_bottom_percent` and a `percent_save_pending` latch that copy
the bottom-tab latch exactly: `save_bottom_percent` writes
`set -g @sidebar_bottom_height <n>`, `retry_pending_saves` retries it,
and `apply_all` adopts the shared value when no write is pending and no
drag is active. `SIDEBAR_BOTTOM_HEIGHT` goes into `SHARED_OPTION_FIELDS`
before `SIDEBAR_REPO_FILTER`, which must stay last.

Parsing: an integer from 0 to 100 is the percent. An unset value, an
empty value, or any other value gives 30 (`DEFAULT_BOTTOM_PERCENT`).
`bottom_panel_height_from_options` and `bottom_panel_height_from_tmux`
in `ui/mod.rs` become this parse, or go if `apply_all` makes them
unused. The `BOTTOM_PANEL_HEIGHT` constant goes.

The startup read stays: `load_from_tmux` already applies every shared
option, so the line in `app/setup.rs` goes.

Alternative: keep the field on `AppState` and read it once at startup.
Rejected, because the specs require that a drag in one instance shows in
the others.

### 2. One function computes the drawn rows

```
split_rows(percent, height) -> u16
  percent == 0 or height < 6  -> 0
  rows = round(height * percent / 100)
  clamp(rows, 3, height - 3)
```

`ui::draw` then applies `clamp_bottom_height` when the sidebar floats,
as now. Thus the floating limit applies after the minimums. While the
sidebar floats, the result can be less than 3 rows on a panel shorter
than 9 rows. The floating panel is out of scope, so this stays.

The inverse for a drag:

```
percent_for_rows(rows, height) = round(rows * 100 / height)
```

On a sidebar higher than 100 rows, one percent is more than one row.
Thus the rule can move by one row when the release changes from the
drag rows to the saved percent. On a sidebar of 100 rows or less, the
round trip gives the same rows. This is acceptable, and keeps the option
a plain percent as agreed.

### 3. The rule and the content

`draw_bottom` draws no block. The first row of its area is the rule, and
the rest of the area goes to the tab renderers, with the full width and
no padding. Thus the content starts at column 0, as the agent rows do.

`top_line_border` becomes a rule builder: `─ ` + the tab spans + ` ` +
`─` to the right edge. The width budget logic stays, with 3 fixed
columns in place of 4. The `─ ` prefix has the same width as the old
`╭ `, so the label columns do not move: `Activity` is on columns 2–9,
`Tasks` on 11–15, `Git` on 17–19, and the `│` separators on 10 and 16.

`handle_bottom_tab_click` today maps every `col - 1 < 9` to Activity, so
columns 0 and 1 (the `─ ` prefix) select Activity. The spec requires that
a click outside the labels changes nothing, so the Activity hit area
becomes `col - 1` in `1..9` (columns 2–9). The Tasks and Git ranges stay.

The divider row between the two parts goes. When the pet is on and the
agent part has room for the 5-row band and 3 agent rows, the band is
taken from the agent part, directly above the rule. Otherwise the band
does not show.

### 4. The press, drag and release state

A new `AppState::split_drag: Option<SplitDrag>` holds a press on the
rule:

```
struct SplitDrag {
    press_col: u16,
    moved: bool,     // a Drag event reached a row other than the press row
    rows: u16,       // the bottom rows that the drag shows
}
```

Mouse flow:

```
Down(Left) on the rule  --> split_drag = Some { moved: false, rows: drawn rows }
                            (no tab change on press)
Drag(Left), row != rule --> moved = true; rows = clamp of (height - row)
                            with the limits of decision 2 and the floating limit
Up(Left)                --> moved:  bottom_percent = percent_for_rows(rows),
                                    save_bottom_percent()
                            !moved: handle_bottom_tab_click(press_col)
                            split_drag = None
```

While `split_drag` has `moved == true`, `ui::draw` uses `rows` in place
of `split_rows(...)`. While `split_drag` is `Some`, `apply_all` does not
adopt the shared percent, so a tick cannot move the rule during the
drag. `apply_all` is a `GlobalState` method and `split_drag` is on
`AppState`, so the caller (`refresh.rs`, the `apply_all` call) passes
the drag state in; the implementer picks a parameter or a gate at the
call site.

A `Down(Left)` on another row keeps its current behavior. If a new
`Down(Left)` arrives while a drag is active (the release did not reach
the pane), the old drag ends as if released at its last rows, and the
new press is then handled. Thus a lost release does not leave the
instance blocked from adoption. This stale end runs only the `moved`
branch: a moved drag saves its rows, and an unmoved press ends with no
tab click, because the user did not release on that label.

`handle_terminal_click` changes so that a press on the rule starts the
drag and does not select a tab. Its return value told `input.rs` to sync
the git poller after a possible tab change. That sync moves to the
release path.

Alternative: select the tab on press and start a drag only on the dash
fill. Rejected in exploration: the user agreed to a resize from any
part of the rule.

### 5. Tests

- Unit tests for `split_rows`, `percent_for_rows` and the percent parse,
  with the numbers from the spec scenarios.
- `GlobalState` tests for the percent latch, copied from the tab latch
  tests, and for no adoption while a drag is active.
- `AppState` tests for press-move-release and press-release with no
  move, through the same entry points that `input.rs` calls.
- Inline snapshots (`insta::assert_snapshot!(output, @"...")`) for the
  new rule, the full-width content, and the hidden panel. The existing
  snapshots that show the box are accepted again. Their diffs show the
  box change and also one more agent-list row, because the divider row
  goes.
- `test_pet_enabled_preserves_bottom_panel_border` is deleted.
- Tests that set `state.bottom_panel_height` to a line count change to
  `state.global.bottom_percent` with a percent that gives a known row
  count for the test frame height. Tests that set
  `state.layout.bottom_panel_height` stay, because that field is still
  the drawn row count.

## Risks / Trade-offs

- [A user binding in tmux consumes `MouseDrag1Pane` or `MouseUp1Pane`]
  → The drag does not work, but a click still works if the release
  arrives. A lost release is handled by decision 4. A manual check in
  tmux is a task.
- [Users with a line count in their tmux configuration get a percent]
  → The release note and the option docs say that the unit changed.
  Values above 100 fall back to 30, so `set -g @sidebar_bottom_height
  120` cannot fill the sidebar.
- [A reload of `tmux.conf` sets the configured value again over a
  dragged value] → Accepted. The configured value is the user's
  choice.
- [The website capture images show the old box] → Out of scope; the
  `regenerate-captures` skill can update them later.

## Migration Plan

`pmux-explorer.conf` sets `@sidebar_bottom_height` to `30` when it is
empty. The docs and the scenario fixture change to the percent unit.
Rollback is a revert of the change; a dragged percent left in tmux is
then read as a line count until the user sets the option again.
