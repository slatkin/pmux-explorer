---
title: pi
description: What the sidebar shows for pi panes, and how the pmux extension maps pi's events.
---

pi works with the sidebar through the pmux extension. The extension loads in
every pi session, so a pi pane reports itself whether pmux started it or a
human opened it by hand.

## What you get

### Status and prompts

- Live status from the turn lifecycle: running while the turn works, idle when
  it ends, error when it fails
- Prompt text from the input event
- Response preview from the last assistant message of the turn, headed by the Nerd Font chat glyph (U+F0B79)
- Elapsed time since the last prompt

### Attention cues

- Waiting status and wait reason when pi waits on a blocking prompt, which is
  how a permission extension asks the user
- API failure reason when the run settles with an error
- `notification` desktop alerts for the wait

### Activity log

- Tool calls recorded when they start and when they end, with pi's tool names
  normalised to the sidebar's vocabulary

### Git and worktrees

- Branch display from the pane `cwd`
- Worktree and branch when the pane sits in a worktree the sidebar spawned

## What is not available

| Feature | Why |
| --- | --- |
| Permission badge | pi has no permission mode the sidebar can read |
| Background shell state | pi runs its shell inside the agent, so there is no background command to report |
| Task progress counter | pi has no task tool |
| Sub-agent tree | pi has no in-process sub-agents; pmux children are panes of their own, and the sidebar lists each of them |
| Worktree lifecycle tracking | pi does not report worktree events; the sidebar writes the markers when it spawns the worktree |

## Setup

The producer is the pmux pi extension, so install pmux as a pi package and
nothing else. The contract it implements is written down in the sidebar's
[agent event protocol](https://github.com/slatkin/pmux-explorer/blob/main/docs/agent-protocol.md),
and `pmux-explorer setup pi` prints it as JSON.
