# Design

## Context

The binary has two modes today: the TUI and the word subcommands in
`src/cli/mod.rs` (`hook`, `setup`, `task-set`, …). It has `serde_json`
and `libc`, and no async runtime. The Claude plugin manifest
(`.claude-plugin/plugin.json`) holds metadata only, and the hooks live in
`hooks/hooks.json` (pinned by `tests/plugin_hooks_tests.rs`). The plugin
cache holds
no binary, so `hook.sh` looks for it in `bin/`, `target/release/`, the
TPM directory and then `PATH`.

The Channels contract (code.claude.com/docs/en/channels,
channels-reference and mcp, read 2026-10-25) sets these constraints:

- Claude Code spawns a channel server over stdio. The server declares
  `experimental["claude/channel"]` and emits
  `notifications/claude/channel` with `content` and `meta`. Meta keys
  must be identifiers, and other keys are dropped silently.
- Claude Code does not acknowledge an event, and it drops the event
  silently when the session did not register the channel.
- On the v2 client runtime, a server that negotiates `2026-07-28` is not
  registered as a channel.
- A custom channel needs `--dangerously-load-development-channels`
  during the preview. The flag needs an interactive session.

pmux `channel-report-wake` is the client. It owns the request line, the
reply classes and the fallback. This design covers the server side.

## Goals / Non-Goals

**Goals:**
- A report pointer reaches the open session as a channel event, and
  the sender gets a synchronous answer.
- The sender can tell "no channel" from "emitted".

**Non-Goals:**
- Any change to the TUI, the hook protocol or the tmux options.
- Delivery to a session that is closed.

## Decisions

### D1. Hand-written JSON-RPC on `std::thread`

The server answers two methods (`initialize`, `ping`) and emits one
notification kind. An MCP crate would add an async runtime and a
dependency tree to a binary that has neither. Three threads cover the
work: the stdin loop (requests), the socket accept loop (reports, status
and arm), and the main thread that joins
them. One `Arc<Mutex<Stdout>>` writer serializes every stdout line, so a
channel notification never interleaves with a reply. Each write is one
line and a flush.

- Alternative: the `rmcp` crate with tokio. Rejected for its size, and
  because the pinned-version policy in `Cargo.toml` would need a large
  new pinned set.

### D2. Revision pinned below `2026-07-28`

The server answers with the client's revision when it is one of the
three it supports, else `2025-06-18`. It never answers `2026-07-28`, so
Claude Code keeps it on the earlier handshake, where channels register.
The user does not need `MCP_PROTOCOL_NEGOTIATION=legacy`.

### D3. Arm probe, and a repeat while unarmed

The probe proves the full path once per process: the session flag, the
org policy, the provider, and that the model reads the events. The probe
goes after `notifications/initialized`. If Claude Code registers the
channel listener later than that, the first probe is lost. Therefore a
`report` request on an unarmed server sends the probe again, at most
once per 60 s and at most 3 repeats per process. That report still gets
`unarmed` and takes the tmux fallback, and a later report finds the
server armed. The cap bounds the cost in a flagged session whose model
never runs the command: at most 4 probe turns per process, not one per
minute. A session without the flag drops every probe at no cost.

The answer to the probe is a CLI command, not an MCP tool:
`<exe> channel-ready --nonce <n>` sends an `arm` request over the same
socket. The probe names the absolute path of the running binary, because
the plugin binary is often not on `PATH`. The command runs in the parent's
pane, so its own `TMUX_PANE` finds the right endpoint.

- Alternative: an MCP tool `pmux_channel_ready`. A tool definition, and
  the `instructions` that would explain it, sit in the context of every
  turn of every session that loads the plugin, flagged or not
  (estimated 100–160 tokens per turn). The CLI form costs nothing outside
  the probe turn, and it follows the `ack` pattern that the parent
  already uses. Its cost is one bash call in the probe turn, and possibly
  one permission prompt until the user allows the command. Rejected.
- Alternative: arm on the first `emitted` report. The server cannot
  know that the event arrived, so this would arm a session that drops
  events. Rejected.
- The nonce is 16 bytes from `/dev/urandom`, as 32 hex characters. No
  new crate is needed.

### D4. The endpoint is a Unix socket keyed by tmux socket and pane

