# command-highlighting Specification

## Purpose
Defines how the Tools tab colors the shell command of a `Bash` or `Monitor` entry by token class, so a command line reads like a highlighted shell prompt, and how the activity log keeps the characters the coloring needs.

## Requirements

### Requirement: only shell command entries are highlighted

The Tools tab SHALL color the label of an entry whose tool is `Bash` or `Monitor` by token class. The label of every other entry, `PowerShell` included, SHALL keep one muted text color. Coloring SHALL NOT change a character of the label.

#### Scenario: a Bash entry is colored
- **WHEN** the Tools tab shows a `Bash` entry with the label `cargo build --release`
- **THEN** `cargo` is in the command color, `build` is in the muted text color, and `--release` is in the flag color

#### Scenario: another tool stays muted
- **WHEN** the Tools tab shows a `Read` entry with the label `src/main.rs`
- **THEN** the whole label is in the muted text color

#### Scenario: PowerShell stays muted
- **WHEN** the Tools tab shows a `PowerShell` entry with the label `Get-Process -Name foo`
- **THEN** the whole label is in the muted text color

### Requirement: the command word is the first word of a segment

A command word SHALL be the first word of a command segment. A segment starts at the start of the label and after `|`, `||`, `&&`, `;`, `&`, `(` and `$(`. A leading `NAME=value` word SHALL NOT end the search for the command word: it draws in the variable color and the next word is the command word.

#### Scenario: a pipeline
- **WHEN** the label is `cat a.log | grep foo && echo done`
- **THEN** `cat`, `grep` and `echo` are in the command color, and `a.log`, `foo` and `done` are in the muted text color

#### Scenario: a leading assignment
- **WHEN** the label is `RUST_LOG=debug cargo test`
- **THEN** `RUST_LOG=debug` is in the variable color and `cargo` is in the command color

#### Scenario: a command substitution
- **WHEN** the label is `echo $(git rev-parse HEAD)`
- **THEN** `echo` and `git` are in the command color

### Requirement: flags are words that start with a dash

A word in an argument position that starts with `-` SHALL be in the flag color, `--` and `--name=value` included. A command word SHALL NOT be a flag.

#### Scenario: short and long flags
- **WHEN** the label is `ls -la --color=auto /tmp`
- **THEN** `-la` and `--color=auto` are in the flag color and `/tmp` is in the muted text color

### Requirement: strings, variables and comments have their own colors

A word or part of a word inside single or double quotes, quotes included, SHALL be in the string color. A string that is not closed SHALL run to the end of the label. A `$` followed by a name, a digit, `?` or `{...}` outside single quotes SHALL be in the variable color. A `#` that starts a word SHALL start a comment in the comment color, to the end of the label. A backslash SHALL make the next character plain text.

#### Scenario: quoted text with a pipe
- **WHEN** the label is `grep "a | b" file`
- **THEN** `"a | b"` is in the string color, with the `|` inside it not drawn as an operator, and `file` is in the muted text color

#### Scenario: a variable
- **WHEN** the label is `echo $HOME ${USER}`
- **THEN** `$HOME` and `${USER}` are in the variable color

#### Scenario: an unclosed string
- **WHEN** the label is `echo "never closed`
- **THEN** `"never closed` is in the string color

#### Scenario: a comment
- **WHEN** the label is `make # build all`
- **THEN** `# build all` is in the comment color

#### Scenario: a hash inside a word
- **WHEN** the label is `git log abc#1`
- **THEN** `abc#1` is in the muted text color

### Requirement: operators and redirections have their own colors

The separators `|`, `||`, `&&`, `;` and `&`, and the substitution marks `(`, `)` and `$(`, SHALL be in the operator color. A redirection (`>`, `>>`, `<`, `<<`, `<<<`, `&>`, and these with a leading file descriptor or a trailing `&n`, such as `2>` and `2>&1`) SHALL be in the operator color. The target word of a redirection SHALL be in the muted text color.

#### Scenario: a redirection
- **WHEN** the label is `make 2>&1 > build.log`
- **THEN** `2>&1` and `>` are in the operator color and `build.log` is in the muted text color

#### Scenario: a separator
- **WHEN** the label is `cd src; ls`
- **THEN** `;` is in the operator color and `ls` is in the command color

### Requirement: the colors are tmux options

Each token class SHALL have a color the user sets with a tmux option: `@sidebar_color_cmd_command`, `@sidebar_color_cmd_flag`, `@sidebar_color_cmd_string`, `@sidebar_color_cmd_variable`, `@sidebar_color_cmd_operator` and `@sidebar_color_cmd_comment`. The muted text color of an argument word SHALL stay `@sidebar_color_text_muted`. A missing or invalid option value SHALL give the default color of the class.

#### Scenario: a user sets the flag color
- **WHEN** `@sidebar_color_cmd_flag` is `#ff0000` and the Tools tab shows `ls -la`
- **THEN** `-la` is in `#ff0000`

#### Scenario: an invalid value
- **WHEN** `@sidebar_color_cmd_string` is `not-a-color`
- **THEN** a string is drawn in the default string color

### Requirement: wrapping and truncation do not change

A highlighted label SHALL wrap exactly as an unhighlighted label does: two columns of indent, at most three lines, words kept whole unless a word is wider than the line, and an ellipsis at the end of the last line when text remains. Every character SHALL keep its token class after the wrap. The ellipsis SHALL be in the muted text color.

#### Scenario: a string wraps
- **WHEN** the sidebar is 20 columns wide and the label is `echo "one two three four five"`
- **THEN** the label wraps over several lines as an unhighlighted label does, and every word of the quoted text is in the string color on each line

#### Scenario: the label is cut
- **WHEN** the label needs more than three lines
- **THEN** the third line ends with an ellipsis in the muted text color, and the earlier characters keep their colors

### Requirement: the activity log keeps the pipe

The label that the hook writes to the activity log SHALL keep every `|` of the command. A line break in the label SHALL be replaced with a space, so an entry stays one line. The sidebar SHALL read a label that holds `|` back unchanged.

#### Scenario: a pipeline is logged
- **WHEN** a `Bash` event has the command `cat a.log | grep foo` and the hook writes the activity entry
- **THEN** the log line is `<time>|Bash|cat a.log | grep foo` and the sidebar reads the label `cat a.log | grep foo`

#### Scenario: a multi-line command is logged
- **WHEN** a `Bash` event has a command with a line break
- **THEN** the log holds one line for the entry, and the line break is a space

#### Scenario: an old log line
- **WHEN** the log holds a line written before this change, where each pipe is a space
- **THEN** the Tools tab draws it with the command, flag and string colors and no operator
