# Proposal

## Why

On an ultra-wide terminal, a strip is very wide (381 columns on a full-screen
window of the requester's monitor). The strip splits this width between the
agent panel and one tab panel, so each panel is about 190 columns wide and
most of each panel is empty. The user can see only one tab at a time.
Issue 13 asks for a three-panel layout for this case.

## What Changes

- A strip that is wide enough shows two tab panes side by side, to the right
  of the agent panel: pane A and pane B. The strip goes to this split layout
  automatically and goes back to one tab pane when it becomes narrower.
- New option `@sidebar_tab_split_width`: the smallest strip width, in
  columns, that shows the split layout. The default is 300. The value 0 turns
  the split layout off.
- The tabs are in two fixed groups. Pane A has Tools and Git. Pane B has
  Tasks and Thought. No tab can show in both panes. For a pane that does not
  run pi, pane B shows Tasks and hides Thought.
- New shared options `@sidebar_split_tab_a` (`activity` or `git`, default
  `activity`) and `@sidebar_split_tab_b` (`tasks` or `thought`, default
  `tasks`) store the tab of each pane. A tab selection in a split strip does
  not change `@sidebar_bottom_tab`.
- Both dividers of a split strip can be dragged. New shared options
  `@sidebar_split_agents_width` (divider 1) and `@sidebar_split_tab_b_width`
  (divider 2) give the agent panel and pane B as a percent of the strip
  width, with a default of 33 each. Pane A gets the columns that are left. A
  split strip does not read or write `@sidebar_tab_width`.
- Each of the three panels keeps at least 20 columns. If the strip cannot
  give each panel 20 columns, it shows one tab pane, also above the
  threshold.
- The keyboard focus gets a fourth stop. Left and Right move through the
  filter bar, the agent panel, pane A and pane B. The tab cycle key cycles
  the tabs of the focused tab pane only.
- Pane A's tab bar keeps the focused pane's id and name. Pane B's tab bar
  shows only its tab glyphs.
- A column and a strip that is narrower than the threshold do not change.

## Capabilities

### New Capabilities

- `split-strip`: the split layout of a wide strip — the width threshold, the
  two tab groups and their stored selections, the three panel widths and
  their dividers, the focus order, and the tab bars of the two panes.

### Modified Capabilities

- `strip-layout`: the requirements for the strip's parts, tab bar, divider
  drag and arrow keys say that a strip has one tab panel. They change to
  apply to a one-pane strip and to point to `split-strip` for a split strip.
- `shared-view-settings`: the list of shared settings adds the two split tab
  selections and the two split widths, and states that a split strip and a
  one-pane strip keep their own tab selections and widths.

## Impact

- `src/state/layout.rs`: strip geometry (`StripLayout`,
  `strip_layout_for_tab_cols`), press routing and the divider drag
  (`SplitDrag`) for a second divider.
- `src/ui/mod.rs` (`draw_strip`) and `src/ui/bottom.rs`: draw two tab bars
  and two tab contents; `draw_bottom_content` takes the tab as an argument.
- `src/state/tab.rs`: per-pane shown tabs, displayed tab, cycle and click.
- `src/state/focus.rs` and `src/app/input.rs`: a fourth focus stop, the
  strip key map, mouse scroll routing, and the git poller flag (on while
  pane A shows Git).
- `src/state/global.rs`, `src/tmux/options.rs`, `src/tmux/panes.rs`: four new
  shared options with save latches, and the threshold option in the
  per-tick read.
- Tests: geometry, routing, key map and inline-snapshot frame tests for a
  split strip.
- Docs: `README` option list and `docs/state-management.md`.
