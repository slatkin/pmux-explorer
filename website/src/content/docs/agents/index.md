---
title: Agent support overview
description: What the sidebar shows for Claude Code, Codex, OpenCode, and pi, side by side.
---

Claude Code, Codex, OpenCode, and pi work with the sidebar, but they expose different sets of events — so the sidebar's surface area is narrower for some of them than it is for Claude Code.

## Feature support by agent

| Feature | Claude Code | Codex | OpenCode | pi | Notes |
| --- | --- | --- | --- | --- | --- |
| Base status tracking | ✓ | ✓ | ✓ | ✓ | Covers `running`, `idle`, and `error`; `waiting` and `background` depend on agent-specific events |
| Prompt text display | ✓ | ✓ | ✓ | ✓ | Saved from the prompt event of each agent |
| Response text display (chat glyph, U+F0B79) | ✓ | ✓ | ✓ | ✓ | Populated from the end-of-turn payload |
| Background shell state | ✓ | — | — | — | Claude Bash tools can report `run_in_background`; the others do not document a background Bash flag |
| Waiting status + wait reason | ✓ | — | ✓ | ✓ | Claude has `Notification`, `PermissionDenied`, and `TeammateIdle`; OpenCode maps permission prompts; pi maps its blocking prompt events |
| API failure reason display | ✓ | — | ✓ | ✓ | The failure event is wired for Claude, OpenCode, and pi |
| Permission badge | ✓ (`plan` / `edit` / `auto` / `!`) | ✓ (`auto` / `!` only) | — | — | Codex badges are inferred from process arguments; OpenCode and pi do not expose permission modes |
| Git branch display | ✓ | ✓ | ✓ | ✓ | Uses the pane `cwd`; Claude updates dynamically via `CwdChanged` |
| Elapsed time | ✓ | ✓ | ✓ | ✓ | Since the last prompt |
| Task progress | ✓ | — | — | — | Requires `PostToolUse`; Codex fires `PostToolUse` only for `Bash`, and OpenCode and pi do not surface task progress |
| Task lifecycle notifications | ✓ | ✓ (`Stop` only) | ✓ | ✓ (`Stop` only) | `Stop` desktop notifications fire for all four. The other events vary. |
| Sub-agent display | ✓ | — | — | — | Requires `SubagentStart` / `SubagentStop`; pmux children are panes of their own and the sidebar lists each of them |
| Activity log | ✓ | ✓ (Bash only) | ✓ | ✓ | Codex's `PostToolUse` fires only for `Bash`; OpenCode and pi record the tool events their bridge receives |
| Worktree lifecycle tracking | ✓ | — | — | — | Requires `WorktreeCreate` / `WorktreeRemove`; for pi the sidebar writes the markers when it spawns the worktree |
