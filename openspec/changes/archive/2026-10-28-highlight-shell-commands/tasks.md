# Tasks

## 1. The activity log keeps the pipe

- [x] 1.1 Add `sanitize_log_label` beside `sanitize_tmux_value` in `src/cli/mod.rs` (replaces only `\n` and `\r` with a space), with unit tests next to the existing `sanitize_tmux_value` tests; verify `cargo test sanitize_log_label` passes
- [x] 1.2 Use it in `write_activity_entry` (`src/cli/hook/activity.rs`) and leave the `@pane_bg_cmd` write on `sanitize_tmux_value`; update `write_activity_entry_sanitizes_label` to assert the label `cat file | grep foo bar` keeps its `|` and the entry stays one line; verify `cargo test activity` passes, including `handle_activity_log_sanitizes_command_before_writing` (adjust its expectation only for the log, not for the pane option)
- [x] 1.3 Add a `parse_entry` / `read_activity_log` test in `src/activity.rs` that reads back a label holding `|`; verify it passes
- [x] 1.4 Document in `docs/agent-protocol.md` (`activity-log`) that a label is one line, with line breaks as spaces and `|` kept; verify the text is English and matches the spec requirement "the activity log keeps the pipe"

## 2. The tokenizer

- [x] 2.1 Create `src/shell_highlight.rs` with `Class` and `classify(&str) -> Vec<Class>` (one class per `char`), register it in `src/lib.rs`, and implement the state machine from the design; verify `cargo build` passes
- [x] 2.2 Add unit tests in the module for every scenario of the spec: pipeline, leading assignment, `$(...)`, short/long flags, quoted text holding `|`, unclosed string, variables, comment, `#` inside a word, redirections (`2>&1`, `>`), separators, backslash escape, empty label, and non-ASCII text (the vector length equals the `char` count); verify `cargo test shell_highlight` passes
- [x] 2.3 Add the module to the module list in `CLAUDE.md` (leaf helpers line); verify the entry names the file and its one job

## 3. Theme colors and options

- [x] 3.1 Add the six `SIDEBAR_COLOR_CMD_*` constants to `src/tmux/options.rs`, the six `cmd_*` fields and defaults to `Theme` and the `read` calls in `src/ui/colors.rs`; verify `cargo build` passes
- [x] 3.2 Extend `tests/color_tests.rs` with a test that each option overrides its field and an invalid value keeps the default (state-field assertions, not frame substrings); verify `cargo test color` passes
- [x] 3.3 List the six options and their defaults in `website/src/content/docs/reference/tmux-options.md`; verify the table rows match the constants in `options.rs`

## 4. Wrap that keeps the class

- [x] 4.1 Factor the layout core of `wrap_text` in `src/ui/text.rs` so it works on `(char, Class)` pairs, add `wrap_classified`, and make `wrap_text` call the core with the class `Arg` for every char; verify every existing `wrap_text_*` test passes unchanged
- [x] 4.2 Add tests for `wrap_classified`: a string token split across two lines keeps its class on both, the ellipsis has the `Arg` class, a hard-split word keeps its classes, and the line texts equal the `wrap_text` result for the same input; verify `cargo test wrap_classified` passes

## 5. Draw the Tools tab

- [x] 5.1 In `src/ui/bottom/activity.rs` build the classes with `classify` for `Bash` and `Monitor` entries (all `Arg` for every other tool), call `wrap_classified`, and draw one `Span` per run of equal class with the theme color of the class; verify `cargo build` passes and the existing `tests/bottom_tests.rs` snapshots are unchanged
- [x] 5.2 Add a styled snapshot test (`render_to_styled_string`, inline `insta::assert_snapshot!`) in `tests/styled_tests.rs` for a Tools tab with a `Bash` pipeline entry and a `Read` entry; verify the snapshot shows the class colors on the `Bash` label and the muted color on the `Read` label
- [x] 5.3 Add a styled snapshot for a narrow sidebar where a quoted string wraps over two lines and the last line ends in an ellipsis; verify the string color continues on the second line
- [x] 5.4 Run `cargo fmt`, `cargo clippy` and `cargo test`, then `cargo build --release`; verify all pass with no warnings

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
