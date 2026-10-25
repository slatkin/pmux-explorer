# channel-endpoint Specification

## Purpose
A Claude Code channel server inside each parent session, so that pmux can push a report pointer into the session without typing into the terminal, and can tell when the channel is not there.

## Requirements

### Requirement: MCP channel server mode
`pmux-explorer --mcp` SHALL run a stdio MCP server: one JSON-RPC message per line on stdin and stdout, and logs on stderr only. It SHALL answer `initialize` and `ping`, SHALL answer any other request with error `-32601`, and SHALL ignore unknown notifications. At stdin EOF it SHALL remove its endpoint socket and exit 0.

#### Scenario: Unknown method
- **WHEN** the client sends a request with method `tools/list`
- **THEN** the reply is a JSON-RPC error with code `-32601`, and the server keeps running

#### Scenario: Stdout holds only protocol
- **WHEN** the server runs a full session
- **THEN** every stdout line parses as a JSON-RPC message

#### Scenario: Client exits
- **WHEN** stdin reaches EOF
- **THEN** the endpoint socket file is gone and the exit code is 0

### Requirement: Channel capability and revision
The `initialize` reply SHALL declare `capabilities.experimental["claude/channel"]` as `{}` and no other capability: no `tools`, no `resources`, no `prompts` and no `claude/channel/permission`. It MUST NOT carry `instructions`, so the server adds nothing to the model's context outside an event. Its `protocolVersion` SHALL be the requested revision when it is `2024-11-05`, `2025-03-26` or `2025-06-18`, and `2025-06-18` otherwise. It MUST NEVER be `2026-07-28`.

#### Scenario: Newest revision requested
- **WHEN** `initialize` requests protocol version `2026-07-28`
- **THEN** the reply's `protocolVersion` is `2025-06-18`

#### Scenario: Older revision requested
- **WHEN** `initialize` requests protocol version `2025-03-26`
- **THEN** the reply's `protocolVersion` is `2025-03-26`

#### Scenario: Capabilities
- **WHEN** the server answers `initialize`
- **THEN** the reply's capabilities hold only `experimental["claude/channel"]`, and the reply has no `instructions` key

### Requirement: Arm probe
After `notifications/initialized`, the server SHALL emit one probe event: `notifications/claude/channel` with `meta` `{"kind":"probe","nonce":<32hex>}` and one-line content that asks Claude to run `<exe> channel-ready --nonce <nonce>` and do nothing else. `<exe>` is the shell-quoted absolute path of the running binary, so the command works when `pmux-explorer` is not on `PATH`.

#### Scenario: Probe after initialized
- **WHEN** the client sends `notifications/initialized`
- **THEN** the next stdout line is the probe notification with a fresh nonce

#### Scenario: Probe names the running binary
- **WHEN** the server runs from `/opt/px/pmux-explorer` and is not on `PATH`
- **THEN** the probe content holds `'/opt/px/pmux-explorer' channel-ready --nonce <nonce>`

### Requirement: Arm request
A request `{"v":1,"kind":"arm","nonce":<string>}` SHALL arm the server for the rest of the process when the nonce equals the probe's nonce, and reply `{"ok":true,"armed":true}`. A different nonce MUST NOT arm it, and SHALL get `{"ok":false,"reason":"nonce"}`. An arm request on an armed server with the correct nonce SHALL reply `{"ok":true,"armed":true}` again.

#### Scenario: Correct nonce arms
- **WHEN** an arm request carries the probe's nonce
- **THEN** the reply is `{"ok":true,"armed":true}` and a `status` request reports `armed:true`

#### Scenario: Wrong nonce
- **WHEN** an arm request carries a different nonce
- **THEN** the reply is `{"ok":false,"reason":"nonce"}` and the server stays unarmed

