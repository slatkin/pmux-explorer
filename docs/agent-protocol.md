# The Agent Event Protocol

The sidebar learns about an agent through one channel: a producer that runs a
command and hands it a JSON object. This document is that channel's
specification. A producer may be a hook of an agent that supports hooks, a
plugin of an agent that supports plugins, or an extension of an agent that
supports extensions. The sidebar does not care which.

## The call

```sh
tmux-agent-sidebar hook <agent> <event>
```

The command reads one JSON object from standard input and writes nothing. It
exits 0 whether or not it understood the event, and a producer never has to
read its output.

The `<agent>` argument names the agent the pane runs. The sidebar uses it to
choose how to read the payload, to colour the pane and to label it. The
agents it knows are `claude`, `codex`, `opencode` and `pi`.

The `<event>` argument names what happened. The event names are the ones
listed below, with a hyphen between the words: `session-start`,
`user-prompt-submit`, `activity-log` and so on.

**The pane is taken from the environment.** The sidebar reads `TMUX_PANE` from
the command's own environment to know which pane the event is about. A
producer must therefore run the command in the environment of the pane it
describes, which is the environment of the agent process itself. A producer
that runs outside tmux, or that spawns the command with a cleaned
environment, describes nothing.

**A producer must not wait for the answer.** The command is cheap, but it
starts a process, and the process may be missing. Producers spawn it, write
the payload, close the pipe and move on. A producer that awaits the exit code
delays the agent's turn for a sidebar.

**A missing command is not an error.** If the binary is not installed, the
producer does nothing. It does not warn and it does not retry.

## The payload

Every payload is a JSON object. The fields below are the ones the sidebar
reads. A field that a producer cannot fill is omitted or sent empty; the
sidebar treats an empty string as "not reported".

### `session-start`

The agent started, or resumed.

| Field | Meaning |
| --- | --- |
| `cwd` | The working directory of the agent. |
| `session_id` | The agent's own identifier for this session. The sidebar uses it to find the session's name and its transcript. |
| `source` | Why the session started. Free text. |
| `permission_mode` | The agent's permission mode, if it has one. Only Claude reports it, and only Claude shows the badge. |
| `agent_id` | The identifier of a subagent, when the event is about one. |
| `worktree` | An object with `name`, `path`, `branch` and `originalRepoDir`, when the agent works in a worktree. |

### `session-end`

The agent's session ended.

| Field | Meaning |
| --- | --- |
| `end_reason` | Why it ended. Free text. |

### `user-prompt-submit`

The agent received a prompt and starts a turn. An empty `prompt` means the
turn continues without a new prompt; the sidebar then returns the pane to
running without touching the prompt text it shows.

| Field | Meaning |
| --- | --- |
| `prompt` | The text the user submitted. |
| `cwd` | The working directory. |
| `session_id` | The session identifier. |
| `permission_mode` | The agent's permission mode. |
| `worktree` | The worktree object, if any. |

### `notification`

The agent waits for a human. The sidebar marks the pane as waiting, sets its
attention flag and raises a desktop notification.

| Field | Meaning |
| --- | --- |
| `notification_type` | Why the agent waits. Claude sends `permission`; pi sends the kind of prompt it is showing. The value is shown next to the pane. |

`wait_reason` is accepted as an alias for `notification_type`, because that is
the name the OpenCode producer already uses.

### `stop`

The turn ended. The pane becomes idle, and the response preview is the last
assistant message.

| Field | Meaning |
| --- | --- |
| `last_assistant_message` | The text of the last assistant message of the turn. |
| `cwd` | The working directory. |
| `session_id` | The session identifier. |
| `permission_mode` | The agent's permission mode. |
| `worktree` | The worktree object, if any. |

### `stop-failure`

The turn ended in an error. The pane becomes an error pane.

| Field | Meaning |
| --- | --- |
| `error` | The error text. `error_type`, `error_message` and `error_details` are accepted as alternatives. |
| `cwd` | The working directory. |
| `session_id` | The session identifier. |

### `activity-log`

A tool ran, or started. The sidebar appends an entry to the pane's activity
log, and the entry's label comes from the tool name and its arguments.

