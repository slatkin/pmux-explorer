# pi-agent-support Specification

## Purpose
Lets the sidebar treat pi as an agent it knows: a pi pane is listed like any
other agent pane, and a producer can report pi activity through the documented
event protocol.

## Requirements

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

The sidebar SHALL NOT report a permission mode for a pi pane, because pi
reports none. The pi adapter SHALL report the default permission mode. A pi
pane CAN report task progress through the common task command into its own per-pane store, while every
other agent keeps its existing task reporting unchanged. The sidebar SHALL
NOT report an in-process subagent tree for a pi pane; a pmux subagent runs
in its own pane and is marked on that pane instead.

#### Scenario: A pi pane with a finished turn
- **WHEN** a pi turn ends
- **THEN** the pane shows the idle status and no permission mode badge

#### Scenario: The permission mode of a pi pane
- **WHEN** a pi pane is listed
- **THEN** the pane carries no permission mode badge

#### Scenario: A pi pane with per-pane tasks
- **WHEN** a pi pane records tasks through the common task command
- **THEN** the Tasks tab shows that pane's list

#### Scenario: No in-process subagent tree for pi
- **WHEN** a pi pane is listed
- **THEN** the pane shows no in-process subagent rows

### Requirement: a pmux task prompt marks the pane as a subagent

The sidebar SHALL recognise a pi `user-prompt-submit` prompt that starts with
`<file name="PATH">`, where PATH ends in
`/artifacts/<id>/context/<stem>.md` (spawned) or
`/artifacts/<id>/pmux-resume/<stem>.md` (resumed), and `<stem>` is a name, a
hyphen and a timestamp `YYYY-MM-DDTHH-MM-SS`. It SHALL read only that
attribute, never the text inside the tag.

#### Scenario: A spawn prompt
- **WHEN** a pi pane sends `user-prompt-submit` with a prompt that starts with `<file name="/s/artifacts/P1/context/unit1-idle-collapse-2026-09-07T15-18-30.md">`
- **THEN** the pane is a spawned subagent with parent session id `P1` and task `unit1 idle collapse`

#### Scenario: A resume prompt
- **WHEN** a pi pane sends `user-prompt-submit` with a prompt that starts with `<file name="/s/artifacts/P1/pmux-resume/u1-placement-2026-10-21T05-41-15.md">`
- **THEN** the pane is a resumed subagent with parent session id `P1` and task `u1 placement`

#### Scenario: Another file prompt
- **WHEN** a pi pane sends a prompt that starts with `<file name="/home/me/notes.md">`
- **THEN** the pane is not marked as a subagent and the prompt is stored as before

#### Scenario: A stem without a timestamp
- **WHEN** the PATH has the spawn shape but its stem does not end with a hyphen and a timestamp
- **THEN** the pane is not marked as a subagent and the prompt is stored as before

#### Scenario: The tag is not at the start
- **WHEN** a prompt holds a matching `<file name="…">` tag after other text
- **THEN** the pane is not marked as a subagent

#### Scenario: Another agent
- **WHEN** a Claude, Codex or OpenCode pane sends a prompt that starts with a matching tag
- **THEN** the pane is not marked as a subagent

### Requirement: the fuzzy task comes from the name

The fuzzy task SHALL be the stem without its hyphen and timestamp, with each
hyphen shown as a space. The stems `subagent` (spawned) and `resume`
(resumed) SHALL give no task, because pmux writes them when the subagent has
no usable name.

#### Scenario: A hyphenated name
- **WHEN** the stem is `review-unit1-2026-09-07T15-26-50`
- **THEN** the task is `review unit1`

#### Scenario: The spawn fallback stem
- **WHEN** a spawn PATH has the stem `subagent-2026-09-07T15-26-50`
- **THEN** the pane is a spawned subagent with no task

#### Scenario: The resume fallback stem
- **WHEN** a resume PATH has the stem `resume-2026-10-21T05-41-15`
- **THEN** the pane is a resumed subagent with no task

### Requirement: a pmux task prompt is not shown

