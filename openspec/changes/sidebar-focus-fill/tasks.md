# Tasks

## 1. Theme colours

- [ ] 1.1 Add `SIDEBAR_COLOR_FOCUS_BG` (`@sidebar_color_focus_bg`) and `SIDEBAR_COLOR_FOCUS_ACTIVE_BG` (`@sidebar_color_focus_active_bg`) constants in `src/tmux/options.rs`, and `focus_bg` / `focus_active_bg` fields on `ColorTheme` in `src/ui/colors.rs` with defaults `Indexed(234)` / `Indexed(236)`, loaded through the existing `from_options` path. Verify with a `from_options` unit test covering a valid override and an invalid-value fallback for each option.

## 2. Focus fills in `draw()`

- [ ] 2.1 In `src/ui/mod.rs::draw`, when `state.focus_state.sidebar_focused` is true, fill `chunks[0]` and `chunks[1]` with `Block::default().style(Style::default().bg(..))` before the section renders: the keyboard-owning section gets `focus_active_bg` (`ActivityLog` → bottom, otherwise top; top also gets the active fill when `bot_h == 0`), the other gets `focus_bg`. When not focused, paint nothing. Verify with a TestBackend snapshot test per spec scenario: focused+Panes (top active / bottom base), focused+ActivityLog (bottom active / top base), unfocused (no fill on any cell), and the `bot_h == 0` fallback.

## 3. Interaction checks

- [ ] 3.1 Verify via cell-level buffer assertions that the selected pane's marker rows still show `selection_bg` over the fill (patch semantics), and that a popup / flash banner rendered with `Clear` shows no fill. Verify also that the pet band inside `chunks[0]` carries the top fill when the pet is enabled.
- [ ] 3.2 Run `cargo fmt`, `cargo clippy`, and `cargo test`; confirm existing snapshots still pass unchanged (unfocused rendering is untouched).

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result.