### Requirement: Channel-ready command
`pmux-explorer channel-ready --nonce <n>` SHALL send an arm request to the endpoint of its own pane (`TMUX` and `TMUX_PANE`, by the endpoint path rule), print the reply line on stdout, and exit 0 when the reply has `armed:true`. Otherwise it SHALL exit 1: on a `nonce` reply, on a missing `--nonce`, and when the endpoint is missing or does not answer within 2 seconds, with a message on stderr.

#### Scenario: Armed
- **WHEN** Claude runs the probe's command in its pane and the nonce matches
- **THEN** stdout is `{"ok":true,"armed":true}` and the exit code is 0

#### Scenario: No endpoint
- **WHEN** the command runs in a pane with no endpoint socket
- **THEN** it exits 1 with a message on stderr that names the endpoint path

#### Scenario: Missing nonce
- **WHEN** the command runs without `--nonce`
- **THEN** it exits 1 with a message on stderr naming `--nonce`, and sends no request

### Requirement: Probe repeat
While the server is not armed, a `report` request SHALL cause a new probe with the same nonce, at most once per 60 seconds and at most 3 repeats in the life of the process. An armed server MUST NOT emit a probe.

#### Scenario: Report while unarmed
- **WHEN** the server is unarmed, the last probe was 90 seconds ago, and a `report` request arrives
- **THEN** the server emits one probe and replies `unarmed`

#### Scenario: Probe limit
- **WHEN** two `report` requests arrive 10 seconds apart while the server is unarmed
- **THEN** at most one probe is emitted for the two

#### Scenario: Lifetime cap
- **WHEN** the server has sent the first probe and 3 repeats, stays unarmed, and a report arrives 10 minutes later
- **THEN** no probe is emitted, and the reply is `unarmed`

### Requirement: Endpoint path
The endpoint path SHALL be `<dir>/<h>-<n>.sock`. `<dir>` is `$XDG_RUNTIME_DIR/pmux-channel`, or `/tmp/pmux-channel-<uid>` when `XDG_RUNTIME_DIR` is unset or empty. `<h>` is the 16-digit lowercase hex FNV-1a 64-bit hash of the tmux socket path (the first comma field of `TMUX`). `<n>` is `TMUX_PANE` without its `%`. This rule SHALL match pmux `report-channel`.

#### Scenario: Shared test vector
- **WHEN** `TMUX` is `/tmp/tmux-1000/default,1,0`, `TMUX_PANE` is `%12` and `XDG_RUNTIME_DIR` is `/run/user/1000`
- **THEN** the path is `/run/user/1000/pmux-channel/79301cf9592f327a-12.sock`

#### Scenario: No runtime dir
- **WHEN** `XDG_RUNTIME_DIR` is unset and the uid is 1000
- **THEN** the directory is `/tmp/pmux-channel-1000`

### Requirement: Endpoint ownership
The server SHALL create the directory with mode 0700 and refuse it when another uid owns it, and SHALL create the socket with mode 0600. When the socket file exists and refuses a connection, the server SHALL replace it. When it accepts a connection, the server MUST NOT replace it. With no `TMUX` or `TMUX_PANE`, or after a refusal, the server SHALL run with no endpoint and log the reason to stderr.

#### Scenario: Stale socket
- **WHEN** the socket file exists and no process listens on it
- **THEN** the server removes it and listens on a new socket at the same path

#### Scenario: Live owner
- **WHEN** another live server listens on the path
- **THEN** this server does not bind, logs the reason to stderr, and still answers MCP requests

#### Scenario: Not in tmux
- **WHEN** `TMUX_PANE` is unset
- **THEN** no socket file is created and the server still answers MCP requests

#### Scenario: Foreign directory
- **WHEN** the endpoint directory exists and another uid owns it
- **THEN** the server does not bind and logs the reason to stderr

### Requirement: Endpoint connection
Each connection SHALL carry one request line and one reply line, and then close. The server SHALL close a connection from a peer with another uid with no reply. A request line longer than 4 KiB, or one that does not arrive within 2 seconds, SHALL get `{"ok":false,"reason":"invalid"}`. A request that is not JSON, has `v` other than `1`, or has an unknown `kind` SHALL get the same reply.

