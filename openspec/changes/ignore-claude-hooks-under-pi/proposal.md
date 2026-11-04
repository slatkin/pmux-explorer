# Proposal

## Why

pi can use Claude models through the `pi-claude-bridge` extension. The bridge
runs a `claude` process (through the Claude Agent SDK) as a child of pi, and it
starts a new one on each pi turn. That child inherits pi's `$TMUX_PANE` and
loads the user's Claude Code plugins, so the pmux-explorer Claude hooks run
inside it and write to the pi pane:

- `@pane_agent` becomes `claude`, so the pane shows as a Claude pane.
- `@pane_session_id` becomes the Claude Code session id, so the subagents of
  that pane cannot find their parent.
- A pmux subagent on the bridge is not nested, because the sidebar keeps the
  subagent identity only for pi panes.
- The Claude `session-end` at the end of each turn clears the pane until the
  next pi event writes it again.

The pi producer already reports every event of that pane. The Claude events
only clobber it.

## What Changes

- The `hook` subcommand ignores an event for the agent `claude` when the
  `PI_CODING_AGENT` environment variable is set and is not empty. pi sets this
  variable for every child process, the bridge's `claude` child included. The
  command exits 0 and writes nothing: no pane option, no activity log entry,
  no desktop notification.
- The events of all other agents, `pi` included, are processed as before.
- `docs/agent-protocol.md` states the rule.

Out of scope:

- A badge that shows the Claude backend of a pi pane.
- The tool calls that Claude Code makes inside the legacy AskClaude tool. They
  stay invisible; the sidebar shows only the AskClaude call that pi reports.
- Codex or OpenCode processes that pi starts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pi-agent-support`: adds the requirement that Claude hook events that come
  from a process under pi do not change the pi pane.

## Impact

- `src/cli/hook.rs`: one early exit in `cmd_hook`, and its unit tests.
- `docs/agent-protocol.md`: one paragraph.
- No change to `hooks/hooks.json`, to the adapters, or to the pane state rules.
- A Claude Code session that the user starts by hand from a pi bash tool is
  also not tracked. This is intended: it shares the pi pane.
