# Spec Delta

## Purpose

Lets the sidebar treat pi as an agent it knows: a pi pane is listed like any
other agent pane, and a producer can report pi activity through the documented
event protocol.

## ADDED Requirements

### Requirement: pi is a known agent

The sidebar SHALL recognise the agent label `pi`. A pane whose `@pane_agent`
option is `pi` SHALL be listed as an agent pane, SHALL be rendered with the pi
colour, and SHALL carry the label `pi` where the row shows the agent.

#### Scenario: A pi pane is listed
- **WHEN** a pane carries `@pane_agent` set to `pi`
- **THEN** the pane appears in the sidebar's pane list with the agent label `pi`

#### Scenario: A pi pane is coloured as pi
- **WHEN** the pi colour option `@sidebar_color_agent_pi` is set to a colour
- **THEN** the agent part of a pi row is drawn in that colour

#### Scenario: An unknown agent label is still dropped
- **WHEN** a pane carries `@pane_agent` set to a label the sidebar does not know
- **THEN** the pane is dropped from the pane list, as before

#### Scenario: The pi colour has a default
- **WHEN** `@sidebar_color_agent_pi` is unset
- **THEN** the pi agent part is drawn in the theme's default pi colour

### Requirement: pi events are accepted

The `hook` subcommand SHALL accept the agent name `pi` and SHALL map the
protocol payloads into the internal events, so that a pi producer drives the
same pane state as every other agent.

#### Scenario: A session start
- **WHEN** the producer sends `session-start` with `cwd`, `session_id` and `source`
- **THEN** the pane is marked as a pi session with that working directory and session id

#### Scenario: A user prompt
- **WHEN** the producer sends `user-prompt-submit` with a `prompt`
- **THEN** the pane status becomes running and the pane shows that prompt

#### Scenario: A finished turn
- **WHEN** the producer sends `stop` with `last_assistant_message`
- **THEN** the pane status becomes idle and the pane shows that message as the response preview

#### Scenario: A failed turn
- **WHEN** the producer sends `stop-failure` with an `error`
- **THEN** the pane status becomes error and the pane shows the error as the wait reason

#### Scenario: An end of session
- **WHEN** the producer sends `session-end`
- **THEN** the pane's pi state is cleared

#### Scenario: An unknown event name
- **WHEN** the producer sends an event name the sidebar does not know
- **THEN** the sidebar ignores it and exits successfully

### Requirement: pi tool names are normalised

The pi adapter SHALL map pi's tool names onto the canonical tool vocabulary, so
that the activity log and its label extractor work for pi as they do for the
other agents. The mapping SHALL cover `read`, `edit`, `write`, `bash`,
`powershell`, `grep`, `find` and `ls`.

#### Scenario: A pi read tool
- **WHEN** the producer sends `activity-log` with `tool_name` `read` and a `tool_input` holding `path`
- **THEN** the activity entry names the tool `Read` and labels it with the basename of that path

#### Scenario: A pi bash tool
- **WHEN** the producer sends `activity-log` with `tool_name` `bash` and a `tool_input` holding `command`
- **THEN** the activity entry names the tool `Bash` and labels it with that command

#### Scenario: An unknown pi tool name
- **WHEN** the producer sends `activity-log` with a tool name outside the mapping
- **THEN** the entry keeps that name and has an empty label

### Requirement: the producer protocol is documented

The repository SHALL carry a document that states, for every event of the
protocol, the JSON fields a producer sends and what the sidebar does with them.
The `setup` subcommand SHALL accept the agent name `pi` and SHALL print that
contract instead of merging a settings file, because pi has no hook
configuration file.

#### Scenario: The protocol document exists
- **WHEN** the repository is read
- **THEN** a document states the event names, the fields of each event, and the meaning of each field

#### Scenario: The setup command for pi
- **WHEN** `tmux-agent-sidebar setup pi` runs
- **THEN** it prints the producer contract and exits successfully, and it changes no configuration file

### Requirement: a dead pi pane is cleared

The sidebar SHALL clear the state of a pi pane whose pi process has exited,
so that a pane that fell back to a shell does not keep showing the agent's
last prompt and status.

#### Scenario: The pi process is gone
- **WHEN** a pane carries `@pane_agent` set to `pi` and no pi process is left in its process tree
- **THEN** the pane's pi state is cleared and the pane is dropped from the pane list

#### Scenario: The pi process is alive
- **WHEN** a pane carries `@pane_agent` set to `pi` and a pi process is still running in its process tree
- **THEN** the pane keeps its state and stays in the pane list

### Requirement: a pi pane that waits for a human is marked

The sidebar SHALL mark a pi pane that waits for a human, the same way it marks
a pane of any other agent: the status becomes waiting, the attention flag is
set, and the desktop notification fires.

#### Scenario: The producer reports a wait
- **WHEN** the producer sends `notification` with a `wait_reason`
- **THEN** the pane status becomes waiting, the pane carries the attention flag, and the reason is shown

#### Scenario: The wait is over
- **WHEN** the producer sends `user-prompt-submit` with an empty prompt after a wait
- **THEN** the pane status becomes running, the attention flag is cleared, and the pane keeps the prompt text it had

### Requirement: the gaps stay gaps

The sidebar SHALL NOT report a permission mode, a task progress or a subagent
tree for a pi pane, because pi reports none of them. The pi adapter SHALL
report the default permission mode.

#### Scenario: A pi pane with a finished turn
- **WHEN** a pi turn ends
- **THEN** the pane shows the idle status and no permission mode badge

#### Scenario: The permission mode of a pi pane
- **WHEN** a pi pane is listed
- **THEN** the pane carries no permission mode badge
