# Design

## Context

See proposal.md for the motivation. The current strip has one source of
geometry: `strip_layout_for_tab_cols` in `src/state/layout.rs` builds a
`StripLayout` (filter, agents, divider, tabs, tab_bar), and the renderer
(`draw_strip` in `src/ui/mod.rs`), the press routing
(`handle_strip_press`) and the divider drag (`SplitDrag`, axis `Strip`) all
read it. The tab panel draws one tab: `draw_bottom_content` matches on
`displayed_bottom_tab()`. Each tab already owns its scroll state
(`activity.scroll`, `tasks.scroll`, `scrolls.git`, `scrolls.thought` with
`thought.follow`). Focus is a three-value enum (`Filter`, `Panes`,
`ActivityLog`). The shared settings are save latches in
`src/state/global.rs`, read each tick by the one `display-message` call in
`src/tmux/panes.rs`.

## Goals / Non-Goals

**Goals:**
- Keep one geometry function as the single source for draw, hit test and
  drag, now with an optional second tab pane.
- Reuse the per-tab scroll state as it is. No tab can be in two panes, so
  no scroll state needs a second copy.
- Leave the column and the one-pane strip byte-for-byte as they are
  (their existing snapshots must not change).

**Non-Goals:**
- A split layout for a column.
- More than two tab panes, or a user-defined tab grouping.
- A migration of `@sidebar_tab_width` into the split options.

## Decisions

### D1. Fixed tab groups instead of two free selections

Pane A has Tools and Git; pane B has Tasks and Thought.
Rejected alternatives, from the explore session:
- Two free selections with a swap when both panes pick the same tab. It
  changes pane A when the user acts on pane B, which overrides the user's
  intent.
- Two free selections with pane A's tab dimmed in pane B's bar. No
  override, but the user must move pane A first, which is a confusing
  rule.
- Two free selections that allow the same tab in both panes. A shared
  scroll state makes a useless mirror; independent panes need every
  scroll state twice.

Disjoint groups remove the conflict and keep one scroll state per tab.

### D2. Thought fallback is per group

Pane B falls back from a hidden Thought to Tasks without a write, in the
same way the one-pane `displayed_bottom_tab` falls back to Activity. A
helper per pane (for example `displayed_split_tab(Pane::A | Pane::B)`)
returns the displayed tab; `shown_split_tabs(pane)` returns the group, with
Thought only for a pi focus. `draw_bottom_content` takes the tab as an
argument, and the one-pane path passes `displayed_bottom_tab()`.

### D3. Separate stored selections and widths for the split layout

`@sidebar_split_tab_a`, `@sidebar_split_tab_b`,
`@sidebar_split_agents_width` and `@sidebar_split_tab_b_width` are four new
shared settings, each with its own `SaveLatch` in `GlobalState`, read in
the same per-tick `display-message` as the others. A split strip never
reads or writes `@sidebar_bottom_tab` or `@sidebar_tab_width`.
Rejected: a split click that also writes `@sidebar_bottom_tab`. The sync
would go in one direction only (a column click cannot move the split
panes), which is confusing.

### D4. One width option per divider, as strip percents

Divider 1 stores the agent panel width; divider 2 stores pane B's width
with its divider; pane A takes the rest. Each drag then writes exactly one
option on release, as the current strip drag does.
Rejected:
- A fixed 50/50 line for divider 2: the user wants to set every panel.
- Pane A's share of the tab section (`@sidebar_tab_width` kept as the
  tab-section width): it keeps the agent panel at the one-pane width (half
  the strip for the requester), which is the problem the change solves.

Geometry: `available = width - 4`; `agents = round(width * a%)` clamped to
`[20, available - 40]`; `b = round(width * b%)` clamped to
`[20, available - agents - 20]`; `a_pane = available - agents - b`. The
split shows only when `width >= threshold` (threshold > 0) and
`available >= 60`; otherwise the one-pane path runs unchanged.

This clamp order applies to stored percents. A divider 1 drag uses a
tighter bound: the live agent columns are clamped to
`[20, available - b - 20]` with `b` the pane B columns at the press, so
divider 1 stops when pane A with divider 1 reaches 20 columns and never
changes pane B. A divider 2 drag clamps pane B to
`[20, available - agents - 20]`, so it never changes the agent panel.

### D5. Geometry: extend `StripLayout` with optional pane B fields

`StripLayout` gains `divider_b`, `tabs_b` and `tab_bar_b` (all `None` in a
one-pane strip). The existing `divider`, `tabs` and `tab_bar` mean pane A
in a split strip. A new `split_strip_layout(width, height, agents_cols,
b_cols)` builds the split variant; a drag calls it with the drag's live
columns, as `strip_layout_for_tab_cols` does today, after the drag
clamp of D4.

### D6. The drag names its divider

`SplitDrag` gains a target (`Divider1` or `Divider2`; a one-pane strip
uses `Divider1`). `size` is the agent columns for divider 1 and pane B's
columns for divider 2 in a split strip. A press on pane A's bar or divider
1 targets divider 1; a press on pane B's bar or divider 2 targets divider
2. The unmoved release selects a tab through the pane's own hitboxes;
`bottom_tab_targets` gains a pane tag (or a second target list) so a click
on pane B's bar writes only `@sidebar_split_tab_b`.

### D7. A fourth focus value

`Focus` gains `TabPaneB`. `ActivityLog` means pane A in a split strip. The
strip key map adds the step `ActivityLog -> TabPaneB`. When a frame draws
no pane B (the strip became narrow) and the focus is `TabPaneB`, the
renderer moves it to `ActivityLog`. The column key map never reaches
`TabPaneB`. The cycle key (BackTab) cycles pane B's group while pane B
has focus, and pane A's group otherwise. Mouse wheel routing picks the
pane by the drawn rects.

### D8. The git poller follows pane A

The background git poller runs while the displayed tab is Git. In a split
strip the flag is "pane A displays Git", updated on the same events
(BackTab, click release, adopted shared change).

### D9. Threshold read per tick

`@sidebar_tab_split_width` joins the per-tick option read (like
`@sidebar_min_width`). It is a configuration option, not a shared view
setting, so it has no save latch.

## Risks / Trade-offs

- [Rounding makes a saved percent land one column off the drag column] →
  Same as the current strip; scenarios use widths that round-trip, and the
  drag test asserts the re-laid divider column.
- [The latch fixture table in `global.rs` and the per-tick field list grow
  by four or five entries; a field-order slip mis-reads an option] → The
  existing fixture tests cover every latch; add one fixture per new latch
  and a parse test for the field order.
- [The focus fill changes from two sections to three] → The one-pane and
  column paths keep the two-section rule; only the split path paints three.
- [A threshold that is too low makes three narrow panels] → The 20-column
  minimums fall back to one tab pane.

## Migration Plan

No migration. The new options are not set by default, and a strip below
300 columns (or with the threshold at 0) behaves as before. Rollback is a
revert; the unread options stay in tmux and do nothing.