The sidebar SHALL NOT store or show the text of a recognised pmux task prompt
as the pane's prompt. The prompt row keeps what it showed before the prompt
arrived. Response previews SHALL be shown as for any other pi pane.

#### Scenario: A recognised prompt
- **WHEN** a pi pane sends a recognised pmux task prompt
- **THEN** the pane status becomes running and no prompt row shows the tag or its body

#### Scenario: A finished turn of a subagent
- **WHEN** a subagent pane sends `stop` with `last_assistant_message`
- **THEN** the pane shows that message as the response preview

### Requirement: the subagent identity persists for the session

The subagent identity SHALL stay on the pane across later prompts and turns.
A later recognised prompt SHALL replace it. `session-end`, and the clearing of
a pi pane whose pi process is gone, SHALL remove it.

#### Scenario: A later plain prompt
- **WHEN** a subagent pane later sends `user-prompt-submit` with a prompt that is not a recognised pmux task prompt
- **THEN** the pane is still a subagent with the same parent and task

#### Scenario: A later resume prompt
- **WHEN** a spawned subagent pane later sends a recognised resume prompt
- **THEN** the pane is a resumed subagent with the parent and task of that prompt

#### Scenario: The end of the session
- **WHEN** a subagent pane sends `session-end`
- **THEN** the pane is no longer a subagent

#### Scenario: The pi process is gone
- **WHEN** a subagent pane's pi process has exited and the pane's pi state is cleared
- **THEN** the subagent identity is cleared with it

### Requirement: the parent is resolved at each refresh

At each refresh, the sidebar SHALL take as parent the agent pane whose
reported session id equals the recorded parent session id. It SHALL show the
parent by that pane's tmux title, or by its pane id when the title is empty.
When no pane has that session id, it SHALL show `ORPHANED` in the theme's
error colour.

#### Scenario: The parent is live and named
- **WHEN** a subagent records parent session id `P1` and pane `%0` reports session id `P1` with the title `orchestrator`
- **THEN** the subagent shows the parent as `orchestrator`

#### Scenario: The parent has no title
- **WHEN** the parent pane `%0` has an empty title
- **THEN** the subagent shows the parent as `%0`

#### Scenario: The parent is gone
- **WHEN** no pane reports session id `P1`
- **THEN** the subagent shows `ORPHANED` in the error colour

#### Scenario: The parent comes back
- **WHEN** a pane that reports session id `P1` appears after the subagent was shown as orphaned
- **THEN** the next refresh shows that pane as the parent

#### Scenario: The parent is hidden by a filter
- **WHEN** the parent pane is live but the sidebar's repository filter hides its row
- **THEN** the subagent still shows that pane as its parent

### Requirement: a subagent pane carries a badge

The status row of a subagent pane SHALL show the badge ♟ for a spawned
subagent and the badge ↻ for a resumed subagent, in the place where a
permission mode badge is shown for other agents.

#### Scenario: A spawned subagent

- **WHEN** a spawned subagent pane is listed
- **THEN** its status row shows the badge ♟

#### Scenario: A resumed subagent

- **WHEN** a resumed subagent pane is listed
- **THEN** its status row shows the badge ↻

#### Scenario: A pi pane that is not a subagent

- **WHEN** a pi pane without a subagent identity is listed
- **THEN** its status row shows no badge

### Requirement: a subagent pane shows its parent

The pane row of a subagent SHALL show a line directly under the status row
that names the parent (or `ORPHANED`). The line SHALL NOT carry the fuzzy
task: the task names the pmux launch, and the status row's title slot already
shows the pane name pmux derives from the same launch. The line SHALL be cut
to the row width.

#### Scenario: A parent resolves

- **WHEN** a subagent pane whose parent session id resolves to a live parent pane is listed
- **THEN** the line under the status row shows the parent's pane name and no task text

#### Scenario: No parent resolves

- **WHEN** a subagent pane whose parent session id resolves to no live pane is listed
- **THEN** the line under the status row shows `ORPHANED` and no task text

#### Scenario: A narrow sidebar

- **WHEN** the line is wider than the row
- **THEN** the line is cut to the row width and the row count does not change
