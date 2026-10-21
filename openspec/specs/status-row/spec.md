# status-row Specification

## Purpose
Defines what the status row of an agent pane names and how it behaves when the
row runs out of width: the agent label, the pane's own name, and the reserved
elapsed label.

## Requirements

### Requirement: the status row names the agent

The status row of every agent pane SHALL begin with the agent's label
(`pi`, `claude`, `codex`, `opencode`) in the agent's colour, whether or not
the pane has any other name.

#### Scenario: A pane with a name still shows the agent

- **WHEN** an agent pane has a pane name or a session name
- **THEN** its status row still begins with the agent label

#### Scenario: A pane with no name

- **WHEN** an agent pane has no pane name
- **THEN** its status row shows only the agent label in the title slot

### Requirement: a named pane shows its pane name

The status row SHALL show the pane's tmux title after the badge slot when
the title is a real name. A title is a real name when it is non-empty,
differs from the host name, and does not start with a known agent OSC
prefix (`π - ` from pi, `✳ ` from Claude Code); other agents' OSC titles
are a known gap and may show as a name. The window's `automatic-rename`
setting SHALL NOT affect the result.

#### Scenario: A pane pmux named

- **WHEN** pmux named a pane with `select-pane -T my-task` and the pane runs pi
- **THEN** the status row shows `pi my-task`

#### Scenario: A named subagent pane shows its badge before its name

- **WHEN** pmux named a spawned subagent pane with `select-pane -T my-task`
  and the pane runs pi
- **THEN** the status row shows `pi ♟ my-task`


#### Scenario: A pane whose title is the host name

- **WHEN** the pane's title equals the host name (tmux's default, not a name
  given to the pane)
- **THEN** the status row shows only the agent label

#### Scenario: A pi pane with pi's own title

- **WHEN** the pane runs pi and its title is `π - fix the login bug - myrepo`
- **THEN** the status row shows only `pi`

#### Scenario: A claude pane with Claude's own title

- **WHEN** the pane runs claude and its title is `✳ Claude Code`
- **THEN** the status row shows only `claude`

#### Scenario: A title carrying an em dash

- **WHEN** the pane's title is `edit — fix bug`, `edit—fix bug`, or `edit — `
- **THEN** the status row shows the pane name as `edit`: only the text before
  an em dash is kept, because the suffix is decoration, not the name

#### Scenario: A pane with an empty title

- **WHEN** the pane's tmux title is empty
- **THEN** the status row shows only the agent label

### Requirement: the session name is not shown

The status row SHALL NOT show the agent's session name from the
session-name store. The session name's presence SHALL NOT change what the
title slot shows.

#### Scenario: A Claude pane with a session name

- **WHEN** a Claude pane has a session name in the session-name store
- **THEN** the status row shows the agent label and, when present, the pane
  name — never the session name

### Requirement: the elapsed label keeps its space and the name truncates first

The elapsed label SHALL keep its reserved space at the right edge of the
status row. When the title slot runs out of width, the pane name SHALL be
cut to the remaining budget with the ellipsis character; the agent label
SHALL NOT be truncated.

#### Scenario: A very long pane name

- **WHEN** a pane name is longer than the width left after the status icon,
  agent label, badge, and elapsed label
- **THEN** the pane name is truncated with an ellipsis and the elapsed label
  stays fully visible
