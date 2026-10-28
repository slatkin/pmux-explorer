# Proposal

## Why

The Tools tab shows each Bash command as one muted string. A long pipeline is hard to scan: the command word, the flags, the quoted strings and the pipe operators all look the same. A shell highlights these parts, and the tab should do the same. The activity log also loses every `|` before it writes a label, so `cat a.log | grep foo` reaches the tab as `cat a.log   grep foo` and no highlighter can see the pipe.

## What Changes

- The Tools tab draws the label of a `Bash` or `Monitor` entry with one color per token class: the command word, flags, quoted strings, variables, operators, redirections and comments. All other words (the arguments) keep the muted text color.
- A command word is the first word of a command segment: at the start of the label and after `|`, `||`, `&&`, `;`, `&`, `(` and `$(`. Leading `NAME=value` assignments are skipped, and they draw in the variable color.
- The highlight needs no shell, no `$PATH` lookup and no subprocess. It is a plain tokenizer, so an unknown command is not drawn as an error the way fish does.
- Wrapping and truncation do not change: the label keeps the two-column indent, at most three lines, and the closing ellipsis at a word boundary. The colors follow each character through the wrap.
- Each token class has a theme color with a `@sidebar_color_cmd_*` tmux option, like every other sidebar color.
- The activity log keeps the `|` character of a label. Only line breaks are replaced with a space, so the log stays one line per entry. The pane-row background-command value (`@pane_bg_cmd`) keeps its current sanitizing.
- Entries from other tools (`Read`, `Grep`, `WebFetch` and so on) and `PowerShell` entries draw as before.

Out of scope: the background-command line in the pane row, PowerShell syntax, a check that a command exists, heredocs and other multi-line structure (line breaks are already flattened), and reading the user's own shell colors.

## Capabilities

### New Capabilities
- `command-highlighting`: how the Tools tab colors the shell command of a `Bash` or `Monitor` entry — which token classes exist, how each is recognised, which colors and tmux options they use, and that wrapping, truncation and the log label keep the pipe.

### Modified Capabilities

None. `bottom-panel` fixes the panel layout and the width of an activity line; neither changes.

## Impact

- Code: a new leaf module `src/shell_highlight.rs` (the tokenizer); `src/ui/text.rs` (a wrap that keeps a class per character, sharing the layout core of `wrap_text`); `src/ui/bottom/activity.rs` (the draw); `src/ui/colors.rs` and `src/tmux/options.rs` (six theme colors and option names); `src/cli/mod.rs` and `src/cli/hook/activity.rs` (a log-label sanitizer that keeps `|`).
- Docs: `docs/agent-protocol.md` (the `activity-log` label), `website/src/content/docs/reference/tmux-options.md` (the new color options), `CLAUDE.md` (module list).
- Dependencies: none added.
- Compatibility: an old log line already holds spaces in place of pipes. It draws with the command, flag and string colors and no operators. No migration is needed.
- Not affected: the hook protocol (no new field), the adapters, the task store.