The path rule is the same as in pmux `report-channel`. The FNV-1a 64
hash fits the 108-byte socket path limit and keeps two tmux servers
apart. The hash is a few lines in both languages, and both pin the
vector `79301cf9592f327a-12.sock`.

At bind time, a refused connect means a stale file from a crashed
server, and the server replaces it. A connect that succeeds means a live
owner, and the server does not take over. Two Claude processes in one
pane cannot both run in the foreground, so a live owner is a leftover
that will exit.

### D5. Same-user gate and shape check, not a sender allowlist

The Channels docs warn that an ungated channel is a prompt-injection
path. The only senders are local processes of the same user, and such a
process can already type into the pane. So the gate is the peer uid
(`SO_PEERCRED` on Linux, `getpeereid` on macOS, both through `libc`),
the 0700 directory and the 0600 socket, plus the pointer shape check. The
server sends `text` as it got it, after the check. It does not rebuild
the text, so the pointer wording has one owner (pmux).

### D6. Plugin wiring through `mcp.sh`

`mcpServers.pmux.command` is `${CLAUDE_PLUGIN_ROOT}/mcp.sh`. The script
copies the `hook.sh` lookup and runs `exec "$BIN" --mcp`. Unlike
`hook.sh`, it exits 1 with a message when the binary is missing, because
a failed MCP server shows in `/mcp`, while a silent exit 0 would look
like a server that closed. The last lookup step is a `command -v
pmux-explorer` check, so "no binary" is detected there and not left to
a bare `PATH` name that fails later.

The load path is pinned by the docs and by the official channel
plugins:

- The channels page says that the development flag takes the
  `plugin:<name>@<marketplace>` form, and channels-reference ("Package
  as a plugin") says a channel published to your own marketplace runs
  with `--dangerously-load-development-channels`.
- The official channel plugins `fakechat` and `telegram`
  (`anthropics/claude-plugins-official`, `external_plugins/`) declare
  only `mcpServers` and no `channels` entry, and the docs start them
  with `--channels plugin:fakechat@claude-plugins-official`.
- The manifest `channels` field binds a configuration dialog
  (`userConfig`) to a server. The server needs no configuration, so the
  manifest has no `channels` entry.

So the opt-in is `claude --dangerously-load-development-channels
plugin:pmux-explorer@slatkin` (the marketplace in
`.claude-plugin/marketplace.json` is `slatkin`).

The server is inline in `plugin.json`, not in a root `.mcp.json` as
the official plugins do it. The plugin root of this repository is the
checkout, and a root `.mcp.json` would also load as a project MCP
config in every Claude session that runs inside this checkout. `--mcp` is a flag, not a word subcommand,
because the request names it so and it is a mode like the TUI, not a
one-shot command.

## Risks / Trade-offs

- [Channels is a research preview and can change its contract] → the
  server only adds a path. When it fails to arm, pmux falls back to
  `send-keys`, so a break degrades to today's behavior.
- [The arm probe costs one model turn when a flagged session starts]
  → only flagged sessions see it.
- [The model can ignore the probe] → the server stays unarmed and pmux
  uses the fallback. The repeat probe gives it another chance when a
  report comes.
- [A crash leaves a stale socket] → the next server replaces it, and
  pmux reads a refused connect as `refused` and falls back.
- [The v2 client runtime can send handshake methods outside the
  answered set, for example a discovery request] → the server answers
  `-32601`, which a JSON-RPC client reads as "not supported", and the
  docs say that a server without the new revision connects on the
  earlier handshake. If the server still fails to register, it never
  arms, and pmux falls back.
- [The stdout mutex blocks a report while a large reply is written]
  → the replies are small, and the client waits up to 2 s.

## Migration Plan

- New mode only. No existing behavior changes.
- This change lands before pmux `channel-report-wake`. A release with
  the new `mcp.sh` and `plugin.json` is needed before the user can start
  a session with the flag.
- Rollback: remove the `mcpServers` entry. The binary mode is then
  unused.
- Live check (the user runs it, outside `tasks.md`): start a flagged
  session in a tmux pane. Check that `/mcp` lists `pmux` as connected,
  that the probe turn runs `channel-ready`, and that a `status`
  request replies `armed:true`. A session without the flag replies
  `armed:false`.
