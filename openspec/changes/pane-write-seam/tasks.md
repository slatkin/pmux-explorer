# Tasks

## 1. The store seam and the encoder

- [ ] 1.1 Add a `PaneWrites` type (ordered `Set(key, value)` / `Unset(key)`) and a `PaneStore` trait with `snapshot(pane)` and `apply(pane, &PaneWrites)`. Verify: `cargo build` passes and a unit test builds a `PaneWrites` and reads it back.
- [ ] 1.2 Add the tmux adapter. `apply` builds one `tmux` invocation with `;`-joined `set` commands, and `snapshot` reads `status`, `bg_cmd`, `subagents` and `started_at` with one `display-message`, using the `list-panes` escaping. Verify: unit tests pin the exact argument list of a mixed set/unset batch and the snapshot format string.
- [ ] 1.3 Add the in-memory adapter that implements the same trait and holds options per pane. Verify: a unit test applies a batch and reads the options back, including an `Unset`.
- [ ] 1.4 Encode every value in `apply` with `sanitize_tmux_value`, extended to replace `\r`. Verify: unit tests show `\n`, `\r` and `|` in a value come out as one field, for both adapters.
- [ ] 1.5 Create the store once in the `hook` entry (`src/cli/hook.rs`) and pass it down to the handlers. Verify: `cargo build` passes and `cargo test` is green with the handlers unchanged.

## 2. Reported fields per event

- [ ] 2.1 Add a `Reported` set to the event path. The pi adapter builds it from the matching `PI_EVENTS` entry. The Claude, Codex and OpenCode adapters report `session_id`, `cwd` and `worktree` on every event that carries session context. Verify: per-adapter tests pin the set for each event, and a pi test shows `stop` does not report `worktree`.
- [ ] 2.2 Make an invalid payload report nothing. Verify: a test sends invalid JSON through the adapter and gets an empty `Reported`.
- [ ] 2.3 Reword `docs/agent-protocol.md`: the fields listed for an event are reported and an empty value clears; every other field is left alone. Verify: the text no longer says an omitted field and an empty field are the same, and `setup pi` output is byte-for-byte unchanged (existing contract test passes).

## 3. The transition function

- [ ] 3.1 Add `transition(&PaneSnapshot, &AgentEvent, &Reported) -> PaneWrites` for `SessionStart`, `UserPromptSubmit`, `Stop`, `StopFailure`, `Notification` and `PermissionDenied`. Move the status resolvers from `status_priority.rs` next to it and correct the comment about the subagent guard. Verify: unit tests give the option writes for each event with each of `idle`, `running`, `waiting`, `background` as the previous status, and match today's handler output.
- [ ] 3.2 Encode the subagent guard as one check over exactly today's guarded set: agent label, permission mode, session id, cwd, worktree, task reset, task-store clear, worktree-remove teardown, `SessionEnd`. Verify: tests for each scenario in "a parent's identity survives its subagents", and a test that status, attention and prompt writes still happen while subagents are active.
- [ ] 3.3 Apply the reported-fields rule to `session_id`, `cwd` and the worktree name and branch. Verify: tests for each scenario in "a field that is not reported does not clear", including a pi `stop` that keeps the worktree.

## 4. Move the hook handlers onto the store

- [ ] 4.1 Move `on_user_prompt_submit`, `on_stop`, `on_stop_failure` and `on_session_start` to `snapshot` → `transition` → `apply`. The non-option effects (notifications, the task reset, the activity log, `println!`) stay in the handler. Verify: the existing handler tests pass, migrated to the in-memory adapter, and `on_stop` issues one write batch.
- [ ] 4.2 Move `on_notification`, `on_permission_denied` and `on_teammate_idle`. Verify: the existing attention tests pass, and a `wait_reason` with a newline is stored on one line.
- [ ] 4.3 Move `on_subagent_start`, `on_subagent_stop`, the worktree handler and the pending-teardown helpers. The `@pane_subagents` read-modify-write stays as it is. Verify: the existing subagent and teardown tests pass.
- [ ] 4.4 Remove the per-handler `sanitize_tmux_value` calls and the `set_status` / `set_attention` helpers in `src/cli/mod.rs` once no caller is left. Verify: `cargo clippy` reports no dead code and `grep` finds no `set_pane_option` in `src/cli/hook/handlers/`.

## 5. The activity-log rule

- [ ] 5.1 Move the status logic of `handle_activity_log` into `transition`: `waiting` and `error` become `running`; `idle`, `background` and an absent status stay; `started_at` is set on the first move to `running`. The plan-mode permission write keeps the subagent guard. Verify: one test per scenario in "a finished pane stays finished".
- [ ] 5.2 Make the handler apply the result as one batch and write the activity entry as before. Verify: `handle_activity_log` makes one `snapshot` and at most one `apply`, and the activity log still gets its entry in every scenario.

## 6. Sidebar sweeps and teardown

- [ ] 6.1 Express the bg-shell sweep in `state/refresh.rs` as `PaneWrites`: unset `bg_cmd`, and set `status = idle` only when the status is `background`. Verify: tests for both scenarios in "the sidebar's own sweeps follow the same rules".
- [ ] 6.2 Express the three wipe paths (`clear_all_meta`, `clear_dead_agent_metadata`, `clear_agent_pane_state`) as one `Unset` batch each, keeping each path's own skip list. Verify: existing wipe tests pass, and each wipe makes one `tmux` call.

## 7. Integration

- [ ] 7.1 Add a write-then-read test: apply a `Notification` with a multi-line `wait_reason` through the in-memory adapter, render that pane as a `list-panes` line, and parse it with the `query.rs` decoder. Verify: the pane is present and the reason is one line.
- [ ] 7.2 Update `docs/state-management.md` for the store, the snapshot and the batch. Verify: the doc names the store, the transition and the rules, and no longer describes handlers writing options one by one.
- [ ] 7.3 Run `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release` and `openspec validate pane-write-seam --strict`. Verify: all five exit with status 0.
- [ ] 7.4 Commit the work in concern-sized commits on `main` (seam and encoder, reported fields, transition and handlers, sweeps, docs) and confirm `git status` is clean. Verify: `git status` shows no changes.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
