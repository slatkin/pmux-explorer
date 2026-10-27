# Spec Delta

## REMOVED Requirements

### Requirement: Arm probe
**Reason**: The server must not spend a model turn on arming; the
user's orchestration prompt owns the arm decision (design decision D1).

### Requirement: Probe repeat
**Reason**: No probe exists to repeat; an unarmed report is answered
`unarmed` and nothing more (design decision D1).

### Requirement: Arm request
**Reason**: The arm carries no credential now — its nonce scenarios do
not apply. Re-added as `Channel arming` (a delta cannot drop scenarios
in place).

### Requirement: Channel-ready command
**Reason**: The command takes no arguments now — its `--nonce`
scenarios do not apply. Re-added as `The channel-ready command` (a
delta cannot drop scenarios in place).

### Requirement: Status request
**Reason**: Its scenario names the arm probe, which no longer exists —
re-added as `Endpoint status` (a delta cannot rename a scenario in
place).

## ADDED Requirements

### Requirement: Channel arming
A request `{"v":1,"kind":"arm"}` SHALL arm the server for the rest of
the process and reply `{"ok":true,"armed":true}`. The request carries no
credential: the peer-uid gate is the access control. Fields the handler
does not read — a `nonce` sent by an older caller — SHALL be ignored, so
an older `arm` request still arms.

#### Scenario: Arm request
- **WHEN** an arm request arrives on an unarmed server
- **THEN** the reply is `{"ok":true,"armed":true}` and a `status`
  request reports `armed:true`

#### Scenario: Arm with a stale nonce
- **WHEN** an arm request carries a `nonce` field
- **THEN** it arms like any other arm request

#### Scenario: Re-arm
- **WHEN** an arm request arrives on an armed server
- **THEN** the reply is `{"ok":true,"armed":true}` again

### Requirement: The channel-ready command
`pmux-explorer channel-ready` SHALL send an arm request to the endpoint
of its own pane (`TMUX` and `TMUX_PANE`, by the endpoint path rule),
print the reply line on stdout, and exit 0 when the reply has
`armed:true`. The command takes no arguments; arguments an older prompt
may still pass, such as `--nonce <n>`, SHALL be ignored. Otherwise it
SHALL exit 1 with a message on stderr: on a reply that is not armed, and
when the endpoint is missing or does not answer within 2 seconds.

#### Scenario: Armed
- **WHEN** Claude runs `channel-ready` in its pane
- **THEN** stdout is `{"ok":true,"armed":true}` and the exit code is 0

#### Scenario: No endpoint
- **WHEN** the command runs in a pane with no endpoint socket
- **THEN** it exits 1 with a message on stderr that names the endpoint
  path

### Requirement: Endpoint status
A request `{"v":1,"kind":"status"}` SHALL get
`{"ok":true,"armed":<bool>,"pane":<TMUX_PANE>}` and MUST NOT emit
anything to Claude.

#### Scenario: Status before the arm request
- **WHEN** a status request arrives before any arm request
- **THEN** the reply has `armed:false` and the pane id, and stdout gets
  no line
