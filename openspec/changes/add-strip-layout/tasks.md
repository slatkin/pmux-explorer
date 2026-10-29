# Tasks

## 1. Positions, options and pane creation

- [x] 1.1 Extend `SidebarPosition` in `src/cli/toggle.rs` with `Top` and `Bottom`, `axis()`, `opposite()` on the same axis, `as_setting()` and `from_setting()` (`top`/`bottom` accepted, any other value gives `Left`); verify with unit tests for every value and for `opposite()` on both axes (`cargo test` passes).
- [x] 1.2 Add `-vfb` / `-vf` to `split_window_flags` and a strip edge target that reads `#{pane_top} #{pane_height} #{pane_id}` (smallest top for `Top`, largest bottom edge for `Bottom`); verify with unit tests in the style of `target_pane_for_left_position_uses_leftmost_pane`, including malformed lines.
- [x] 1.3 Add `@sidebar_height` (`SIDEBAR_HEIGHT` in `src/tmux/options.rs`) with the `@sidebar_width` parse against `#{window_height}`, `DEFAULT_HEIGHT_PERCENT = 30`, and the `30%` seed in `pmux-explorer.conf`; verify with a parse test and a seed test like `the_conf_seeds_the_same_width_percent_the_binary_falls_back_to`.
- [x] 1.4 Make `cmd_toggle` use the vertical flags, the strip target and `@sidebar_height` for `top`/`bottom`, pass `-e PMUX_SIDEBAR_LAYOUT=column|strip` to `split-window`, and write `@pane_sidebar_layout` (`column` or `strip`) next to `@pane_role`; verify with a unit test of the `split-window` arguments for each position, and in a scratch tmux 3.8 server (`tmux -L`) that `@sidebar_position bottom` gives a full-width pane at the bottom edge, 30% of the window high, with `@pane_sidebar_layout strip` and `PMUX_SIDEBAR_LAYOUT=strip` in the pane process environment. Record the output.
- [x] 1.5 Document `top`/`bottom`, `@sidebar_height` and that the layout is fixed when the sidebar is made in the `pmux-explorer.conf` header and `website/src/content/docs/reference/tmux-options.md`.

## 2. Layout kind and shared tab width

- [x] 2.1 Add `LayoutKind { Column, Strip }` and `AppState::layout_kind`, read once at startup from the `PMUX_SIDEBAR_LAYOUT` environment variable in `src/app/setup.rs`, never from the pane option (missing or unknown value gives `Column`); verify with a unit test of the pure parse for `strip`, `column`, an unknown value and a missing value.
- [x] 2.2 Add `@sidebar_tab_width` (`SIDEBAR_TAB_WIDTH`) as a shared field in `GlobalState` with `last_saved_tab_percent`, `tab_percent_save_pending`, a save, a retry and adoption in `apply_all` blocked during a drag, placed before `SIDEBAR_REPO_FILTER` in `SHARED_OPTION_FIELDS`; verify with tests that copy the `bottom_percent` latch tests and a parse test for the reading format.
- [x] 2.3 Add `DEFAULT_TAB_PERCENT = 40` and the `@sidebar_tab_width 40` seed in `pmux-explorer.conf`; verify with a seed test. Add the option to `docs/state-management.md` and the website tmux-option page.

## 3. Strip geometry

- [x] 3.1 Add the pure `strip_layout(width, height, tab_percent)` and `cols_for_percent` in `src/state/layout.rs`: filter 4 columns, divider 2 columns, each panel at least 20 columns, no tab panel below 44 columns or at 0 percent; verify with unit tests for the spec scenarios (120 columns at 40, 1 and 100 percent; 40 columns; 0 percent) and the drag round trip (120 columns, divider to column 60 gives 50).
- [x] 3.2 Add the pure `filter_column_layout(counts, height)`: one chip for each row from row 0, controls on the last row, chips dropped whole, counts of 100 or more drawn as `99`; verify with unit tests for heights 12, 4, 1 and 0.
- [x] 3.3 Add the pure `divider_tab_rows(tabs, height)`: glyph rows centered at `floor((H - R) / 2)` and cut at the bottom; verify with unit tests for a Claude pane and a pi pane at 12 rows, and a run taller than the strip.

## 4. Strip drawing

- [x] 4.1 Split `bottom::draw_bottom` into the column rule and `draw_bottom_content`, and let `panes::draw_agents` draw without the filter row; verify that the whole column snapshot suite passes with no accepted snapshot change.
- [x] 4.2 Add the `Strip` branch to `ui::draw`: the two section fills, `draw_strip_filter_bar`, the agent panel, `draw_strip_divider` (band in the filter-bar background, glyphs, accent on the displayed tab, `bottom_tab_targets` written as rows) and the tab content; no pet. Store the drawn `StripLayout` in `FrameLayout`. Verify with inline `insta` snapshots in `tests/ui_snapshot.rs` for a 120x12 strip with the Tools tab, a pi pane, `@sidebar_tab_width 0`, and a 40-column strip.
- [x] 4.3 Draw the side-switch glyph `⇅` in a strip (dimmed while floating) and keep `⇄` in a column; verify with inline snapshots of the strip control row, docked and floating.
- [x] 4.4 Verify the strip fills with inline snapshots in `tests/focus_fill_tests.rs` for `Panes` focus and `ActivityLog` focus.

