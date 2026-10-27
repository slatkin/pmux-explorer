# Tasks

## 1. Colour plumbing

- [x] 1.1 Add `SIDEBAR_COLOR_FOCUS_BORDER` (`@sidebar_color_focus_border`) in `src/tmux/options.rs`, re-export it from `src/tmux.rs`, and a `focus_border` field on `ColorTheme` in `src/ui/colors.rs` with the default `Rgb(0xE6, 0x98, 0x75)` (everforest orange), loaded through the existing `from_options` path. Verify with `from_options` unit tests covering a valid override and an invalid-value fallback, plus the defaults test in `tests/color_tests.rs`.

## 2. Startup border write

- [x] 2.1 Add a command builder + executor in `src/tmux/commands.rs` that runs `set-option -p -t <pane_id> pane-active-border-style fg=<colour>` for the sidebar's own pane, and call it once from the TUI setup path with `state.theme.focus_border`. Verify with unit tests for the pure argument builder (colour formatting, pane id quoting shape) and confirm the write is issued exactly once, not per tick.

## 3. Gates

- [x] 3.1 Run `cargo fmt`, `cargo clippy`, and `cargo test`; confirm every existing snapshot still passes unchanged (the TUI buffer is untouched). Then `cargo build --release` so the plugin symlink picks up the binary.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result.
