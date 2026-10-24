## Context

The product is a Rust crate and binary named `tmux-agent-sidebar`. About 69 tracked files outside this change mention an old form (`git grep`). The install on the user's machine is not a plugin-manager layout:

- `~/.config/tmux/tmux.conf` loads `/home/slatkin/Dev/tmux-agent-sidebar/tmux-agent-sidebar.tmux` by path. There is no `~/.tmux/plugins/tmux-agent-sidebar` link.
- `make install` put `~/.local/bin/tmux-agent-sidebar` on PATH.
- Claude Code loads the plugin `tmux-agent-sidebar@slatkin`; the `slatkin` marketplace records the checkout path.
- The running tmux server holds `after-new-window` and `pane-exited` hooks that call `@agent_sidebar_bin`.

pmux finds the binary through `pi-extension/pmux/sidebar.ts` (env var, two plugin paths, then PATH) and builds the `/tmp/tmux-agent-tasks*.json` path in `task-reminders.ts`. It also reads `@agent-sidebar-spawned-worktree` and `@agent-sidebar-spawned-branch`. `src/version.rs` polls upstream's releases. `plugin_state.rs` carries a legacy hook needle that feeds a migration prompt.

## Goals / Non-Goals

**Goals:** one new name everywhere a user or pmux can see it; no live link to upstream; pmux updated to match; a cut-over order that leaves no mixed old/new state.
**Non-Goals:** compatibility shims, legacy detection, protocol changes, UI changes, merging with pmux.

## Decisions

1. **Name `pmux-explorer`; Rust identifier `pmux_explorer`.** Changes the crate, `[lib]` imports (`src/main.rs`, `tests/`), and binary.
2. **No aliases and no legacy detection.** Old env var, options, `/tmp` files, binary name, and the old hook needle are not read. The needle, its migration prompt, and its test fixtures are deleted. The user removes old `settings.json` entries by hand during cut-over.
3. **Rename `@agent_sidebar_*` to `@pmux_explorer_*` and `@agent-sidebar-*` to `@pmux-explorer-*`. Keep `@sidebar_*` and `@pane_*`.** The first two families carry the product name; some are user settings, and pmux reads two of them. `@sidebar_*` and `@pane_*` are generic and add churn for no identity gain.
4. **Repo-wide rename by scripted replace, then review.** Forms differ: `tmux-agent-sidebar`, `tmux_agent_sidebar`, `TMUX_AGENT_SIDEBAR`, `agent_sidebar`, `agent-sidebar` (conf file, `@agent-sidebar-` options, `.opencode` file), and the `tmux-agent-` `/tmp` prefix. Do each form separately and read the diff. Use `git grep`, so ignored files (`.zvec-grep/`, `target/`) are skipped. Exclude `openspec/changes/archive/` (history) and this change's folder. File renames (`.tmux`, conf, `.opencode` plugin) are separate `git mv` tasks because grep does not find them.
5. **Keep the MIT notice, add one credit line.** MIT requires keeping the copyright notice; a credit line costs nothing.
6. **Release check targets `slatkin/pmux-explorer`.** The fork has no releases yet, so the check reports "no update" until the first tag. The repo rename can happen at any point before the first tag.
7. **pmux changes ship as a companion change in the pmux repo.** This change's edit root is this repo. The contract pmux needs is: binary name `pmux-explorer`; override env var `PMUX_EXPLORER_BIN` (pmux owns it; this repo never reads it); off-switch rename (pmux owns it); `/tmp/pmux-explorer-tasks*.json`; `@pmux-explorer-spawned-*` options; and updated skill text.
8. **Cut-over order matters.** pmux searches PATH last, and the cached old Claude hook script also falls back to PATH, so a leftover old binary would keep writing old `/tmp` files and shared `@pane_*` options next to the new sidebar. Order:
   1. With the old Makefile, uninstall the old binary (`make uninstall`) and the old Claude plugin (`claude plugin uninstall tmux-agent-sidebar@slatkin`).
   2. Merge the rename and build.
   3. The user renames the checkout directory (not done by the agent).
   4. Edit `tmux.conf` to load `pmux-explorer.tmux` from the new path.
   5. Clear the old tmux hooks (`set-hook -gu` for `after-new-window` and `pane-exited` entries that match `@agent_sidebar_bin`) or restart the tmux server.
   6. `make install`, re-add the Claude marketplace at the new path, then `make install-claude-hooks` to install `pmux-explorer@slatkin`.
   7. Remove old hook entries from `~/.claude/settings.json` by hand.
   8. Land the pmux companion change.
   9. Rename the GitHub repo and update `origin`.

## Risks / Trade-offs

- A stale tmux hook or a stale `target/release/tmux-agent-sidebar` stays after the rename and runs old paths → clear hooks in step 5; run `cargo clean` before the first new build.
- A missed occurrence breaks a path at run time, not compile time (shell, conf, JSON, workflows) → `git grep` for every old form must return zero outside the stated exceptions; run the sidebar for real.
- Release asset names in `release.yml` and the download names in `install-wizard.sh` must change together, or the wizard downloads nothing.
- `cargo insta` snapshots that include the name or option names change → review each diff, then accept.
- Website capture PNGs may show the old name; regenerate them with the `regenerate-captures` skill if they do. This does not gate the change.
- The website base URL moves to the fork's Pages address; if Pages is not enabled on the renamed repo, the deploy workflow fails. This is not checked here.
- GitHub's redirect from the old URL keeps old links alive but does not detach the fork. Detaching needs a GitHub support request and is outside this change.
