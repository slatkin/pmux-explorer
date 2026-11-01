# Spec Delta

## Purpose

Defines how the state of an agent pane is written: which events may change
which pane options, in what order of precedence, and how one event's changes
reach tmux. Hooks and the sidebar's own sweeps follow the same rules.

## ADDED Requirements

### Requirement: one event is applied as one batch

The sidebar SHALL apply all pane option changes that one producer event causes
as a single tmux command, so that the sidebar's poll reads the pane state from
before the event or from after it, and never from between.

#### Scenario: A stop event changes several options
- **WHEN** a `stop` event sets the status, the response preview and the prompt source, and clears the run state
- **THEN** all of those changes reach tmux in one command

#### Scenario: The poll runs during an event
- **WHEN** the sidebar polls while a producer event is being applied
- **THEN** the polled pane shows the state from before the event or from after it, not a mix

### Requirement: a finished pane stays finished

An `activity-log` event SHALL change the pane status from `waiting` or `error`
to `running`, because a tool call after a permission grant is the only sign
that the wait ended. It SHALL NOT change a pane status of `idle` or
`background`, and it SHALL NOT change a status that is absent.

#### Scenario: A tool call ends a wait
- **WHEN** a pane has status `waiting` and an `activity-log` event arrives
- **THEN** the status becomes `running`, the attention flag is cleared and the wait reason is cleared

#### Scenario: A late tool event after stop
- **WHEN** a pane has status `idle` after a `stop` event and a delayed `activity-log` event arrives
- **THEN** the status stays `idle` and the start time stays unset

#### Scenario: A late tool event while a background shell lives
- **WHEN** a pane has status `background` and an `activity-log` event arrives
- **THEN** the status stays `background`

#### Scenario: A tool call during a run
- **WHEN** a pane has status `running` and an `activity-log` event arrives
- **THEN** the status stays `running` and the activity entry is still logged

#### Scenario: A tool call after an error
- **WHEN** a pane has status `error` and an `activity-log` event arrives
- **THEN** the status becomes `running`

### Requirement: a field that is not reported does not clear

A producer event SHALL clear a pane option only when that event is declared to
report the field behind the option. When the event reports the field and the
value is empty, the option SHALL be cleared. When the event does not report
the field, the option SHALL keep its value. This SHALL hold for every agent.

#### Scenario: A pi stop keeps the worktree
- **WHEN** a pi pane has worktree name and branch options and a pi `stop` event arrives without a `worktree` field
- **THEN** the worktree name and branch options keep their values

#### Scenario: A reporting event leaves the worktree
- **WHEN** a Claude event that reports `worktree` arrives with no worktree
- **THEN** the worktree name and branch options are cleared

#### Scenario: A reporting event with an empty session id
- **WHEN** an event that reports `session_id` arrives with an empty one
- **THEN** the session id option is cleared

#### Scenario: A payload that is not valid JSON
- **WHEN** a producer event arrives whose payload is not valid JSON
- **THEN** no field counts as reported and no pane option is cleared

### Requirement: each adapter declares the fields an event reports

Each agent adapter SHALL declare, for every event, the payload fields that
event reports. For pi, the declaration SHALL be the field list that the
producer contract prints. For Claude, Codex and OpenCode, every event that
carries session context SHALL report `session_id`, `cwd` and `worktree`, as
those agents do today. A field that is absent from a payload but reported by
the event counts as empty.

#### Scenario: The printed contract names reported fields
- **WHEN** `pmux-explorer setup pi` runs
- **THEN** the contract lists, for each event, the fields the event reports

#### Scenario: A field is added to an event
- **WHEN** a field is added to an event's declaration
- **THEN** the event clears the matching option when the field is empty, and the printed contract lists the field

### Requirement: a parent's identity survives its subagents

While a pane has active subagents, an event SHALL NOT change the pane's agent
label, permission mode, session id, working directory or worktree options,
and SHALL NOT reset the pane's task state or end its session.

#### Scenario: A child event with another session id
- **WHEN** a pane has active subagents and an event arrives with a different `session_id`
- **THEN** the pane keeps its session id

#### Scenario: A child event with another worktree
- **WHEN** a pane has active subagents and an event arrives with a different worktree
- **THEN** the pane keeps its worktree name, branch and working directory

#### Scenario: A child ends its session
- **WHEN** a pane has active subagents and a `session-end` event arrives
- **THEN** the pane state is kept

#### Scenario: The last subagent stops
- **WHEN** the last active subagent of a pane stops
- **THEN** the next event updates the identity options again

### Requirement: a written value is always one field

The sidebar SHALL encode every value it writes to a pane option so that the
value reads back as a single field of a single record. A newline or a field
separator in a value SHALL NOT split the sidebar's pane list.

#### Scenario: A wait reason with a newline
- **WHEN** a `notification` event carries a `wait_reason` that holds a newline
- **THEN** the pane stays in the pane list and shows the reason on one line

#### Scenario: A prompt with a separator
- **WHEN** a `user-prompt-submit` event carries a prompt that holds a field separator
- **THEN** the pane stays in the pane list and shows the full prompt text

#### Scenario: A stop failure error with a newline
- **WHEN** a `stop-failure` event carries an `error` that holds a newline
- **THEN** the pane stays in the pane list with status `error`

### Requirement: the sidebar's own sweeps follow the same rules

When the sidebar changes an agent pane option itself, it SHALL apply the same
rules as a hook event. The background-shell sweep SHALL clear the background
command of a pane whose shell has ended, and SHALL move the status from
`background` to `idle` only when the status is `background`.

#### Scenario: A background shell ends
- **WHEN** a pane has status `background` and its background shell process is gone
- **THEN** the background command is cleared and the status becomes `idle`

#### Scenario: A background shell ends during a run
- **WHEN** a pane has status `running` and its background shell process is gone
- **THEN** the background command is cleared and the status stays `running`

#### Scenario: A dead pane is wiped
- **WHEN** the sidebar finds a Codex, OpenCode or pi pane whose agent process is gone
- **THEN** the agent state options of that pane are cleared in one command