| Field | Meaning |
| --- | --- |
| `tool_name` | The tool's name, in the canonical vocabulary below. A name outside the vocabulary is kept as it is and gets an empty label. |
| `tool_input` | The tool's arguments, as an object. |
| `tool_response` | The tool's result, as an object or a string. Optional. |

The canonical tool names are `Bash`, `Read`, `Edit`, `Write`, `NotebookEdit`,
`PowerShell`, `Monitor`, `PushNotification`, `Glob`, `Grep`, `WebFetch`,
`WebSearch`, `ToolSearch`, `Skill`, `SendMessage`, `TeamCreate`, `Lsp`,
`CronCreate`, `CronDelete`, `EnterWorktree`, `ExitWorktree`, `Agent`,
`TaskCreate`, `TaskUpdate`, `TaskGet`, `TaskStop`, `TaskOutput`,
`AskUserQuestion` and `TodoWrite`.

The label of an entry is read from the arguments by the tool: the file path
for `Read`, `Edit` and `Write`, the command for `Bash`, the pattern for
`Glob` and `Grep`. The argument keys the sidebar reads are `file_path`,
`notebook_path`, `command`, `message`, `pattern`, `url`, `query`, `skill`,
`to`, `team_name`, `task_id` and `task_subject`. A producer whose agent
names those arguments differently sends the canonical key beside its own, and
the adapter rewrites the rest.

### `permission-denied`

The agent was refused a tool it asked for. The pane waits, and the reason is
`permission_denied`.

### `cwd-changed`

The agent moved to another directory. The sidebar follows it.

### `subagent-start` and `subagent-stop`

A subagent started or finished. The sidebar draws a subagent tree under the
pane.

| Field | Meaning |
| --- | --- |
| `agent_type` | The subagent's type, for example `Explore`. Required; the event is ignored without it. |
| `agent_id` | The subagent's identifier. |
| `last_assistant_message` | The subagent's last message, on `subagent-stop`. |
| `agent_transcript_path` | Where the subagent's transcript is. |

### `task-created` and `task-completed`

A task was created or completed. The sidebar draws the task progress of the
pane.

| Field | Meaning |
| --- | --- |
| `task_id` | The task's identifier. |
| `task_subject` | The task's subject. |

### `teammate-idle`

A teammate of an agent team went idle.

| Field | Meaning |
| --- | --- |
| `teammate_name` | The teammate's name. |
| `team_name` | The team's name. |
| `idle_reason` | Why it went idle. |

## The pane state the sidebar keeps

The events above are the only input. The sidebar turns them into tmux pane
options, and the TUI reads those options once a second. A producer never
writes a tmux option itself.

| Tmux option | Written by |
| --- | --- |
| `@pane_agent` | The first event of a session. |
| `@pane_status` | Every event. One of `running`, `background`, `waiting`, `idle`, `error`. |
| `@pane_attention` | Set when the agent waits for a human, cleared when the turn moves on. |
| `@pane_wait_reason` | The reason the agent waits. |
| `@pane_prompt` | The prompt, then the response preview. |
| `@pane_prompt_source` | `user` or `response`, so the sidebar knows which of the two it shows. |
| `@pane_cwd` | The working directory. |
| `@pane_session_id` | The session identifier. |
| `@pane_permission_mode` | Claude only. |
| `@pane_worktree_name`, `@pane_worktree_branch` | The worktree. |
| `@pane_subagents` | The subagent tree. |

The activity log is a file next to those options, one per pane, written by
the same command.

## What a producer cannot report

The sidebar's features are not all available to every agent, because the
agents do not all report the same things. A producer that cannot report
something leaves it out, and the sidebar shows less rather than guessing.

- The permission mode badge is a Claude feature. pi has no permission mode
  the sidebar can read.
- The task progress needs a task tool. Claude has one, pi does not.
- The subagent tree needs subagents inside the agent's process. pmux runs
  child agents in their own panes, and the sidebar shows each of those panes
  on its own.
- The attention badge needs a signal that the agent waits for a human. Claude
  has one in its hooks. pi has one in its `ui_prompt` events, which a
  producer turns into `notification` and then into an empty
  `user-prompt-submit`.
