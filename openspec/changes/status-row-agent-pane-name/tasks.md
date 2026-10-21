# Tasks

## 1. Pane-name plumbing

- [ ] 1.1 Add `pane_name: Option<String>` to `PaneInfo` (`src/tmux/types.rs`), defaulting to `None` in every constructor and test fixture; verify `cargo build` compiles with no dead-code warnings.
- [ ] 1.2 Add `q("host")` as the last field of `pane_format()` and a matching `pane_line_field` index and `MIN_FIELDS` bump (`src/tmux/query.rs`), then resolve the pane name in `parse_pane_fields_with_processes`: `Some(pane_title)` unless the title is empty, equals the host field, or starts with `π - ` or `✳ `; `None` otherwise. Update the existing fixture lines for the new field. Add parse tests: a pmux-named title yields `Some`; a host-name title, a `π - …` title, a `✳ …` title, and an empty title each yield `None`; verify `cargo test tmux::query` passes.

## 2. Status-row rendering

- [ ] 2.1 Compose the title in `status_row` (`src/ui/panes/row/status.rs`): agent label always, pane name appended after it, badge slot unchanged in position. Reserve the agent label's width in `fixed_width` and give the pane name the remaining budget with ellipsis truncation; the elapsed-truncation fallback stays. Add row tests (inline `insta::assert_snapshot!`) for: agent label shown on a named pane, only the agent label when there is no pane name, and a long pane name truncated while the agent label and elapsed stay visible; verify `cargo test` for the row module passes.
- [ ] 2.2 Change the resumed badge to `↻` with `badge_auto` colour in the `PmuxTaskKind::Resumed` match arm; update the badge test to assert ♟ for spawned and ↻ for resumed; verify `cargo test` passes.
- [ ] 2.3 Regenerate the UI snapshots with `cargo insta accept` and review the diff to confirm only the intended title-composition changes appear; verify `cargo test` passes.

## 3. Session-name pipeline removal

- [ ] 3.1 Remove the session-name pipeline: `src/session.rs` and `pub mod session` in `src/lib.rs`, the `session_poll_loop` thread in `src/app/workers.rs`, the initial scan in `src/app/setup.rs`, `refresh_session_names` and the `sessions.names` map in `src/state/` (not the unrelated `mod session` at `src/state.rs:19`), and the `PaneInfo.session_name` field with its fixture hits. Verify `cargo build` reports no unused-code warnings and `rg session_name src/` returns only unrelated matches (git rename detection, test fixture names).
- [ ] 3.2 Update the row tests that asserted the session-name replacement behaviour (`src/ui/panes/row.rs` "session name should appear / agent label replaced") to assert the new composition instead; verify `cargo test` passes.

## 4. Docs and gates

- [ ] 4.1 Update `docs/state-management.md`: add `pane_name` to the `PaneInfo` field list and data-flow description; remove the session-name map from the state tables and the frequency diagram; verify the documented flow matches the code by re-reading both.
- [ ] 4.2 Run the full gate suite and verify all pass: `cargo fmt`, `cargo clippy` with no new warnings, `cargo test`, and `cargo build --release`.

## Workflow follow-up

- Commit with `cargo fmt` run first; push only on explicit instruction.
- Archive the change after the project's review requirements are satisfied.
