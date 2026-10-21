# Tasks

## 1. Status filter categories

- [x] 1.1 Replace the `background` category with `busy` in `src/state/filter.rs`: the enum, `VARIANTS`, `next`, `prev`, `as_str`, `from_label` (both `running` and `background` map onto `busy`), `matches` (busy matches the running and the background pane status) and `status_counts` (five counts, the busy count summing running and background). Verify: the unit tests in `src/state/filter.rs` cover the new category, the two old persisted labels, and the cycle order, and `cargo test filter` passes.
- [x] 1.2 Render the five chips in `src/ui/panes/filter_bar.rs` with a single space between them, and count the busy chip as running plus background. Verify: a unit test asserts the rendered row text.
- [x] 1.3 Map a click column to a category in `src/state/layout.rs` with the new categories and the single-space separator, so the hit test matches the drawn chips. Verify: a unit test clicks a column of each chip and asserts the selected category.
- [x] 1.4 Draw the busy chip icon in the running colour whether or not the busy chip is active, and draw the active chip's count in the accent colour. Verify: a unit test asserts the foreground of the busy icon and of the active count.
- [x] 1.5 Update the UI snapshots that show the chip row. Verify: `cargo insta accept` followed by `cargo test` passes.

## 2. The pop-out control on the top row

- [ ] 2.1 Make `render_filter_bar` in `src/ui/panes/filter_bar.rs` take the width, build the chip run, compute the gap, and return the column of the control, the way `render_secondary_header` does; store that column in `FrameLayout`. Verify: a unit test asserts the returned column and that the row is as wide as the area.
- [ ] 2.2 Draw the control at the right edge of the top row in the running colour, with the pop-out glyph when the sidebar is docked and the pop-in glyph when it is floating. Verify: a snapshot test shows the row with the control.
- [ ] 2.3 Hit-test the control in the click handler before the chip walk, so a click on the control never changes the status filter. Verify: a unit test clicks the control column and asserts the filter is unchanged.
- [ ] 2.4 Wire the `p` key to the same action as the control. Verify: a unit test in `src/app/input.rs` drives the `p` key and asserts the toggle happened.
- [ ] 2.5 Add the `p` shortcut to the keybindings table in `website/src/content/docs/reference/keybindings.md`. Verify: the table lists `p` for popping the sidebar out and in.

## 3. Floating and docking the pane

- [x] 3.1 Add the tmux calls in `src/tmux/commands.rs`: a `break-pane -W` that floats the pane with a given size and position, and a `join-pane` that puts it back, both reporting tmux's error. Build the argument lists in pure functions, the way `src/cli/toggle.rs` builds its split flags. Verify: unit tests assert the argument lists.
- [x] 3.2 Read the floating flag of the sidebar's own pane in the existing pane query, so the sidebar knows its mode from the pane and not from a remembered flag. Verify: a unit test asserts the parsed flag.
- [ ] 3.3 Implement the pop-out: capture the docked width, compute the panel geometry from the window size (40 columns wide, as tall as the window, anchored to the right edge), and float the pane. Verify: a unit test asserts the computed geometry for a given window size.
- [ ] 3.4 Report a refused pop-out with `AppState::set_flash`, so a zoomed window leaves the sidebar docked and tells the user. Verify: a unit test asserts that a failed call sets the flash message and leaves the mode unchanged.
- [ ] 3.5 Implement the pop-in: join the pane back into the layout and restore the captured docked width with `resize-pane -x`. Verify: a unit test asserts the restored width, and the live check in group 4 confirms it.
- [ ] 3.6 Clamp the Activity/Git bottom panel to a third of the panel height while the sidebar floats. Verify: a unit test asserts the clamped height for a given panel height.
- [ ] 3.7 Document the floating panel in `website/src/content/docs/features/` and note in the README requirements that the floating panel needs tmux 3.8 while the rest of the sidebar works on tmux 3.0+. Verify: the feature page describes the control, the geometry and the tmux version, and the README states the version.

## 4. Keeping the floating panel sized

- [ ] 4.1 Re-assert the panel height when the window height changed, next to the existing size guard in `src/app.rs`, so the panel keeps the height of the window and a hand-resized panel is not fought on every tick. Verify: a unit test asserts that the height is re-asserted on a window height change and left alone otherwise.

## 5. Live verification

- [ ] 5.1 Pop the sidebar out and back in against a live tmux: the panel floats at the right edge, the workspace panes take the full width, the filter, selection, scroll and bottom tab survive the round trip, and the docked width comes back. Verify: the round trip is run in a tmux window and the observed geometry and state are recorded in the change notes.
- [ ] 5.2 Pop the sidebar out in a zoomed window: the sidebar stays docked and the notice appears. Verify: the zoomed case is run against a live tmux and the notice is observed.
- [ ] 5.3 Run the repository gates: `cargo fmt`, `cargo test`, `cargo clippy` and `cargo fmt --check`. Verify: all four pass.
