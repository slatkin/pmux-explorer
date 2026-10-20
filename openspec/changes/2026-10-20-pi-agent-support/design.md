# Design

## D1. The fork

`slatkin/tmux-agent-sidebar` is the working repository. `upstream` stays
configured as a remote so that a later upstream change can be pulled in by
hand, but the fork does not track upstream automatically and does not plan to
push anything back. The MIT licence and the upstream attribution stay in
place.

The repository keeps its own conventions: `cargo fmt` before every commit,
`cargo test` and `cargo clippy` as the gates, and one behaviour per commit.

## D2. The transport stays one process per event

The sidebar receives events through `tmux-agent-sidebar hook <agent> <event>`,
which reads one JSON object from stdin and returns. The Claude Code producer
invokes it from a hook, the OpenCode producer invokes it from a plugin. The pi
producer invokes it from a pi extension.

This stays. The handlers behind that entry point already own the pane-option
writes, the activity log, the desktop notifications and the end-of-session
cleanup, and a second transport (a socket, a daemon, a file of events) would
mean a second path to all of that. The cost is one process spawn per event,
which is what the other agents already pay.

## D3. The pi adapter mirrors the OpenCode adapter

The OpenCode adapter is the closest precedent: OpenCode is a plugin-style agent
with no settings file, its tool names are lowercase, and its producer is a
JavaScript file that spawns the sidebar binary per event. The pi adapter takes
the same shape, and the pi producer in pmux takes the same shape as
`.opencode/plugins/tmux-agent-sidebar.js`.

The field names of the protocol are the Claude adapter's field names, because
the handlers read them. The producer translates; the adapter only normalises
what the producer cannot know better, which is the tool name and the tool
argument keys:

| pi tool name | canonical name | pi argument | canonical argument |
| --- | --- | --- | --- |
| `read` | `Read` | `path` | `file_path` |
| `edit` | `Edit` | `path` | `file_path` |
| `write` | `Write` | `path` | `file_path` |
| `bash` | `Bash` | `command` | `command` |
| `grep` | `Grep` | `pattern` | `pattern` |
| `find` | `Glob` | `pattern` | `pattern` |
| `ls` | `Glob` | `path` | `pattern` |
| `powershell` | `PowerShell` | `command` | `command` |

The label extractor in `src/cli/label.rs` keys off the canonical names and the
canonical argument keys, so without this mapping the activity log shows an
empty label for every pi tool. The OpenCode adapter already does the same for
`filePath`.

## D4. The pi event mapping

pi 1.3.0 gives a producer these extension events: `session_start`,
`session_shutdown`, `input`, `tool_call`, `tool_execution_start`,
`tool_execution_end`, `agent_end`, `agent_start`, `turn_start`, `turn_end`.

| pi event | protocol event | payload the producer sends |
| --- | --- | --- |
| `session_start` | `session-start` | `cwd`, `session_id`, `source`, `worktree` |
| `input` | `user-prompt-submit` | `cwd`, `prompt`, `session_id` |
| `tool_execution_start` | `activity-log` | `tool_name`, `tool_input` |
| `tool_execution_end` | `activity-log` | `tool_name`, `tool_input`, `tool_response` |
| `agent_end` | `stop` | `last_assistant_message` |
| `session_shutdown` | `session-end` | `end_reason` |

`tool_execution_start` carries the arguments, so the activity entry is written
when the tool starts rather than when it ends. The entry therefore has no
result to show, which costs the label extractor one input it does not use, and
gains the property that the pane leaves the waiting state as soon as the tool
starts, because the activity handler clears the attention flag and sets the
status to running.

`agent_end` gives the whole message list of the finished run; the producer
sends the text of the last assistant message as `last_assistant_message`, the
same field Claude's `Stop` hook sends.

## D5. What pi cannot report, and what the sidebar therefore cannot show

Three sidebar features have no pi source, and the change says so rather than
faking them.

The attention badge. The sidebar marks a pane as needing a human when the
agent asks for permission, which for Claude arrives as the `Notification` hook.
pi 1.3.0 has no approval event. Permission in pi is a concept an extension
implements, not a built-in prompt, and the built-in tools run with the
permissions of the pi process. A pi pane therefore shows running, idle and
error, and never the attention badge. The one source that exists is the
`tool_call` event, and the ordering makes it unusable: a permission extension
is itself a `tool_call` handler, so a producer that subscribes to `tool_call`
either runs before the gate and cannot see the decision, or after it and sees
nothing at all while the dialog is open. Reading the pane's screen is the
other candidate and is a screen scrape. Both are worse than an honest gap.

The task-progress row. The sidebar reads it from Claude's `TaskCreate` and
`TaskUpdate` tools. pi has no task tool; its built-in tools are `read`,
`write`, `edit`, `bash`, `powershell`, `ls`, `find` and `grep`. There is
nothing to read.

The subagent tree. pi has no in-process subagents. pmux runs child agents in
their own panes, and each of those panes is a pane of its own in the sidebar,
which is the same information in a shape the sidebar already renders.

## D6. The dead-pane sweep covers pi

The sweep in `src/tmux/query.rs` clears the pane options of an agent that
exited without saying so. It applies to Codex and OpenCode and not to Claude,
because Claude's `SessionEnd` hook does the cleanup. pi's producer sends
`session-end` from `session_shutdown`, so by the same argument pi could be
excluded. It is included anyway: a pi process can be killed, and the pane
falls back to the shell with stale options. The sweep's process check compares
the process name against the agent label, and the pi process is called `pi`, so
the check works with no change.

## D7. The permission mode stays empty for pi

The sidebar reads `@pane_permission_mode` for Claude alone, because the badge
that shows it is a Claude concept. pi has no permission mode that the sidebar
can read, so the pi adapter reports the default and the badge stays off. The
`plan` badge is not extended to pi in this change.

## D8. The setup command prints a contract for a plugin-style agent

`tmux-agent-sidebar setup <agent>` merges hook registrations into a settings
file for an agent that has one. pi has no settings file and no hook
registrations, so `setup pi` prints the producer contract: the binary path,
the event names, the payload fields, and the name of the extension that
implements it. The same treatment applies to OpenCode, which already has no
setup command, and both share one code path.

## D9. The producer lives in pmux

pmux is installed as a pi package on this machine, so its extension loads in
every pi session, not only in the sessions pmux started. That is the property
the sidebar needs: it shows every agent pane in every tmux session, including
pi panes that a human started by hand. A producer that lives in pmux's
supervisor would see only pmux-spawned children.

The producer is a change in the pmux repository (`2026-10-20-sidebar-events`).
The contract is this repository's, and the pmux change is written against
`docs/agent-protocol.md`. Neither repository depends on the other's build.

## D10. The protocol document

`docs/agent-protocol.md` states, per event, the JSON fields, their meaning,
whether they are required, and what the sidebar does with them. It is the
specification the pmux producer is written against, and it replaces the
knowledge that today lives only in the Claude adapter's `parse` arms.
