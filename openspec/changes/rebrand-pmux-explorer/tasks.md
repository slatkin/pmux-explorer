## 1. Baseline and pre-rename uninstall

- [x] 1.1 Create a worktree with `wt rebrand-pmux-explorer`; run `cargo test` and record that it passes
- [x] 1.2 Record the old-name inventory with `git grep -l` for each form: `tmux-agent-sidebar`, `tmux_agent_sidebar`, `TMUX_AGENT_SIDEBAR`, `agent_sidebar`, `agent-sidebar`, `tmux-agent-` (the `/tmp` prefix), and `hiroppy`. Exclude `openspec/changes/archive/` and this change's folder. List file names that need `git mv` separately (`ls` of `*sidebar*`)
- [x] 1.3 With the OLD Makefile still in place, run `make uninstall` to remove `~/.local/bin/tmux-agent-sidebar`, and `claude plugin uninstall tmux-agent-sidebar@slatkin`

## 2. Rust crate and binary

- [x] 2.1 `Cargo.toml`: name, `repository`, and any `[lib]`/`[[bin]]` names; update `Cargo.lock`
- [x] 2.2 Replace `tmux_agent_sidebar` with `pmux_explorer` in `src/main.rs` and `tests/`
- [x] 2.3 `src/version.rs`: release URL to `slatkin/pmux-explorer`; fix its tests. Keep the bare version output format
- [x] 2.4 `src/cli/plugin_state.rs`: change `PLUGIN_NAME`; delete the legacy `RESIDUAL_HOOK_NEEDLE` detection (`plugin_state.rs:172-200`), its test fixtures (`:642-663`), and the migration prompt that uses it (`src/ui/notices/prompts.rs:104`)
- [x] 2.5 `src/ui/notices/prompts.rs` and `website/.../getting-started/claude-code.md`: plugin references become `pmux-explorer@slatkin`
- [x] 2.6 `src/cli/setup/` (including tests): usage text, pi `command` string, fallback hook path, doc comments
- [x] 2.7 Rename the `/tmp/tmux-agent-activity*` and `/tmp/tmux-agent-tasks*` prefixes and their test expectations
- [x] 2.8 Rename the `@agent-sidebar-*` constants to `@pmux-explorer-*` in `src/worktree/config.rs` and `src/worktree/markers.rs`, and their uses in `src/tmux/query.rs` and `src/tmux/types.rs`; update tests and snapshots
- [x] 2.9 Update remaining user-visible strings in `src/` (notices, help text)

## 3. Plugin, shell, and CI layer

- [ ] 3.1 `git mv tmux-agent-sidebar.tmux pmux-explorer.tmux`; update the binary lookup and `@agent_sidebar_bin` to `@pmux_explorer_bin`
- [ ] 3.2 `git mv agent-sidebar.conf pmux-explorer.conf`; update its `@agent_sidebar_*` options, the hook-dedupe match on `@pmux_explorer_bin`, and the `.tmux` reference to it
- [ ] 3.3 `hook.sh`, `install-wizard.sh`, `Makefile` (`CLAUDE_PLUGIN` and install/uninstall targets), `scripts/`: names, paths, release asset names
- [ ] 3.4 `.github/workflows/release.yml`: asset names `pmux-explorer-*`, kept in step with `install-wizard.sh`; `.github/workflows/deploy-website.yml`: names and paths
- [ ] 3.5 `.claude-plugin/plugin.json` and `marketplace.json`: name and repository
- [ ] 3.6 `git mv .opencode/plugins/tmux-agent-sidebar.js .opencode/plugins/pmux-explorer.js`; update its contents (`:26`) and the symlink steps in README and `AGENTS.md`

## 4. Docs, skills, website, fixtures

- [ ] 4.1 README: new name, install steps, one credit line for `hiroppy/tmux-agent-sidebar`; keep `LICENSE`. Replace the `hiroppy.github.io` doc links
- [ ] 4.2 `docs/` (including `docs/superpowers/*`), `AGENTS.md`, `CLAUDE.md` (drop the upstream link), `.claude/skills/*`
- [ ] 4.3 `website/`: `astro.config.mjs` (site, base, author and social metadata, remove upstream's accounts), `package.json` and `package-lock.json` name, `src/styles/custom.css`, `public/robots.txt`, docs content including the tmux options reference for `@pmux-explorer-*`
- [ ] 4.4 `fixtures/` and `renovate.json`: any old-name reference
- [ ] 4.5 Check whether the website capture PNGs show the old name; if so, run the `regenerate-captures` skill

## 5. Verify

- [ ] 5.1 `cargo clean`, `cargo fmt`, `cargo clippy`, `cargo test`; review and accept each `cargo insta` diff that is only a rename
- [ ] 5.2 Re-run the task 1.2 `git grep` forms: zero matches except `LICENSE` and the README credit line
- [ ] 5.3 `cargo build --release`; run `target/release/pmux-explorer version` and `setup pi`; confirm the pi command line reads `pmux-explorer hook pi <event>`
- [ ] 5.4 `openspec validate rebrand-pmux-explorer` passes

## 6. Cut-over (after merge)

- [ ] 6.1 The user renames the checkout directory to `~/Dev/pmux-explorer` (the agent does not do this)
- [ ] 6.2 Edit `~/.config/tmux/tmux.conf` (around line 103) to `run-shell` the new `pmux-explorer.tmux` path
- [ ] 6.3 Clear old tmux hooks: remove `after-new-window` and `pane-exited` entries that match `@agent_sidebar_bin`, or restart the tmux server
- [ ] 6.4 Run `make install`; re-add the Claude marketplace at the new checkout path; run `make install-claude-hooks` to install `pmux-explorer@slatkin`
- [ ] 6.5 Remove old `tmux-agent-sidebar/hook.sh` entries from `~/.claude/settings.json` by hand
- [ ] 6.6 Create a change in `~/Dev/pmux` and land it. It covers:
  - binary lookup and the override env var (now `PMUX_EXPLORER_BIN`) in `pi-extension/pmux/sidebar.ts:26-48`
  - the `TMUX_AGENT_SIDEBAR=0` off-switch at `sidebar.ts:220-224`
  - the `@agent-sidebar-spawned-worktree` and `-branch` reads at `sidebar.ts:80-81`, now `@pmux-explorer-spawned-*`
  - the `/tmp/tmux-agent-tasks` path in `task-reminders.ts:90` and comments in `index.ts`
  - `skills/sidebar-tasks/SKILL.md` (including the snippet at `:45`) and `docs/guide.md`
  - tests `test/sidebar-events.test.ts`, `test/task-reminders.test.ts`, `test/sidebar.test.ts`, `test/cli.test.ts`
  - the live `sidebar-events` and `task-reminders` specs
- [ ] 6.7 Rename the GitHub repo to `slatkin/pmux-explorer` and update `origin`
- [ ] 6.8 Confirm a pi pane started by pmux appears in the sidebar, `task-set` writes `/tmp/pmux-explorer-tasks*.json`, and no `tmux-agent-sidebar` process or `/tmp/tmux-agent-*` file is created
