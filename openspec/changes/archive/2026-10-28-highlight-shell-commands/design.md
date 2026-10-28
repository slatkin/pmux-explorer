# Design

## Context

The label of an activity entry travels as one plain string: `extract_tool_label` (`src/cli/label.rs`) takes the `command` field, `write_activity_entry` (`src/cli/hook/activity.rs`) passes it through `sanitize_tmux_value` (`\n` and `|` become a space) and writes `time|tool|label`. `parse_entry` (`src/activity.rs`) reads it back with `splitn(3, '|')`, so a `|` inside the label already survives the read. `draw_activity_content` (`src/ui/bottom/activity.rs`) wraps the label with `wrap_text` (`src/ui/text.rs`), which returns plain `String` lines, and draws each line as one `Span` in `theme.text_muted`.

`sanitize_tmux_value` also feeds tmux pane options (`@pane_bg_cmd`, the prompt, the session id), where the `|` replacement may matter. This change leaves those callers alone.

See proposal.md for the motivation and the scope.

## Goals / Non-Goals

**Goals:**
- Token-class colors for `Bash` and `Monitor` labels that look like a shell prompt: command, flag, string, variable, operator, comment.
- No new dependency, no subprocess, no I/O on the draw path.
- The wrap result (line count, split points, ellipsis) stays identical to today's for every label.

**Non-Goals:**
- Matching the user's shell colors. The user's shell is fish with its built-in colors, and fish colors a command by a `$PATH` lookup; the sidebar runs in another process and directory, so that lookup would be wrong.
- Parsing the shell grammar. The tokenizer is a lexer with one bit of state ("next word is a command word"), not a parser.
- Highlighting `PowerShell` (cmdlet syntax differs) or the `@pane_bg_cmd` line in the pane row.

## Decisions

### 1. A hand-written tokenizer in a leaf module

`src/shell_highlight.rs` exposes `classify(label: &str) -> Vec<Class>`: one class per `char` of the label. `Class` is `Arg`, `Command`, `Flag`, `Str`, `Variable`, `Operator`, `Comment`. A per-character vector keeps the tokenizer independent of wrapping and lets a token that is split across lines (a long string) keep its class.

State machine, one pass left to right:
- `expect_command` is true at the start and after a separator (`|`, `||`, `&&`, `;`, `&`, `(`, `$(`).
- Outside quotes: whitespace is `Arg` and ends a word; `'` and `"` start a string; `\` makes the next char plain; `$` starts a variable (`$NAME`, `$1`, `$?`, `${...}`); `$(` is an operator and sets `expect_command`; `#` at a word start starts a comment to the end; separators and redirections are `Operator`.
- A plain word is `Command` when `expect_command` is set, else `Flag` when it starts with `-`, else `Arg`. A word shaped `NAME=value` while `expect_command` is set is `Variable` and leaves `expect_command` set.
- A redirection is a run of an optional digit, then `>`, `>>`, `<`, `<<`, `<<<` or `&>`, then an optional `&n`; the word that follows is `Arg`.
- Inside double quotes `$VAR` stays `Str`. This is simpler and reads fine at sidebar size.

*Alternatives:* `fish_indent --ansi` reformats the command (it rewrote `> /tmp/x` to `>/tmp/x` in a test), needs a subprocess per entry, and printed no color at all because the user sets no `fish_color_*` variable. `syntect` or `tree-sitter-bash` add a large dependency and a C build to a project that pins exact versions and ships one stripped binary, for labels that are one line long.

### 2. Wrap after classifying, with one shared layout core

`wrap_text` splits on whitespace into word tokens, hard-splits words wider than the line, and ends the last line with an ellipsis at a word boundary. Re-classifying each wrapped line would lose the state at a line start (a line that begins inside a string). Instead the wrap works on `(char, Class)` pairs: `wrap_text` keeps its signature and its tests, and a new `wrap_classified(text, classes, max_width, max_lines) -> Vec<Vec<(String, Class)>>` shares the layout core. `wrap_text` calls the core with the class `Arg` for every char. The ellipsis gets `Arg`, which is the muted text color.

*Alternative:* map the plain `wrap_text` output back to the class vector by walking both. It breaks on the ellipsis and on collapsed whitespace, and it duplicates the layout rules in a second place.

### 3. Draw: one span per run of equal class

`draw_activity_content` builds the two-space indent span, then one `Span` per run of equal class on each wrapped line. Only `Bash` and `Monitor` entries call `classify`; every other tool passes the `Arg` class to the same path, so there is one draw path. The classification is cheap (a label is under a few hundred chars) and runs per frame for the entries on screen. No cache is needed. If a profile ever disagrees, a cache keyed by the label belongs in `AppState`.

### 4. Six theme colors and options

`Theme` gets `cmd_command`, `cmd_flag`, `cmd_string`, `cmd_variable`, `cmd_operator`, `cmd_comment`, each read from `@sidebar_color_cmd_<class>` in the way the other colors are read (`src/ui/colors.rs`, constants in `src/tmux/options.rs`). `Arg` uses `text_muted`, so the unhighlighted base stays the one the user already tunes.

Defaults, in the palette the theme already uses: command `#7aa2f7`, flag `#7dcfff`, string `#e0af68`, variable `#bb9af7`, operator `#89ddff`, comment `#565f89`. All six differ from the muted text color `#a9b1d6`, and the tool name keeps its own color (`114`) on the line above.

### 5. The log label keeps `|`

A new `sanitize_log_label` replaces only `\n` and `\r` with a space and sits beside `sanitize_tmux_value` in `src/cli/mod.rs`. `write_activity_entry` uses it. The log stays one entry per line, and `parse_entry`'s `splitn(3, '|')` already keeps the pipes of the third field. `@pane_bg_cmd` and the other pane options keep `sanitize_tmux_value`.

A flattened line break loses the `;` meaning of a multi-line script: `cd x` and `ls` on two lines read as `cd x ls`, so `ls` is an argument. This is accepted; the sidebar shows a one-line glimpse, not a faithful script.

## Risks / Trade-offs

- [Lexer mistakes on odd syntax (here-docs, `case` arms, brace expansion, `[[ ]]`)] → The tokenizer never fails and never changes text; the worst case is a wrong color on an odd word. Unit tests pin the common shapes.
- [An unclosed quote colors the rest of the label as a string] → Same as a shell prompt before the closing quote; the spec states it.
- [Colors do not show in `insta` text snapshots] → Class correctness is pinned by tokenizer unit tests and by one styled snapshot (`render_to_styled_string`, as in `tests/styled_tests.rs`) of a Tools tab with a pipeline.
- [Six new options widen the documented surface] → They follow the existing `@sidebar_color_*` pattern and are listed in `tmux-options.md`.
- [Old log lines have spaces in place of pipes] → They draw with fewer colors, never wrong ones. The log trims itself, so the old lines age out.

## Migration Plan

No data migration. The new binary reads old and new log lines. An old binary reading a new log line shows the `|` in the label, which is correct text. Rollback is a revert of the change.
