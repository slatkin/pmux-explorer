---
title: Keybindings
description: Every shortcut in the sidebar, the worktree spawn modal, and the close-pane modal.
---

## Sidebar

| Key            | Action                                                              |
| -------------- | ------------------------------------------------------------------- |
| `prefix + e`   | Toggle sidebar                                                      |
| `prefix + E`   | Toggle sidebar in all windows                                       |
| `j` / `Down`   | Move selection down                                                 |
| `k` / `Up`     | Move selection up                                                   |
| `h` / `Left`   | Previous status filter                                              |
| `l` / `Right`  | Next status filter                                                  |
| `r`            | Open repo filter popup                                              |
| `Enter`        | Jump to the selected pane                                           |
| `Tab`          | Cycle status filter                                                 |
| `Shift+Tab`    | Cycle bottom panel tab (Tools, Thought (pi panes only), Tasks, Git) |
| `prefix + Ctrl+P` | Pop the sidebar out / back in while the sidebar has focus        |
| `prefix + Ctrl+O` | Switch the sidebar's side (every window) while the sidebar has focus |
| `Esc`          | Return focus or close the popup                                     |

### Strip sidebar (top / bottom position)

When the sidebar is a top or bottom strip, the keys change shape: Left/Right
(`h`/`l`) walk the focus between the three parts — filter bar, agent panel,
tab panel — and stop at the ends, while Up/Down (`j`/`k`) act inside the
focused part.

| Key            | Action                                                              |
| -------------- | ------------------------------------------------------------------- |
| `h` / `Left`   | Move focus one part to the left (stops at the filter bar)           |
| `l` / `Right`  | Move focus one part to the right (stops at the tab panel)           |
| `k` / `Up`     | Filter bar: previous status filter · Agent panel: select the pane above · Tab panel: scroll up |
| `j` / `Down`   | Filter bar: next status filter · Agent panel: select the pane below · Tab panel: scroll down |
| `r`            | Open repo filter popup                                              |
| `Enter`        | Jump to the selected pane                                           |
| `Tab`          | Cycle status filter (while the filter bar has focus)                |
| `Shift+Tab`    | Cycle bottom panel tab (Tools, Thought (pi panes only), Tasks, Git) |
| `prefix + Ctrl+P` | Pop the sidebar out / back in while the sidebar has focus        |
| `prefix + Ctrl+O` | Switch the sidebar's side (every window) while the sidebar has focus |
| `Esc`          | Return focus or close the popup                                     |

For pop-out/pop-in, press the tmux prefix, release it, then press `Ctrl+P`.
Bare `p` does nothing. `Ctrl+P` without the prefix still moves the selection up.
Outside the sidebar, `prefix + Ctrl+P` keeps tmux's previous-window action.

The side switch works the same way: press the tmux prefix, release it, then
press `Ctrl+O` while the sidebar has focus (or click the `⇄` control left of
the pop-out control in the top row). The switch moves the docked sidebar of
every window to the opposite edge — a zoomed window or a floating sidebar is
skipped — and reports the moved and skipped counts. Bare `o` and `Ctrl+O`
without the prefix do nothing. Outside the sidebar, `prefix + Ctrl+O` keeps
tmux's `rotate-window` action. See [tmux options](/pmux-explorer/reference/tmux-options/)
for how the switch interacts with `@sidebar_position`.

## Repo filter popup

Opened with `r` or by clicking the repo filter button in the sidebar header.

| Key           | Action                                 |
| ------------- | -------------------------------------- |
| `j` / `Down`  | Move selection down                    |
| `k` / `Up`    | Move selection up                      |
| `Enter`       | Confirm — filter the list to that repo |
| `Esc`         | Cancel                                 |

## Notices popup

Opened by clicking the `ⓘ` badge shown when hooks or plugin setup are missing.

| Key   | Action          |
| ----- | --------------- |
| `Esc` | Close the popup |

## Worktree

| Key | Action                                 |
| --- | -------------------------------------- |
| `n` | Spawn a new worktree + agent           |
| `x` | Remove the selected spawn-created pane |

## Spawn worktree modal

Opened with `n` on a repo.

| Key                                | Action                                                                                           |
| ---------------------------------- | ------------------------------------------------------------------------------------------------ |
| Text keys                          | Type the name (used as the branch slug and tmux window name)                                     |
| `↑` / `↓` / `Tab` / `Shift+Tab`    | Move focus between `NAME` / `AGENT` / `MODE` fields                                              |
| `←` / `→`                          | Cycle the value when the agent or mode field has focus                                           |
| `Enter`                            | Create the worktree + window and launch the agent                                                |
| `Esc`                              | Cancel                                                                                           |

## Close pane modal

Opened with `x` on a spawn-created pane.

| Key             | Action                                                                                                    |
| --------------- | --------------------------------------------------------------------------------------------------------- |
| `y` / `Enter`   | Close the tmux window, remove the git worktree (`--force`), and delete the branch (`git branch -D`)       |
| `c`             | Close the tmux window only, keep the worktree and branch on disk                                          |
| `n` / `Esc`     | Cancel                                                                                                    |
