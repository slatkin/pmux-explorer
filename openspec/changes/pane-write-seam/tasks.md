# Tasks

## 1. The store seam and the encoder

- [x] 1.1 Add a `PaneWrites` type (ordered `Set(key, value)` / `Unset(key)`) and a `PaneStore` trait with `snapshot(pane)`, `get(pane, key)` and `apply(pane, &PaneWrites)`. Verify: `cargo build` passes and a unit test builds a `PaneWrites` and reads it back.
- [x] 1.2 Add the tmux adapter. `apply` builds one `tmux` invocation with `;`-joined `set` commands and is a no-op on an empty list; `snapshot` reads `status`, `bg_cmd`, `subagents` and `started_at` with one `display-message`, using the `list-panes` escaping; `get` reads one option. Verify: unit tests pin the exact argument list of a mixed set/unset batch and the snapshot format string.
- [x] 1.3 Add the in-memory adapter that implements the same trait and holds options per pane. Verify: a unit test applies a batch and reads the options back, including an `Unset` and a `get`.
- [x] 1.4 Encode every value in `apply` with `sanitize_tmux_value`, extended to replace `\r`. Verify: unit tests show `\n`, `\r` and `|` in a value come out as one field, for both adapters.
- [x] 1.5 Create the store once in the `hook` entry (`src/cli/hook.rs`) and pass it down to the handlers. Verify: `cargo build` passes and `cargo test` is green with the handlers unchanged.

## 2. Reported fields per event

- [x] 2.1 Add a `Reported` set to the event path. The pi adapter builds it from the matching `PI_EVENTS` entry. The Claude, Codex and OpenCode adapters report `session_id`, `cwd` and `worktree` on every event that carries session context (for Claude that includes `cwd-changed`). Verify: per-adapter tests pin the set for each event, and a pi test shows `stop` does not report `worktree`.
- [x] 2.2 Make an invalid payload report nothing. Verify: a test sends invalid JSON through the adapter and gets an empty `Reported`.
- [x] 2.3 Reword `docs/agent-protocol.md`: the fields listed for an event are reported and an empty value clears; every other field is left alone. Verify: the text no longer says an omitted field and an empty field are the same, and `setup pi` output is byte-for-byte unchanged (existing contract test passes).

## 3. The transition function

- [x] 3.1 Add `transition(&PaneSnapshot, &AgentEvent, &Reported) -> PaneWrites` for `SessionStart`, `UserPromptSubmit`, `Stop`, `StopFailure`, `Notification`, `PermissionDenied`, `CwdChanged`, `TaskCompleted` and `TeammateIdle`. (`ActivityLog` lands in 5.1; `SessionEnd`, `SubagentStart`, `SubagentStop` and `WorktreeRemove` keep their guarded plain-list/RMW paths; `TaskCreated` and `WorktreeCreate` write nothing.) Move the status resolvers from `status_priority.rs` next to it and correct the overstated guard comment on `sync_pane_location` in `context/location.rs` — it claims every pane-scoped write is skipped while subagents are active, but status, attention and prompt writes are not. Verify: unit tests give the option writes for each event with each of `idle`, `running`, `waiting`, `background` as the previous status, matching today's handler output except where the spec changes it — a reported-but-empty field clears (which changes `cwd`: today an absent or empty `cwd` is ignored and the stored value survives), and a pi `stop` keeps the worktree.
- [x] 3.2 Encode the subagent guard as one `identity_guarded` check over the set the spec names — agent label (unguarded today; the spec adds it), permission mode, session id, cwd, worktree, task reset, task-store clear, worktree-remove teardown, `SessionEnd`. Verify: tests for each scenario in "a parent's identity survives its subagents", and a test that status, attention and prompt writes still happen while subagents are active.
- [x] 3.3 Apply the reported-fields rule to `session_id`, `cwd` and the worktree name and branch. Verify: tests for each scenario in "a field that is not reported does not clear", including a pi `stop` that keeps the worktree and a Claude `notification` without `cwd` that clears it.
- [x] 3.4 Extract the status↔attention coupling into `status_writes(status) -> PaneWrites` — the single owner of the `clear`/`running`/`idle` attention handling `set_status` carries today — used by both `transition` and `cmd_set_status`. Verify: unit tests pin the write list for `clear`, `running`, `idle` and a plain `waiting` status.

## 4. Move the hook handlers onto the store

