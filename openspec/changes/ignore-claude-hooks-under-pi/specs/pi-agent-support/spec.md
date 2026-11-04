# Spec Delta

## ADDED Requirements

### Requirement: Claude events from a process under pi are ignored
The `hook` subcommand SHALL ignore an event for the agent `claude` when the
`PI_CODING_AGENT` environment variable of the command is set and not empty.
pi sets this variable for its child processes, so such an event comes from a
Claude Code process that pi started in its own pane. An ignored event SHALL
change no pane option, SHALL write no activity log entry, SHALL send no
desktop notification, and SHALL exit with status 0.

#### Scenario: A Claude child of pi does not relabel the pi pane
- **GIVEN** a pane whose `@pane_agent` is `pi` and whose `@pane_session_id` is the pi session id
- **WHEN** a `hook claude session-start`, `user-prompt-submit` or `stop` event arrives for that pane with `PI_CODING_AGENT=true` in its environment
- **THEN** `@pane_agent` stays `pi`, `@pane_session_id` stays the pi session id, and the status does not change

#### Scenario: A Claude session end under pi does not clear the pane
- **GIVEN** a running pi pane
- **WHEN** a `hook claude session-end` event arrives for that pane with `PI_CODING_AGENT=true` in its environment
- **THEN** the pane keeps all of its options and its activity log

#### Scenario: A pi subagent on a Claude model nests under its parent
- **GIVEN** a pmux subagent pane whose pi process uses a Claude model through a `claude` child process, and a parent pi pane whose session id matches the subagent's recorded parent
- **WHEN** the `claude` child reports its events with `PI_CODING_AGENT=true` in its environment
- **THEN** the subagent pane stays a pi pane and is drawn nested under its parent

#### Scenario: An empty variable does not ignore the event
- **WHEN** a `hook claude` event arrives with `PI_CODING_AGENT` set to the empty string
- **THEN** the event is processed as a normal Claude event

#### Scenario: Other agents are not affected
- **WHEN** a `hook pi`, `hook codex` or `hook opencode` event arrives with `PI_CODING_AGENT=true` in its environment
- **THEN** the event is processed as before
