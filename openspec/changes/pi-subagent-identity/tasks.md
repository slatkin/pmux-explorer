# Tasks

## 1. Task tag parser

- [x] 1.1 Add the leaf module with `PmuxTaskKind`, `PmuxTask` and `parse_pmux_task` (design D2), registered in `src/lib.rs` like the other leaf helpers; verify unit tests pass for: a spawn path, a resume path, a hyphenated name (`review-unit1` -> `review unit1`), the `subagent` and `resume` fallback stems (no task), a non-pmux `<file>` path, a stem without a timestamp, a malformed timestamp, an empty id, a tag that is not at the start, and an image-style `<file name="…"></file>` with a pmux path (still matches, body ignored)

## 2. Hook: recognise and store the identity

- [x] 2.1 Add `pmux_task: Option<PmuxTask>` to `AgentEvent::UserPromptSubmit`; set `None` in the Claude, Codex and OpenCode adapters and in their tests; verify `cargo build` and the existing adapter tests pass
- [x] 2.2 In `src/adapter/pi.rs`, call the parser on `prompt`; on a match set `pmux_task` and an empty `prompt` (design D1); verify new adapter tests for a spawn prompt, a resume prompt and a plain prompt (unchanged `prompt`, `pmux_task: None`)
- [ ] 2.3 Add `@pane_subagent_kind`, `@pane_subagent_parent` and `@pane_subagent_task` to `src/tmux/options.rs` and to both sweep lists (`clear_all_meta`, `clear_agent_pane_state`); verify `clear_all_meta_drops_every_pane_option_we_own` is extended with the three keys and passes
- [ ] 2.4 Pass `pmux_task` through `src/cli/hook.rs` to `on_user_prompt_submit` and write the three options on `Some` (unset the task option when there is no task); verify handler tests with the tmux mock: a match sets the options and leaves `@pane_prompt` unchanged and the status running; a later plain prompt keeps the options; a later resume match replaces them; `session-end` teardown removes them
- [x] 2.5 Update `docs/state-management.md` (per-pane option table) and `docs/agent-protocol.md` (what the sidebar does with a pmux task prompt, and the "cannot report" list); verify the text names the three options and the two path shapes

## 3. Query: fields and parent resolution

- [ ] 3.1 Append the three options and `pane_title` to `pane_format()`, add their `pane_line_field` indices and grow both `MIN_FIELDS` by four (design D4); verify the existing query tests pass after their line builders are extended
- [ ] 3.2 Add `subagent: Option<SubagentInfo>` to `PaneInfo` (design D5), set `None` at every other construction site, and parse it from the new fields with parent `Orphaned`; verify a parse test for a pane with and without the options, and that an unknown kind value gives `None`
- [ ] 3.3 Build the session-id -> label map in `build_session_hierarchy` and add the resolution post-pass next to `resolve_codex_permission_modes`; verify query tests: a live parent with a title, a live parent with an empty title (label is the pane id), no parent (`Orphaned`), and a parent in another tmux session

## 4. Rendering

- [ ] 4.1 Show the `subagent` / `resumed` badge in `status_row` (design D6); verify inline `insta` snapshots for a spawned subagent, a resumed subagent, and a plain pi pane (no badge)
- [ ] 4.2 Add the identity row directly under the status row (parent label or red `ORPHANED`, then the task); verify inline `insta` snapshots for parent and task, parent without task, `ORPHANED`, and a narrow width where the row is cut to one line, plus a color test that `ORPHANED` uses `status_error`
- [ ] 4.3 Verify a full-frame inline snapshot in `tests/ui_snapshot.rs` with a parent pi pane and a subagent pi pane whose parent row is hidden by the repository filter: the subagent still shows the parent label

## 5. Integration

- [ ] 5.1 Run `cargo fmt`, `cargo clippy` and `cargo test`; verify all pass with no new warnings
- [ ] 5.2 Run `cargo build --release`, restart the sidebar, spawn a pmux scout from a pi pane whose title is set with `select-pane -T`; verify the scout row shows the `subagent` badge, the parent title and the fuzzy task, no `<file` text, and `ORPHANED` in red after the parent pi exits

## Workflow follow-up

- Archive the change after review, so the `pi-agent-support` main spec takes the delta.