- [x] 4.1 Move `on_user_prompt_submit`, `on_stop`, `on_stop_failure` and `on_session_start` to `snapshot` → `transition` → `apply`. The non-option effects (notifications, the task reset, the activity log, `println!`) stay in the handler and run after `apply`; an effect that consults `@pane_subagents` (`mark_task_reset`, the teardown drains, the `SessionEnd` bail) must see the post-batch state — the Stop batch clears the stale list first, so the reset marker still lands. Verify: the existing handler tests pass, migrated to the in-memory adapter, and `on_stop` issues one write batch for the transition output.
- [x] 4.2 Move `on_notification`, `on_permission_denied` and `on_teammate_idle`. Verify: the existing attention tests pass, and a `wait_reason` with a newline is stored on one line.
- [x] 4.3 Move `on_subagent_start`, `on_subagent_stop`, the worktree handler and the pending-teardown helpers. The `@pane_subagents` read-modify-write stays as it is, expressed as `get` + `apply`; `mark_pending` and the pending-marker reads in `drain_pending_teardowns` go through the store. Verify: the existing subagent and teardown tests pass.
- [x] 4.4 Route the dispatch-inline writes and `set-status`: the `CwdChanged` arm runs `snapshot` → `transition` → `apply`; the `TaskCompleted` attention flag comes from `transition`; `cmd_set_status` applies `status_writes` through the tmux adapter. Verify: a `task-completed` event still sets the attention flag, and `set-status` keeps its set/clear behaviour.
- [x] 4.5 Thread `&dyn PaneStore` into the remaining option call sites — `notify_lifecycle`/`notify_desktop`/`notify_if_allowed`/`has_run_scoped_stamp`, `set_notification_run_id`/`notification_run_id`, `repo_label_from_pane`/`branch_label_from_pane` — so every `@pane_*` read or write in hook code goes through the store. Change `encode_stamp`/`parse_stamp` to a `:`-separated stamp so the encoder's `|` replacement leaves it intact. Verify: the notification, dedup and label tests pass on the in-memory store.
- [x] 4.6 Remove the per-handler `sanitize_tmux_value` calls, the `set_status` / `set_attention` helpers in `src/cli/mod.rs`, and `tmux::test_mock` once no caller is left. Verify: `cargo clippy` reports no dead code and `grep` finds no `set_pane_option`/`unset_pane_option`/`get_pane_option_value` outside `src/tmux/` and the sidebar-option paths (`src/cli/toggle.rs`, `src/main.rs`).

## 5. The activity-log rule

- [x] 5.1 Move the option writes of `handle_activity_log` into `transition`'s `ActivityLog` arm: the status move (`waiting` and `error` become `running`; `idle`, `background` and an absent status stay; `started_at` is set on the first move to `running`), the background-Bash `@pane_bg_cmd` write, and the plan-mode permission write, which keeps the subagent guard. Verify: one test per scenario in "a finished pane stays finished", plus tests for the bg-cmd and plan-mode writes.
- [x] 5.2 Make the handler apply the result as one batch and write the activity entry as before. Verify: `handle_activity_log` makes one `snapshot` and at most one `apply`, and the activity log still gets its entry in every scenario.

## 6. Sidebar sweeps and teardown

- [ ] 6.1 Express the bg-shell sweep in `state/refresh.rs` as `PaneWrites`: unset `bg_cmd`, and set `status = idle` only when the status is `background`. The sweep takes the store as an argument, constructed at `apply_session_snapshot` (tmux adapter in the sidebar, in-memory adapter in tests). Verify: tests for both scenarios in "the sidebar's own sweeps follow the same rules".
- [ ] 6.2 Express the three wipe paths (`clear_all_meta`, `clear_dead_agent_metadata`, `clear_agent_pane_state`) as one `Unset` batch each, keeping each path's own skip list; each takes the store like the sweep, threaded through `query_sessions_with_process_snapshot` the way `FeedEnv` is. Verify: existing wipe tests pass, and each wipe makes one `tmux` call.

## 7. Integration

- [ ] 7.1 Add a write-then-read test: apply a `Notification` with a multi-line `wait_reason` through the in-memory adapter, render that pane as a `list-panes` line, and parse it with the `query.rs` decoder. Verify: the pane is present and the reason is one line.
- [ ] 7.2 Update `docs/state-management.md` for the store, the snapshot and the batch. Verify: the doc names the store, the transition and the rules, and no longer describes handlers writing options one by one.
- [ ] 7.3 Run `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build --release` and `openspec validate pane-write-seam --strict`. Verify: all five exit with status 0.
- [ ] 7.4 Commit the work in concern-sized commits on `main` (seam and encoder, reported fields, transition and handlers, sweeps, docs) and confirm `git status` is clean. Verify: `git status` shows no changes.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
