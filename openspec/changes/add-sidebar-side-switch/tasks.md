## 1. tmux layer

- [x] 1.1 Add a pure `join-pane` argument builder for a move to the left or right edge in `src/tmux/commands.rs`, with a unit test that asserts the exact arguments for both sides (`cargo test` passes).
- [x] 1.2 Add a window sweep that lists every sidebar pane with its window, floating flag and outermost target pane, and moves each one, returning counts of moved and skipped windows. Unit-test the pure parts: choosing the target and counting a refused call as skipped (`cargo test` passes).

## 2. Switch action

- [x] 2.1 Add the switch action in a new `src/state/` module: derive the new side from the current one, run the sweep, set the global `@sidebar_position`, and set the flash text for moved and skipped counts. Unit-test the outcome handling with injected results, as `popout.rs` does (`cargo test` passes).
- [x] 2.2 Make the action a no-op while the sidebar is floating, with a unit test.

## 3. Control and key

- [x] 3.1 Reserve the side-switch control's columns in `filter_bar_layout`, draw its glyph (dimmed while floating) and add the click hit test in `src/state/layout.rs`. Update the top-row inline `insta` snapshots, including a narrow-width case (`cargo test` passes).
- [x] 3.2 Map `M-C-o` in `src/app/input.rs` to the action, ignored while a popup is open, with a key-event test.
- [x] 3.3 Add the `C-o` binding to `pmux-explorer.conf` with the focus guard and the `rotate-window` fallback; verify with `tmux source-file` and `tmux list-keys | grep C-o` in a test tmux server.

## 4. Verification and docs

- [x] 4.1 Verify in a scratch tmux server (tmux 3.8) with three windows, one zoomed: the switch moves two sidebars, skips the zoomed one, sets `@sidebar_position`, and a new window follows. Record the output.
- [x] 4.2 Document the control and the key in `docs/` and the website keybinding and tmux-option pages, including that the config value returns after a restart.
- [x] 4.3 Run `cargo fmt`, `cargo clippy`, `cargo test` and `cargo build --release`; all pass.

## 5. Icon tabs

- [x] 5.1 Draw each bottom-panel tab as its Nerd Font glyph in `BottomTab::label` (Tools , Tasks , Git 󰊢, Thought 󰟶); update the rule tests and inline snapshots (`cargo test` passes).
- [x] 5.2 Center the tab run on the rule in `top_line_border`, keep the hitboxes on the drawn columns, and update the column-range and click tests (`cargo test` passes).
