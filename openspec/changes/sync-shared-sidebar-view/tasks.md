# Tasks

## 1. Read the shared options in the per-tick pane query

- [x] 1.1 Extend the `display-message` format in `get_sidebar_pane_info` (`src/tmux/panes.rs`) with `#{@sidebar_filter}`, `#{@sidebar_cursor}` and `#{@sidebar_repo_filter}`, with the repo filter as the last field. Move the output parse into a pure function that returns the four current values plus a map of the shared options, which leaves out empty fields and keeps spaces in the repo filter. Update the one caller in `src/state/refresh.rs`. Verify with unit tests for: all fields set, empty option fields (key not in the map), a repo name with spaces, and short or empty output (same defaults as today).

## 2. Apply the shared options on every tick

- [x] 2.1 In `AppState::refresh` (`src/state/refresh.rs`), give the shared options map to `GlobalState::apply_all` before `apply_session_snapshot`, so the `rebuild_row_targets` call that already runs in that tick uses the new filters. Put the apply step where a test can give it a map without tmux. Verify with a test in `tests/state_tests.rs`: a status filter written by another instance changes the visible rows after one refresh step.
- [x] 2.2 Remove `window_inactive_count`, the `load_from_tmux` call and the extra `rebuild_row_targets` call from the refresh block in `src/app.rs`. Keep the `size_guard::enforce` call gated on `is_window_active`, and keep `load_from_tmux` in `src/app/setup.rs`. Verify that `cargo build` and `cargo clippy` pass with no new warnings.
- [x] 2.3 Replace the "window activation sync tests" section of `tests/state_tests.rs` (`global_state_stable_during_task_completion`, `window_switch_syncs_after_debounce`, `window_active_flicker_does_not_trigger_sync`, `window_activation_syncs_all_fields`) with per-tick tests: a failed save is not reverted over several ticks, a pending cursor save is not reverted before it is flushed, a cursor and repo filter written by another instance are picked up in one tick, and an unchanged tmux value over many ticks leaves a local change alone. Verify that `cargo test` passes.
- [ ] 2.4 Update `docs/state-management.md`: the Global State section says the settings are read on every refresh tick (not "reloaded on SIGUSR1"), and the update-frequency diagram moves "GlobalState reloaded from tmux variables" from "On SIGUSR1" to the 1 second refresh row. Verify that the doc no longer says the read happens only on SIGUSR1 or window activation.

## 3. Integration checks

- [ ] 3.1 Run `cargo fmt --check`, `cargo clippy`, `cargo test` and `cargo build --release`. Verify that all four pass.
- [ ] 3.2 Do a manual check in tmux with the release build and two sessions, each with two windows that have a sidebar. Set the waiting filter in session A, switch the client to session B, and verify that its sidebar shows the waiting filter within 1 second. Then go from window 1 to window 2, change the filter, go back in less than 2 seconds, and verify that window 1 shows the new filter. Verify that the bottom tab of each sidebar stays as it was.
