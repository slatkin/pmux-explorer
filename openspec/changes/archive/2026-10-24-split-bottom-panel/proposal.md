# Proposal

## Why

The Activity/Tasks/Git panel is an inset box at the bottom of the sidebar
with a fixed height in lines. The box border takes 3 columns and 2 rows
from the content, a blank row separates it from the agent list, and the
height does not follow the pane height. The user cannot change the height
without a change to a tmux option.

## What Changes

- The bottom panel becomes a split part directly under the agents part.
  One horizontal rule separates the two parts. The rule holds the
  `Activity│Tasks│Git` tabs. The panel has no side border and no bottom
  border, and the blank row between the two parts goes.
- **BREAKING**: `@sidebar_bottom_height` is a percent of the sidebar
  height, not a line count. The default changes from `20` (lines) to `30`
  (percent). `0` still hides the panel. A configuration that sets a line
  count gets that number as a percent.
- The user can resize the split with the mouse: a press on the rule and a
  move resizes; a press and release with no move is a click, and a click
  on a tab label selects that tab (the tab changes on release, not on
  press).
- The split height is a shared view setting. A drag writes the new
  percent to the global tmux option on release, and every sidebar
  instance shows it within one refresh tick.
- Each part keeps at least 3 rows. When the sidebar is shorter than 6
  rows, the bottom panel does not show.
- The test `test_pet_enabled_preserves_bottom_panel_border` is deleted.
  The pet will be deprecated, so no test protects it. The pet band stays
  directly above the rule.

Out of scope:

- The floating panel. Its existing limit (at most a third of the panel
  height) stays and applies after the percent, so a drag stops at that
  limit while the sidebar floats. Later work on the floating panel must
  know this.
- The pet, apart from the deleted test.
- The website capture images, which show the old box.

## Capabilities

### New Capabilities

- `bottom-panel`: the layout of the Activity/Tasks/Git panel under the
  agent list — the rule with the tabs, the height as a percent, the
  minimum heights, and the mouse resize of the split.

### Modified Capabilities

- `shared-view-settings`: the split height becomes a fifth shared view
  setting, and a drag in progress is not reverted by a refresh tick.

## Impact

- `src/ui/mod.rs`, `src/ui/bottom.rs`: the layout split and the rule.
- `src/app/input.rs`, `src/state/layout.rs`, `src/state/tab.rs`: the mouse
  press, drag, and release on the rule.
- `src/state.rs`, `src/state/global.rs`, `src/tmux/panes.rs`,
  `src/app/setup.rs`: the percent as a shared option that is read every
  tick and written on release.
- `pmux-explorer.conf`, `website/src/content/docs/reference/tmux-options.md`,
  `fixtures/scenarios/common/_lib.sh`, `docs/state-management.md`: the
  new unit and default.
- Tests: the inline snapshots that show the box, and the tests that set
  `bottom_panel_height` as a line count.
