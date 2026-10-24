# The Agent Event Protocol

The sidebar learns about an agent through one channel: a producer that runs a
command and hands it a JSON object. This document is that channel's
specification. A producer may be a hook of an agent that supports hooks, a
plugin of an agent that supports plugins, or an extension of an agent that
supports extensions. The sidebar does not care which.

## The call

```sh
pmux-explorer hook <agent> <event>
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
| `permission_mode` | The agent's permission mode, if it has one. Only Claude reports it, and only Claude shows the permission-mode badge. pi reports none. The badge slot is shared: a pmux subagent pane shows a `subagent`/`resumed` badge there instead. |
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

### A pmux task prompt

pmux starts a pi subagent with its task written as a prompt file. The prompt
starts with a `<file name="...">` tag, and the path inside the tag has one of
two shapes:

- spawn: `<sessionDir>/artifacts/<parent session id>/context/<name>-<timestamp>.md`
- resume: `<sessionDir>/artifacts/<parent session id>/pmux-resume/<name>-<timestamp>.md`

When a pi pane sends a `user-prompt-submit` prompt that starts with a tag of
that shape, the sidebar reads only the `name` attribute. The body of the file
is never read. The prompt is not stored as the pane prompt, so the prompt row
keeps what it showed before. Instead the sidebar writes three options:
`@pane_subagent_kind` (`spawned` or `resumed`), `@pane_subagent_parent` (the
parent session id) and `@pane_subagent_task` (the fuzzy task, unset when
there is none). The task is the `<name>` part of the file name, that is the
stem without its `-YYYY-MM-DDTHH-MM-SS` timestamp, with each hyphen turned
into a space, so `unit1-idle-collapse` gives `unit1 idle collapse`. The task
is stored, not drawn: the pane name the status row shows already carries the
same launch name, so the identity row below the status row names only the
parent (or `ORPHANED`).
pmux writes the fallback stems `subagent` (a spawned subagent) and `resume`
(a resumed subagent) when it has no usable name, and those give no task at
all. A later recognised prompt replaces them. The identity stays
until the session ends or the pane's pi state is cleared.

A prompt that starts with a `<file name="...">` tag of any other shape, or a
prompt from another agent, is stored and shown as before.

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

These events report an agent's task tool. An agent without a task tool
sends neither, and its task progress comes to the sidebar through the task
command below instead.

### `teammate-idle`

A teammate of an agent team went idle.

| Field | Meaning |
| --- | --- |
| `teammate_name` | The teammate's name. |
| `team_name` | The team's name. |
| `idle_reason` | Why it went idle. |

## The task command

The bottom panel carries a Tasks tab beside Activity and Git that shows the
todo list of the focused pane. The pane's agent writes that list through a
shell subcommand rather than an event. This is the one write path every
agent can use, because every agent can run a shell command, and it needs no
hook registration.

```sh
pmux-explorer task-set <subject>
pmux-explorer task-update <id> <status>
```

`task-set` records one item as `pending` and prints the id the sidebar
assigns it, so the agent can name the item in a later `task-update`.
`task-update` moves one item to `pending`, `in_progress` or `completed`;
an unknown id or an unknown status changes nothing.

**The pane is taken from the environment.** The task command reads
`TMUX_PANE` the way the hook command does, so it must run in the
environment of the pane it describes.

**The command never breaks the turn.** It exits 0 on every path, including
a missing `TMUX_PANE`, where it records nothing.

The list is one JSON file per pane,
`/tmp/pmux-explorer-tasks<PANE>.json`, written beside the activity log. Each
item holds an `id`, a `subject` and a `status`, and the pane id is encoded
in the file name the way the activity log encodes it. The store keeps at
most 50 items and trims completed ones oldest first; when no completed
item remains, the oldest item is dropped instead. A `session-start`
event clears the pane's store, so a pane reused for new work starts empty.

### The task reminders

The task write path carries two short reminders, one per empty-list
condition. The wording of both is defined exactly once in the module
`src/task_reminders.rs`, and every surface that tells an agent about the
reminders quotes it from there verbatim: the command's help output, this
document and the pmux extension's delivery. A reminder fires only in its
own condition; a turn whose list already holds items at its start receives
none.

- A turn that starts while the pane's todo list is empty receives, verbatim
  from `RECORD_FIRST_REMINDER`:

  `No tasks recorded yet: before starting work, record your todo list with task-set <subject>.`

- A turn that ends while items are still open receives, verbatim from
  `UPDATE_BEFORE_FINISH_REMINDER`:

  `Reminder: make sure the task board is up to date.`

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
| `@pane_subagent_kind` | A recognised pmux task prompt (pi only). `spawned` or `resumed`. |
| `@pane_subagent_parent` | A recognised pmux task prompt. The parent session id. |
| `@pane_subagent_task` | A recognised pmux task prompt. The fuzzy task, unset when there is none. |

The activity log is a file next to those options, one per pane, written by
the same command. The task store of the task command is another file next
to those options, one per pane, and it feeds the Tasks tab.

## What a producer cannot report

The sidebar's features are not all available to every agent, because the
agents do not all report the same things. A producer that cannot report
something leaves it out, and the sidebar shows less rather than guessing.

- The permission mode badge is a Claude feature. pi has no permission mode
  the sidebar can read. A pmux subagent pane is still marked with a
  `subagent`/`resumed` badge in that slot, which is not a permission mode.
- The task progress events need a task tool. Claude has one; pi does not,
  so the pi producer reports no tasks and sends neither event. pi still
  keeps a todo list through the task command of the section above, and the
  Tasks tab shows it like any other pane's.
- The subagent tree needs subagents inside the agent's process. pmux runs
  child agents in their own panes, and the sidebar shows each of those panes
  on its own. A pmux subagent pane is marked as a subagent instead, with its
  parent session and its task (see "A pmux task prompt" above).
- The attention badge needs a signal that the agent waits for a human. Claude
  has one in its hooks. pi has one in its `ui_prompt` events, which a
  producer turns into `notification` and then into an empty
  `user-prompt-submit`.
