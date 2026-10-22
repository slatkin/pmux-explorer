# Tasks

## 1. Task store and shell write path

- [x] 1.1 Add per-pane task store module (read/write/list/update, `$TMUX_PANE` encoding mirroring activity log path, item cap with completed-first trim, session-start clear) and verify with unit tests covering record, update, trim, and missing-pane-env paths.
- [x] 1.2 Add `task-set` / `task-update` CLI subcommands on the `run` dispatch that write to the calling pane's store and exit 0 on every path, and verify with CLI tests including the no-`$TMUX_PANE` case.
- [x] 1.3 Wire the store into refresh with the same mtime-check pattern as the activity log, and verify with refresh unit tests for fresh, stale, and cleared stores.

## 2. Tasks tab UI

- [x] 2.1 Extend `BottomTab` with Tasks, three-way cycle, title layout, click hitboxes, scroll dispatch with its own offset, and per-pane tab memory with manual-only auto-switch, and verify with state/tab unit tests.
- [x] 2.2 Render the Tasks list (status icons plus subject, empty state, truncation) for the focused pane in a new `bottom/tasks.rs`, and verify with `insta` snapshot tests for populated, empty, and narrow-width frames.
- [x] 2.3 Update key and mouse input paths (`BackTab` cycling, click-to-tab, scroll routing, focus) for three tabs, and verify with input/layout unit tests.

## 3. Reminders and docs

- [x] 3.1 Add the single-source reminder wording module (record-first and update-before-finish) surfaced through task-command help, and verify the help output contains both strings verbatim.
- [x] 3.2 Update `docs/agent-protocol.md` task section and the pi producer contract note that pi reports no tasks, and verify the doc states the shell command, fields, and reminder wording.
- [x] 3.3 File the pmux-extension companion change for input/settle-time reminder delivery reusing the same wording, and verify the issue text names the wording source file.

## 4. Pi proof and integration

- [x] 4.1 Prove end to end with a live pi pane: pi records tasks, updates a status, and the Tasks tab shows each step, and verify by attaching the observed tab output for the empty, recorded, and updated states.
- [x] 4.2 Run `cargo fmt`, `cargo clippy`, `cargo test`, and `openspec validate --strict` for the change, and verify all gates pass.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived specs merged into the main specs.