#### Scenario: Other user
- **WHEN** a process with another uid connects
- **THEN** the server closes the connection with no reply and emits nothing

#### Scenario: Oversized request
- **WHEN** a request line is longer than 4 KiB
- **THEN** the reply is `{"ok":false,"reason":"invalid"}`

#### Scenario: Unknown kind
- **WHEN** a request is `{"v":1,"kind":"other"}`
- **THEN** the reply is `{"ok":false,"reason":"invalid"}`

### Requirement: Report request
A request `{"v":1,"kind":"report","id":<32hex>,"text":<pointer>,"bytes":<n>,"token":<32hex>}` SHALL be valid only when `id` is 32 lowercase hex characters and `text` is one line of at most 1 KiB that starts with `pmux report <id>. `. When the server is armed, it SHALL emit `notifications/claude/channel` with `content` equal to `text` and `meta` `{"kind":"report","report_id":<id>,"bytes":"<n>"}`, and reply `{"ok":true,"emitted":true}` only after the write is flushed.

#### Scenario: Armed report
- **WHEN** an armed server gets a valid report request
- **THEN** stdout gets exactly one notification whose content is the text, and the reply is `{"ok":true,"emitted":true}`

#### Scenario: Unarmed report
- **WHEN** an unarmed server gets a valid report request
- **THEN** no report notification is emitted, and the reply is `{"ok":false,"reason":"unarmed"}`

#### Scenario: Wrong text shape
- **WHEN** the text holds a line break, or does not start with `pmux report <id>. `
- **THEN** nothing is emitted, and the reply is `{"ok":false,"reason":"invalid"}`

#### Scenario: Write failure
- **WHEN** the stdout write fails
- **THEN** the reply is `{"ok":false,"reason":"write-failed"}`

### Requirement: Opaque report fields
The `token` and `bytes` fields of a report request SHALL be opaque: the server MUST NOT validate them, MUST NOT emit `token`, and SHALL put `bytes` into `meta.bytes` as a string as given, or `""` when it is absent.

#### Scenario: Missing opaque fields
- **WHEN** an armed server gets a valid report request with no `token` and no `bytes`
- **THEN** the event is emitted with `meta.bytes` equal to `""`, and the reply is `{"ok":true,"emitted":true}`

#### Scenario: Token not emitted
- **WHEN** an armed server gets a valid report request with `token` `abc`
- **THEN** the emitted `meta` has no `token` key

### Requirement: Status request
A request `{"v":1,"kind":"status"}` SHALL get `{"ok":true,"armed":<bool>,"pane":<TMUX_PANE>}` and MUST NOT emit anything to Claude.

#### Scenario: Status before the probe answer
- **WHEN** a status request arrives before an arm request with the correct nonce
- **THEN** the reply has `armed:false` and the pane id, and stdout gets no line

### Requirement: Claude plugin registration
The Claude plugin manifest SHALL declare the server inline in `mcpServers` under the name `pmux`, with no `channels` entry, through a wrapper script beside `hook.sh` that finds the binary in the same order as `hook.sh` and runs it with `--mcp`. The session opt-in SHALL be `claude --dangerously-load-development-channels plugin:pmux-explorer@slatkin`. When no binary is found, the wrapper SHALL write a message to stderr and exit 1.

#### Scenario: Manifest shape
- **WHEN** `.claude-plugin/plugin.json` is read
- **THEN** `mcpServers.pmux.command` names `${CLAUDE_PLUGIN_ROOT}/mcp.sh`, and the manifest has no `channels` key

#### Scenario: Binary found
- **WHEN** the plugin's wrapper runs and the binary exists under `target/release/`
- **THEN** it runs that binary with `--mcp`

#### Scenario: No binary
- **WHEN** no binary exists in any lookup location
- **THEN** the wrapper writes a message naming the lookup to stderr and exits 1