## 5. Strip mouse input

- [x] 5.1 Give `SplitDrag` an axis and a `size` field; route a strip press in `handle_terminal_click` by the drawn `StripLayout`: the divider starts a drag, the filter column goes to a new `handle_filter_column_click`, and the agent panel goes to a strip twin of `handle_mouse_click` (popups, `+` and `×` in absolute coordinates; `row == agents.y` to `handle_secondary_header_click(col - agents.x)`; list rows through `line_to_row` at `(row - agents.y - 1) + offset`). Verify with state tests for a chip click, a click on the repo header row (the status filter does not change), an agent row click with a scroll offset (the right agent is selected), a click on a `+` spawn button, and a press on the divider.
- [x] 5.2 Make `handle_split_drag_move` follow the mouse column in a strip and `end_split_drag` save `@sidebar_tab_width` on a moved release and select the tab of the press row on an unmoved release; verify with state tests for the spec scenarios (drag to column 60 gives 50, one write on release, a drag from a glyph keeps the tab, a click on the band changes nothing), and that a column drag still saves only `@sidebar_bottom_height`.
- [x] 5.3 Route `handle_mouse_scroll` by column in a strip (agent panel or tab panel); verify with state tests for each panel.

## 6. Strip keyboard input

- [ ] 6.1 Add the strip key map in `src/app/input.rs`: Left/`h` and Right/`l` move focus through `Filter`, `Panes`, `ActivityLog` and stop at the ends; Up/Down step the category, move the selection without leaving the agent panel, or scroll the tab panel; verify with key-event tests for each spec scenario and a test that the column key map is unchanged.
- [ ] 6.2 Add a strip key table to `website/src/content/docs/reference/keybindings.md`.

## 7. Side switch in a strip

- [ ] 7.1 Add the vertical `join-pane -v -f -d [-b]` builder in `src/tmux/commands.rs`; verify with a unit test of the exact arguments for `Top` and `Bottom`.
- [ ] 7.2 Add `#{@pane_sidebar_layout}` and `#{pane_at_top}` to the sweep format; move each docked sidebar on the acting axis to the target edge (the opposite of the acting edge) and each sidebar on the other axis to its own opposite edge; set `@sidebar_position` to the target edge. Verify with unit tests of the parse and the target choice for: a column-only server with a skipped window (same result as today), two strips on different edges, and a mixed server. `switch_sidebar_side` reads the acting edge by `layout_kind`: `from_pane_at_top` (`#{pane_at_top}`) for a strip and `from_pane_left` for a column. The fallback from `@sidebar_position` is used only when the value is on the sidebar's own axis; otherwise a strip falls back to `Bottom` and a column to `Left`. Verify with unit tests: a strip at the top gives the target `Bottom`, a strip at the bottom gives `Top`, and a strip whose option says `left` falls back on its own axis.
- [ ] 7.3 Verify in a scratch tmux 3.8 server with a bottom strip in one window and a left column in another: the switch gives a top strip and a right column, and sets `@sidebar_position top`. Record the output.

## 8. Pop-out and pop-in of a strip

- [ ] 8.1 Add `height` (`#{pane_height}`) and `at_top` (`#{pane_at_top}`) to `SidebarPaneInfo` before the shared options; verify with the reading-parse tests.
- [ ] 8.2 Make `panel_geometry` take the layout kind and the docked edge, capture `docked_size` and the edge on pop-out, and skip `clamp_bottom_height` in a strip; verify with unit tests for the column (unchanged), a top strip and a bottom strip.
- [ ] 8.3 Pop a strip in with the edge join and `resize-pane -y <h>`; fall back to the bare join when there is no edge target; verify with outcome tests in `src/state/popout.rs`.
- [ ] 8.4 Make `panel_guard` keep a floating strip as wide as the window and at its anchor after a window resize; verify with unit tests in the style of `position_target_reasserts_after_a_window_width_change`.
- [ ] 8.5 Make `size_guard::enforce` do nothing for a strip; verify with a unit test.
- [ ] 8.6 Verify in a scratch tmux 3.8 server: pop out a 10-row bottom strip, resize the window, pop it back in; the strip floats full width at the bottom edge, stays flush after the resize, and docks again as a 10-row full-width strip at the bottom. Record the output.

## 9. Integration

- [ ] 9.1 Optional smoke run: start the `ui-showcase` scenario with `@sidebar_position bottom` and record a `tmux capture-pane` of the strip. Acceptance does not depend on this run; it depends on the inline snapshots of 4.2-4.4.
- [ ] 9.2 Run `cargo fmt`, `cargo clippy`, `cargo test` and `cargo build --release`; all pass.
