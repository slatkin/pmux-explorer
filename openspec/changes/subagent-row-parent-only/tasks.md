# Tasks

## 1. Row rendering

- [x] 1.1 In `subagent_identity_row` (`src/ui/panes/row/body.rs`), stop building the `  <task>` suffix: the line renders `▷ <parent label>` (or `▷ ORPHANED`), still cut to one row by the per-piece cut; update the function doc comment.
- [x] 1.2 Update the identity-row tests in `src/ui/panes/row.rs`: the parent+task snapshot renders the parent only with a kept assertion that the task input is never drawn, the duplicate no-task test goes away, the orphaned snapshot renders `▷ ORPHANED`, the narrow-width test keeps the parent cut, and the branch-row placement snapshot drops the task.
- [x] 1.3 Regenerate the `snapshot_subagent_identity_survives_repo_filter_hiding_parent` snapshot in `tests/ui_snapshot.rs` with `cargo insta accept`; verify `cargo test` passes.

## 2. Docs and gates

- [x] 2.1 Fix the wording that says the hyphens are "shown" as spaces in `docs/agent-protocol.md` and `docs/state-management.md` — the task is stored, nothing shows it now.
- [x] 2.2 Run and pass the gates: `openspec validate --strict`, `cargo fmt`, `cargo clippy` with no new warnings, `cargo test`, and `cargo build --release`.
- [x] 2.3 Commit with `cargo fmt` run first.
- [ ] 2.4 Archive the change after the row is confirmed against a live pmux subagent pane (the archive commit merges the delta into `openspec/specs/pi-agent-support/spec.md` and lands as its own chore commit).
