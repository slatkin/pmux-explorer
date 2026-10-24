# Orchestrator handoff — rebrand-pmux-explorer

Date: 2026-10-24 · Change: rebrand-pmux-explorer · Worktree: ~/Dev/worktrees/rebrand-pmux-explorer · Branch: feat/rebrand-pmux-explorer · Accepted HEAD: aee48bb

## Start here

Rename tmux-agent-sidebar → pmux-explorer repo-wide. Change artifacts committed on the feature branch (186ac49). Baseline + U-rust done. Next: U-shell.

## Accepted units

- Rows 1.1–1.3 (baseline) — orchestrator-run, verified directly:
  - 1.1 worktree created via `wt`; `cargo test` = 1169 pass / 3 fail — failures are `group::tests::{group_panes_display_name_is_basename, resolve_git_info_for_real_repo, worktree_and_main_share_same_repo_root}`, the known pre-existing worktree-path fixtures (same set as sidebar-width-lock campaign; cannot pass outside main checkout).
  - 1.2 inventory at /tmp/rebrand-inventory.txt. Counts (excl. archive + change dir): tmux-agent-sidebar 54, tmux_agent_sidebar 13, TMUX_AGENT_SIDEBAR 1, agent_sidebar 15, agent-sidebar 59, tmux-agent- 59, hiroppy 13. `git mv` list: `.opencode/plugins/tmux-agent-sidebar.js`, `agent-sidebar.conf`, `tmux-agent-sidebar.tmux`.
  - 1.3 `make uninstall` ran (removed nonexistent ~/.local/bin binary = no-op). No `tmux-agent-sidebar@slatkin` plugin and no `slatkin` marketplace installed — nothing to remove.

- U-rust (rows 2.1–2.9) ACCEPTED: impl `95797bc` (33 files, src/+Cargo.*+tests/), task-state `aee48bb`. rust-reviewer PASS no findings. Deviations recorded: `plugin_manifest_omits_redundant_hooks_field` fails until row 3.5 lands plugin.json (self-resolving); popup label `migrate`→`install`; fn rename `build_claude_install_prompt`. Known-fail set is now 4 until row 3.5.

## Active state

- U-shell (rows 3.1–3.6) ISSUED to worker `rebrand-ushell` from clean HEAD aee48bb. Waiting on worker report. Note for worker: it un-breaks `plugin_manifest_omits_redundant_hooks_field` via row 3.5.

## First action

Await `rebrand-ushell` report → mechanical verify → reviewer on the unit range → accept → dispatch U-docs.

## Next unit

- U-docs (rows 4.1–4.5): README/docs/AGENTS.md/CLAUDE.md/.claude/skills/website/fixtures/renovate.json + capture-PNG check. Role: `worker` (doc lane). SCOUT: none — inventory gives the file list; 4.5 is a conditional check.

## Following queue

- Verify (rows 5.1–5.4): orchestrator gates — cargo clean/fmt/clippy/test, insta diff review, zero-match git grep re-run, release build + `version`/`setup pi` smoke, `openspec validate`.

## Environment truths (differ from design doc)

- Checkout dir ALREADY renamed to ~/Dev/pmux-explorer (row 6.1 effectively done early by user).
- tmux.conf:103 `run-shell` points at dead path `/home/slatkin/Dev/tmux-agent-sidebar/tmux-agent-sidebar.tmux` (row 6.2 post-merge).
- Running tmux server holds `@agent_sidebar_bin` + after-new-window/pane-exited hooks pointing at the dead path; 3 stale sidebar processes run from the moved binary (row 6.3 post-merge).
- No ~/.local/bin binary, no plugin dir, no Claude plugin/marketplace — pre-rename install was already absent.

## Open decisions

- None.

## Campaign constraints

- No compat aliases; delete legacy needle + migration prompt (design D2). Keep `@sidebar_*`/`@pane_*` (D3). Release URL → slatkin/pmux-explorer (D6). MIT notice + one README credit line only (D5). Exclude openspec/changes/archive/ and the change dir from renames (D4).
- Worker verification must expect the 3 known group::tests worktree failures; snapshot regen allowed only for pure-rename diffs (non-rename diffs = deviation).
- Post-merge cut-over rows 6.1–6.8 stay unchecked; 6.6 is a companion change in ~/Dev/pmux (separate scope).

## Suggested skills

- implementer: rust-worker (U-rust), worker (U-shell, U-docs); reviewer: rust-reviewer (U-rust), reviewer (U-shell/U-docs).
