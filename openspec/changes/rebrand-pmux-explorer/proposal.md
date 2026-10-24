## Why

This fork has diverged far from `hiroppy/tmux-agent-sidebar` and no longer tracks it. It is used only beside pmux. The old name, the upstream repository links, and the upstream release check all say otherwise. Rebranding to `pmux-explorer` makes the break firm and gives pmux one stable name to point at.

## What Changes

- **BREAKING** Rename the product, crate, library, and binary from `tmux-agent-sidebar` to `pmux-explorer`. No compatibility alias is kept.
- **BREAKING** Rename the tmux entry script (`pmux-explorer.tmux`), the Claude plugin (`pmux-explorer@slatkin`), the conf file, the release asset names, and the hook-script path that `setup` writes.
- **BREAKING** Rename the `@agent_sidebar_*` tmux options to `@pmux_explorer_*` and the kebab-case `@agent-sidebar-*` options (`default-agent`, `branch-prefix`, `worktree-dir`, `spawned-worktree`, `spawned-branch`, and the rest of that family) to `@pmux-explorer-*`.
- **BREAKING** Rename the `/tmp/tmux-agent-*` file prefix to `/tmp/pmux-explorer-*`.
- **BREAKING** Remove legacy hook detection. The old `tmux-agent-sidebar/hook.sh` needle and its migration prompt are deleted; the user cleans old hook entries by hand once.
- Rename the checkout directory to `~/Dev/pmux-explorer`. The user does this step.
- Point the update check and all repository, website, and plugin links at `slatkin/pmux-explorer`. Nothing may reference `hiroppy`, except a credit line in the README and the retained MIT copyright notice.
- Rebrand README, website (including author and social metadata), docs, skills, workflows, fixtures, and tests.
- Rename the GitHub repository to `slatkin/pmux-explorer`.
- Update pmux in a companion change in the pmux repo. That change owns the binary override env var (`PMUX_EXPLORER_BIN`), the `TMUX_AGENT_SIDEBAR=0` off-switch, the `/tmp` task path, the `@agent-sidebar-spawned-*` reads, and the sidebar skill.
- Keep the `@sidebar_*` and `@pane_*` tmux options and the agent event protocol unchanged.

## Capabilities

### New Capabilities
- `product-identity`: the product's name, binary, install paths, tmux option and storage names, update source, and attribution, as seen by users and by pmux.

### Modified Capabilities
- `pi-agent-support`: the `setup pi` scenario names the new binary.

## Impact

- Code: `Cargo.toml`, `src/main.rs`, `src/version.rs`, `src/cli/plugin_state.rs`, `src/cli/setup/`, `src/worktree/`, `src/tmux/query.rs`, `src/tmux/types.rs`, `src/ui/notices/prompts.rs`, `hook.sh`, `tmux-agent-sidebar.tmux`, `agent-sidebar.conf`, `install-wizard.sh`, `Makefile`, `.claude-plugin/`, `.opencode/plugins/`, `.github/workflows/`, and about 69 files in total that mention an old form (`git grep`, excluding this change).
- Tests: any test or snapshot that pins the old name, binary, option names, or `/tmp` path.
- Docs: README, `docs/`, `website/`, `.claude/skills/`, `AGENTS.md`, `CLAUDE.md`.
- User machine: `~/.config/tmux/tmux.conf` loads the old script by path; `~/.local/bin/tmux-agent-sidebar` is on PATH; the Claude plugin `tmux-agent-sidebar@slatkin` and marketplace record the old checkout path; the running tmux server holds old hooks. The cut-over tasks cover each.
- pmux: `pi-extension/pmux/sidebar.ts`, `task-reminders.ts`, `skills/sidebar-tasks/SKILL.md`, `docs/guide.md`, and tests. These are tracked by a companion change in the pmux repo.
- Out of scope: any change to the event protocol, UI behavior, or merging the repos.
