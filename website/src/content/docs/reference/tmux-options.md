---
title: tmux options
description: Every @sidebar_* / @pmux-explorer-* option the plugin reads.
---

Most options must be set **before** loading the plugin in your `tmux.conf`. Color values can be xterm 256-color numbers (`0`-`255`) or six-digit hex colors (`#RRGGBB` or `RRGGBB`); icons can be any Unicode glyph.

## Sidebar behavior

| Option                           | Default | Description                                                                             |
| -------------------------------- | ------- | --------------------------------------------------------------------------------------- |
| `@sidebar_key`                   | `e`     | Prefix-triggered keybinding to toggle the sidebar in the current window                 |
| `@sidebar_key_all`               | `E`     | Prefix-triggered keybinding to toggle the sidebar in all windows                        |
| `@sidebar_width`                 | `15%`   | Width in columns or as a percentage                                                     |
| `@sidebar_min_width`             | `35`    | Minimum sidebar width in columns; the sidebar grows itself back when squeezed below it  |
| `@sidebar_max_width`             | `0`     | Maximum sidebar width in columns; the sidebar shrinks itself back when tmux's resize behaviour pushes it past it. `0` disables it, and a ceiling at or below the effective minimum is ignored — the floor wins |
| `@sidebar_position`              | `left`  | Sidebar placement (`left` or `right`). The side switch (`prefix + Ctrl+O` or the `⇄` control in the top row) flips this option globally for the running server, so sidebars created later follow the switched side; the value in your tmux config is never written and applies again after a tmux server restart                  |
| `@sidebar_bottom_height`         | `30`    | Bottom panel height as a percent of the sidebar height (set `0` to hide); resize by dragging the rule above the panel. A configuration with a line count gets that number as a percent |
| `@sidebar_auto_create`           | `on`    | Auto-create the sidebar on new windows (set `off` to disable)                           |
| `@sidebar_auto_create_delay`     | `0`     | Seconds to defer auto-create after a window opens, so a declaratively-built window (e.g. tmuxinator's `select-layout`) finishes before the sidebar pane is injected; accepts fractional seconds. `0` keeps the create synchronous |
| `@sidebar_notifications`         | `on`    | Master switch for desktop notifications                                                 |
| `@sidebar_notifications_events`  | unset   | Restrict events — see [Notifications](/pmux-explorer/features/notifications/)       |
| `@sidebar_pet`                  | `off`   | Show the animated pet in a 5-row band above the bottom panel                           |

Setting `@sidebar_min_width` to `0` disables the width floor entirely;
`@sidebar_max_width` defaults to `0` (no ceiling) for the same reason a
larger manual size sticks — set a ceiling only when tmux's resize
behaviour hands the sidebar more columns than it asked for, which shows
up when it docks on the right.

## Worktree spawn defaults

| Option                            | Default     | Description                                               |
| --------------------------------- | ----------- | --------------------------------------------------------- |
| `@pmux-explorer-default-agent`    | `claude`    | Agent launched by `n`&nbsp;(also accepts `codex`)         |
| `@pmux-explorer-branch-prefix`    | `agent/`    | Branch prefix for new worktrees                           |
| `@pmux-explorer-worktree-dir`     | `.worktrees` | Repo-relative directory for sidebar-spawned worktrees; absolute paths and `..` are rejected |

## Status and filter colors

| Option                            | Default         | What it paints                                                    |
| --------------------------------- | --------------- | ----------------------------------------------------------------- |
| `@sidebar_color_all`              | `#7aa2f7`&nbsp;(blue)| Selected "all" filter icon                                        |
| `@sidebar_color_running`          | `#9ece6a`&nbsp;(green)   | Busy chip icon (selected or not), the pop-out control glyph, and running/background pane status |
| `@sidebar_color_waiting`          | `#e0af68`&nbsp;(yellow)  | Selected waiting filter icon, waiting pane status, version banner |
| `@sidebar_color_idle`             | `#7dcfff`&nbsp;(cyan) | Selected idle filter icon and idle pane status                  |
| `@sidebar_color_error`            | `#f7768e`&nbsp;(red) | Selected error filter icon and error pane status                 |
| `@sidebar_color_filter_inactive`  | `#9aa5ce`&nbsp;(blue-gray) | Unselected status filter icons and zero counts                   |

## Structural colors

| Option                     | Default              | What it paints                                                                                          |
| -------------------------- | -------------------- | ------------------------------------------------------------------------------------------------------- |
| `@sidebar_color_border`    | `240`&nbsp;(dark gray)    | Unfocused panel borders and tab separators                                                              |
| `@sidebar_color_accent`    | `#7aa2f7`&nbsp;(blue) | Active pane marker, focused repo header, displayed bottom tab, repo popup border — the brand color |
| `@sidebar_color_session`   | `#7dcfff`&nbsp;(cyan)          | Session name                                                                                            |
| `@sidebar_color_selection` | `#414868`&nbsp;(dark blue-gray)    | Selected row background                                                                                 |
| `@sidebar_color_filter_bar_bg` | `#414868`&nbsp;(dark blue-gray) | Filter bar row background, painted at all times — focused or not |
| `@sidebar_color_focus_bg`  | `#1a1b26`&nbsp;(Tokyo Night background) | Base background over the whole sidebar, painted at all times — focused or not |
| `@sidebar_color_focus_active_bg` | `#24283b`&nbsp;(Tokyo Night storm) | Background over the keyboard-owning section, layered on `focus_bg` |
| `@sidebar_color_focus_border` | `#e69875`&nbsp;(orange) | Border colour tmux draws around the sidebar pane while it is the active pane of its window |

## Agent colors

| Option                          | Default            | What it paints       |
| ------------------------------- | ------------------ | -------------------- |
| `@sidebar_color_agent_claude`   | `#ff9e64`&nbsp;(orange) | Claude brand color   |
| `@sidebar_color_agent_codex`    | `#bb9af7`&nbsp;(purple)     | Codex brand color    |
| `@sidebar_color_agent_opencode` | `#7dcfff`&nbsp;(cyan) | OpenCode brand color |
| `@sidebar_color_agent_pi`       | `#e0af68`&nbsp;(yellow) | pi agent color       |

## Pet colors

| Option                       | Default           | What it paints                 |
| ---------------------------- | ----------------- | ------------------------------ |
| `@sidebar_color_pet_body`    | `#ff9e64`&nbsp;(orange) | Body of the animated pet   |
| `@sidebar_color_pet_eye`     | `#9ece6a`&nbsp;(green)  | Eye of the animated pet    |

## Text colors

| Option                         | Default          | What it paints                                                                                   |
| ------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------ |
| `@sidebar_color_text_active`   | `#c0caf5`&nbsp;(foreground)    | Primary text — active rows, counts, filtered repo label                                          |
| `@sidebar_color_text_muted`    | `#a9b1d6`&nbsp;(muted) | Secondary text — tree branches, empty-state messages, inactive bottom tabs, activity log labels |
| `@sidebar_color_text_inactive` | `#565f89`&nbsp;(dark blue-gray) | Body text of unfocused pane rows — prompt / response, idle hint                                  |
| `@sidebar_color_port`          | `#9aa5ce`&nbsp;(blue-gray) | Port numbers                                                                                   |
| `@sidebar_color_wait_reason`   | `#e0af68`&nbsp;(yellow)   | Wait reason text for a reason that does not demand the user                                                     |
| `@sidebar_color_wait_reason_action` | `#f7768e`&nbsp;(red) | Wait reason text for a permission dialog or a blocking prompt, which need the user |
| `@sidebar_color_response_arrow`| `#7dcfff`&nbsp;(cyan) | Response arrow                                                                                 |

## Task and sub-agent colors

| Option                          | Default           | What it paints        |
| ------------------------------- | ----------------- | --------------------- |
| `@sidebar_color_task_progress`  | `#ff9e64`&nbsp;(orange) | Task progress summary |
| `@sidebar_color_task_pending`   | `#7aa2f7`&nbsp;(blue) | Tasks tab `◻` icon and subject of pending items |
| `@sidebar_color_task_completed` | `#9ece6a`&nbsp;(green) | Tasks tab `✔` icon and subject of completed items |
| `@sidebar_color_subagent`       | `#73dacb`&nbsp;(teal)  | Sub-agent tree        |

## Git tab colors

| Option                          | Default            | What it paints      |
| ------------------------------- | ------------------ | ------------------- |
| `@sidebar_color_branch`         | `#73dacb`&nbsp;(teal)       | Git branch name     |
| `@sidebar_color_commit_hash`    | `#e0af68`&nbsp;(yellow)     | Commit hash         |
| `@sidebar_color_diff_added`     | `#9ece6a`&nbsp;(green)      | Added diff lines    |
| `@sidebar_color_diff_deleted`   | `#f7768e`&nbsp;(red) | Deleted diff lines  |
| `@sidebar_color_file_change`    | `#e0af68`&nbsp;(yellow)     | File change stats   |
| `@sidebar_color_pr_link`        | `#7dcfff`&nbsp;(cyan) | PR link / number    |

## Section titles and timestamps

| Option                                | Default      | What it paints      |
| ------------------------------------- | ------------ | ------------------- |
| `@sidebar_color_section_title`        | `#2ac3de`&nbsp;(cyan) | Section titles      |
| `@sidebar_color_activity_timestamp`   | `#2ac3de`&nbsp;(cyan) | Activity timestamps |
| `@sidebar_color_thought_timestamp`    | `#e0af68`&nbsp;(yellow) | Thought tab's `HH:mm:ss` time line |

## Status icons

Any Unicode glyph works. Make sure the glyphs render in your terminal font.

| Option                  | Default | Meaning                      |
| ----------------------- | ------- | ---------------------------- |
| `@sidebar_icon_all`     | `󱓯`     | Status filter bar "all" icon |
| `@sidebar_icon_running`    | ``     | Running status icon          |
| `@sidebar_icon_background` | `◎`     | Background shell status icon |
| `@sidebar_icon_waiting`    | `󰥔`     | Waiting status icon          |
| `@sidebar_icon_idle`    | `󰒲`     | Idle status icon             |
| `@sidebar_icon_error`   | ``     | Error status icon            |
| `@sidebar_icon_unknown` | `·`     | Unknown status icon          |

## Example config

```bash
# Behavior
set -g @sidebar_key T
set -g @sidebar_width 32
set -g @sidebar_position right
set -g @sidebar_bottom_height 25
set -g @sidebar_notifications_events "stop,notification"
set -g @pmux-explorer-default-agent codex

# Colors
set -g @sidebar_color_accent 117
set -g @sidebar_color_agent_claude "#d97757"
set -g @sidebar_color_agent_opencode 39

# Icons
set -g @sidebar_icon_running '▶'
set -g @sidebar_icon_error   '⚠'

run-shell ~/.tmux/plugins/pmux-explorer/pmux-explorer.tmux
```
