# Tasks

## 1. Options and shared state

- [x] 1.1 Add `SIDEBAR_TAB_SPLIT_WIDTH`, `SIDEBAR_SPLIT_TAB_A`, `SIDEBAR_SPLIT_TAB_B`, `SIDEBAR_SPLIT_AGENTS_WIDTH` and `SIDEBAR_SPLIT_TAB_B_WIDTH` to `src/tmux/options.rs`, and the five fields to the per-tick `display-message` reading in `src/tmux/panes.rs` (before the repo-filter field); verify with a parse test of the field order that reads each new value and an empty value for each.
- [x] 1.2 Add the pure parses in `src/ui/mod.rs`: threshold (unset, empty or not a whole number gives 300; 0 is off), split tab A (`activity`|`git`, else `activity`), split tab B (`tasks`|`thought`, else `tasks`), and the two widths (1-100, else 33); verify with unit tests for each valid value, an empty value, a value from the other group (`tasks` for A) and an out-of-range width.
- [x] 1.3 Add four `SaveLatch` fields to `GlobalState` (two split tabs, two split widths) with save, retry and adoption in `apply_all`, the widths blocked during a split drag; verify by adding one `LatchFixture` per latch to the fixture table in `src/state/global.rs` and that the whole latch suite passes.
- [x] 1.4 Seed `@sidebar_tab_split_width 300`, `@sidebar_split_agents_width 33` and `@sidebar_split_tab_b_width 33` in `pmux-explorer.conf` with header comments; verify with a seed test like `the_conf_seeds_the_same_width_percent_the_binary_falls_back_to` for each default.
- [x] 1.5 Document the five options in `website/src/content/docs/reference/tmux-options.md` and the four shared ones in `docs/state-management.md`; verify the table rows render (no broken table columns) by reading the files back.

## 2. Split geometry

- [x] 2.1 Extend `StripLayout` with `divider_b`, `tabs_b` and `tab_bar_b` (`None` in a one-pane strip) and add the pure `split_strip_layout(width, height, agents_cols, b_cols)` per design D4/D5; verify with unit tests for the spec scenarios: 400 columns at the defaults (columns 4-135, 136, 137-267, 268, 269-399), agents at 95 percent (356/20/20), and a 60-column strip with threshold 50 (no pane B).
- [x] 2.2 Add the selector that picks the split or one-pane geometry from the strip width, the threshold and the minimums; verify with unit tests for 381 and 250 columns at the default threshold, threshold 0 at 381 columns, and that the one-pane result equals `strip_layout` exactly.

## 3. Tab groups

- [x] 3.1 Add `shown_split_tabs(pane)` and `displayed_split_tab(pane)` in `src/state/tab.rs` (pane B: Thought only for a pi focus, a hidden Thought displays as Tasks without a write); verify with state tests for a pi focus, a Claude focus, and a focus change from a Claude pane back to a pi pane with `thought` stored.
- [x] 3.2 Add the per-pane cycle (BackTab: the focused tab pane's group, pane A's group when no tab pane has focus) and the per-pane click handler that writes only its pane's option; verify with state tests that pane B's cycle leaves pane A and `@sidebar_bottom_tab` unchanged and that a pane A click leaves pane B unchanged.
- [x] 3.3 Make `draw_bottom_content` take the tab as an argument and make the one-pane callers pass `displayed_bottom_tab()`; verify that the whole snapshot suite passes with no accepted snapshot change.

## 4. Split drawing

- [x] 4.1 Add the split branch to `draw_strip`: divider 1 and 2 lines, pane A and pane B contents, pane A's tab bar with the pane label, pane B's bar with glyphs only (pane A: Tools then Git; pane B: Tasks then Thought), hitboxes tagged by pane; verify with inline `insta` snapshots in `tests/ui_snapshot.rs` for a split strip with a pi focus and with a Claude focus.
- [x] 4.2 Paint the three-section focus fill in a split strip; verify with inline snapshots in `tests/focus_fill_tests.rs` for `Panes`, `ActivityLog` (pane A) and `TabPaneB` focus, and that the one-pane strip fill snapshots do not change.

## 5. Split mouse input

- [x] 5.1 Give `SplitDrag` a divider target and route a split press: divider 1 or pane A's bar targets divider 1, divider 2 or pane B's bar targets divider 2; verify with state tests for a press on each divider and each bar.
- [x] 5.2 Make the drag move follow the mouse column per target within the minimums, and the moved release write only its option; verify with state tests for the spec scenarios (divider 1 to column 104 writes 25 to `@sidebar_split_agents_width`, divider 2 to column 300 writes 25 to `@sidebar_split_tab_b_width`, divider 2 to column 140 stops at the pane A minimum, divider 1 to column 350 stops at column 248 with pane B unchanged, one write on release, `@sidebar_tab_width` unchanged).
- [x] 5.3 Make an unmoved release on a bar select the tab in that pane, and keep the git poller flag on while pane A displays Git; verify with state tests for a click on pane B's Thought glyph and for the flag after a pane A Git click.
- [x] 5.4 Route `handle_mouse_scroll` to pane A or pane B by the drawn rects; verify with a state test for each pane.

## 6. Split keyboard input

- [ ] 6.1 Add `Focus::TabPaneB`, the strip key steps `ActivityLog -> TabPaneB` (Right stops at pane B), Up/Down scroll the focused pane, and the move from `TabPaneB` to `ActivityLog` when a frame draws no pane B; verify with key-event tests for each spec scenario and a test that the column key map is unchanged.
- [ ] 6.2 Add the split strip keys to `website/src/content/docs/reference/keybindings.md`; verify by reading the table back.

## 7. Integration

- [ ] 7.1 Run `cargo fmt --check`, `cargo clippy` and `cargo test`; verify all pass.
- [ ] 7.2 Run `cargo build --release` and check a split strip in a scratch tmux server (`tmux -L`) 381 columns wide with `@sidebar_position bottom`: capture the pane with `tmux capture-pane -p` and confirm three panels, then resize the window to 190 columns and confirm one tab panel. Record the output.
