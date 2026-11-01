---
title: OpenCode
description: What the sidebar shows for OpenCode panes, and how the plugin bridge maps its events.
---

OpenCode works with the sidebar through a local plugin bridge written for the
OpenCode 2 plugin API (`{ id, setup(context) }`), so the visible surface is
similar to Codex but with a different event source.

## What you get

### Status and prompts

- Live status from `session.created`, `session.inbox.enqueued` and
  `session.execution.started/succeeded`
- Prompt text from the `session.inbox.enqueued` payload
- Response preview from the last `session.text.ended` of the turn, sent on
  `session.execution.succeeded`, headed by the Nerd Font chat glyph (U+F0B79)
- Elapsed time since the last prompt

### Attention cues

- Waiting status + wait reason from `permission.asked`
- API failure reason from `session.execution.failed`
- `notification` desktop alerts for permission prompts

### Activity log

- Tool calls recorded from the `tool.execute.after` hook; OpenCode 2's
  `shell` tool is normalized to the sidebar's `Bash` label alongside `bash`

### Git

- Branch display from the pane's `cwd`

## What is not available

| Feature                    | Why |
| -------------------------- | --- |
| Permission badge           | OpenCode does not expose the Claude-style permission modes |
| Background shell state     | OpenCode does not currently document a background shell flag |
| Task progress counter      | The bridge does not map a task-progress event |
| Sub-agent tree             | OpenCode does not emit Claude-style sub-agent hooks |
| Worktree lifecycle tracking | OpenCode does not emit `WorktreeCreate` / `WorktreeRemove` |

## Setup

Wire the plugin bridge from [OpenCode setup](/pmux-explorer/getting-started/opencode/).
