# Proposal

## Why

The Git tab shows status only: the branch, the ahead/behind counts, the PR link and the changed files with their line counts. To stage, commit, check out a branch or manage a worktree for the focused agent's repo, the user must leave the sidebar and open a git tool in another pane. lazygit already does all of this, and a narrow layout of it that shows only its list panels fits in the bottom panel.

## What Changes

- A new global tmux option `@sidebar_lazygit` (`on`/`off`, default `off`). The conf seeds it, and the sidebar reads it once at startup. With `off`, the Git tab is the native view, with no change.
- With `on`, the Git tab shows a live lazygit under one native header row. lazygit runs on a pseudo-terminal that the sidebar owns, and the sidebar draws lazygit's screen into the tab's content area. This works in a column, in a one-pane strip and in pane A of a split strip.
- lazygit shows only its list panels (status, files, branches, commits, stash). It does not show the diff panel. The sidebar passes lazygit a config overlay after the user's own config. The overlay turns off the diff section, auto-fetch, suspend, the bottom line, the tips and the command log, and sets the files refresh to 2 seconds.
- The header row shows the PR link (OSC 8) at the left and the total `+n/-n` diff stat at the right. A reduced git poll feeds it. The native file list is not fetched in lazygit mode.
- lazygit follows the focused agent pane's repo. When the repo top-level changes, the sidebar restarts lazygit with the new path after a 300 ms debounce. A move inside the same repo does not restart it.
- Explicit key capture: Enter on the focused Git tab, or a click in the lazygit area, sends the keys and mouse to lazygit. Ctrl-] or a click outside the area releases them. Uncaptured, lazygit keeps drawing and polling, and the sidebar's keys work as before. The header row shows the capture state.
- lazygit runs only while the sidebar's window is the active window of an attached session. After the window is hidden for 10 seconds, lazygit stops. It starts again when the window is shown.
- Fallbacks: when `lazygit` is not on `PATH`, the tab shows the native view and one notice. When the focused path is not in a git repo, the tab shows a notice and starts no lazygit. When lazygit exits, the tab shows a notice, and Enter starts it again.

## Capabilities

### New Capabilities

- `lazygit-view`: the opt-in lazygit Git tab — the option and its fallbacks, the list-only layout, the header row, repo following, key and mouse capture, the visibility gate and the exit handling.

### Modified Capabilities

- None. No current spec covers the content of the native Git tab, and with the option `off` nothing changes. The capture rules are new behavior in `lazygit-view`. They do not change the key maps in `strip-layout`, `split-strip` or the column, which stay true while keys are not captured.

## Impact

- `Cargo.toml`: two new pinned dependencies, a terminal screen model (`vt100`) and a pseudo-terminal (`portable-pty`).
- New module for the lazygit host: the pty child, the screen, the key and mouse encoders, the restart debounce, the visibility gate and the overlay config file.
- `src/ui/bottom.rs` (`draw_bottom_content`) and `src/ui/bottom/git.rs`: the Git tab chooses the native view or the lazygit view, and the lazygit view draws the header row and the screen cells.
- `src/app.rs`: the event loop drains the pty output channel and redraws, and ticks the host (debounce, gate, grace period).
- `src/app/input.rs`: capture, release and forwarding of keys, mouse and paste.
- `src/app/workers.rs` and `src/git.rs`: a reduced poll mode (diff stat, branch, remote URL, PR number).
- `src/app/setup.rs`, `src/ui/mod.rs`, `src/tmux/options.rs`: the startup read of `@sidebar_lazygit` and the `PATH` lookup.
- `src/tmux/panes.rs`: `#{session_attached}` joins the sidebar's per-tick pane read.
- `pmux-explorer.conf`, `docs/state-management.md`, `website/src/content/docs/reference/tmux-options.md`: the new option.
- Runtime: one lazygit process per visible sidebar while the option is `on`. Users who leave the option `off` get no new process and no new behavior.
