# Tasks

## 1. Shared option plumbing

- [ ] 1.1 Add the `SIDEBAR_BOTTOM_TAB` (`@sidebar_bottom_tab`) global
  option constant in `src/tmux/options.rs` beside `SIDEBAR_FILTER`, and
  verify with `cargo build` that it compiles.
- [ ] 1.2 Insert `SIDEBAR_BOTTOM_TAB` into `SHARED_OPTION_FIELDS` in
  `src/tmux/panes.rs` before `SIDEBAR_REPO_FILTER` (the repo filter must
  stay last for its terminator parse), and verify with `cargo test
  parse_sidebar_pane` that the reading parse still passes its existing
  tests plus a new case asserting `options[SIDEBAR_BOTTOM_TAB]` round-trips
  a four-field line (`activity`, `tasks`, `git`) and is absent when unset.

## 2. Shared tab state in GlobalState

- [ ] 2.1 Move `bottom_tab` into `GlobalState` (`src/state/global.rs`) as
  `bottom_tab: BottomTab` with `last_saved_bottom_tab` and a
  `tab_save_pending` latch; add `save_bottom_tab` writing `set -g
  @sidebar_bottom_tab`, and verify with a new unit test that a rejected
  write leaves `last_saved_bottom_tab` at the old value and sets the latch
  (mirroring the existing `a_pending_filter_save_is_not_overwritten_by_the_shared_value`
  test shape).
- [ ] 2.2 Extend `apply_all` to adopt `SIDEBAR_BOTTOM_TAB` under the
  `last_saved_bottom_tab` guard, skipping adoption while
  `tab_save_pending`, and add `retry_pending_tab_save`; verify with unit
  tests that (a) a peer's `tasks` value is adopted, (b) a local value whose
  write failed is not reverted, and (c) a successful retry clears the latch
  so the next read adopts.
- [ ] 2.3 Rewire `AppState::refresh` (`src/state/refresh.rs`) to call
  `retry_pending_tab_save` alongside `retry_pending_filter_save` before
  `apply_all`, and verify with `cargo test state_tests` that refresh tests
  still pass (no behavior change yet for writers).

## 3. Tab writers and readers on the shared field

- [ ] 3.1 Update `next_bottom_tab` and `handle_bottom_tab_click`
  (`src/state/tab.rs`) to write `state.global.bottom_tab` and call
  `save_bottom_tab`, and update the `git_tab_active` stores in
  `src/app/input.rs` and `src/app.rs` to read the new location; verify with
  `cargo test bottom_tests` after migrating those tests to
  `state.global.bottom_tab` (the cycle and hitbox tests must pass
  unchanged in behavior).
- [ ] 3.2 Update the render path (`src/ui/bottom.rs` title/content match)
  and `src/app/workers.rs` to read `state.global.bottom_tab`; verify with
  `cargo test ui_snapshot` and `cargo test styled_tests` after migrating
  their `state.bottom_tab = …` assignments to
  `state.global.bottom_tab = …`.
- [ ] 3.3 Add a cross-instance test: instance A sets `tasks`, a reading
  with `@sidebar_bottom_tab=tasks` applied through `apply_all` makes
  instance B show Tasks within one tick; verify with a new test in
  `tests/state_tests.rs` (spec scenario "a bottom tab change applies to
  another window").

## 4. Remove the focus-driven auto-switch

- [ ] 4.1 Delete `auto_switch_tab`, `TabDecision`, `resolve_bottom_tab`,
  `resolve_tab_for_focused_pane`, `save_current_tab` and
  `detect_new_agents` from `src/state/tab.rs`, and drop the
  `auto_switch_tab()` call from `refresh_activity_data`
  (`src/state/refresh.rs`); verify with `cargo test` that only
  auto-switch-specific tests fail and fix them in 4.3.
- [ ] 4.2 Remove `PaneRuntimeState.tab_pref` (`src/state/pane_runtime.rs`)
  and `PaneRuntimeMap.seen` plus its inserts/removals; verify with
  `cargo build` that no remaining reference to `tab_pref` or `seen`
  compiles (`rg 'tab_pref|\.seen' src` returns no code hits).
- [ ] 4.3 Replace the auto-switch and per-pane memory tests in
  `tests/state_tests.rs`, `tests/bottom_tests.rs` and
  `src/state/tab.rs`'s test module with shared-tab behavior tests: focusing
  a shell pane keeps the tab, focusing an agent keeps the tab, a new agent
  in the focused pane keeps the tab (spec scenarios under "focusing a
  different agent does not change the bottom tab"); verify with
  `cargo test state_tests bottom_tests` passing.
- [ ] 4.4 Update the `BottomTab` doc comment in `src/state.rs` and the
  `PaneRuntimeState` doc so neither mentions auto-switch or per-pane
  preferences; verify with `rg 'auto.switch|tab_pref' src docs` returning
  only intentional history mentions.

## 5. Startup default and empty-value behavior

- [ ] 5.1 Confirm and pin that an instance with no shared value keeps the
  `Activity` default from `AppState::new` and that an empty
  `#{@sidebar_bottom_tab}` expansion is dropped from the options map
  (never adopted as a tab); verify with a unit test asserting
  `apply_all` with no `SIDEBAR_BOTTOM_TAB` key leaves `bottom_tab` on
  Activity (spec scenario "a first instance on a fresh server").

## 6. Documentation

- [ ] 6.1 Update `docs/state-management.md`: move `bottom_tab` into the
  Global State table with its `@sidebar_bottom_tab` variable and update
  trigger, document the `tab_save_pending` latch beside the filter's,
  remove the `tab_pref` rows from the Per-pane table and the Local table's
  `bottom_tab` row, rewrite invariant 3 (per-pane tab restore) into the
  shared-tab invariant, and adjust the refresh data-flow arrows; verify by
  re-reading the rendered file and checking no `tab_pref`/auto-switch row
  remains (`rg 'tab_pref' docs` returns nothing).

## 7. Integration verification

- [ ] 7.1 Run `cargo fmt`, then `cargo test` and `cargo clippy` and
  verify all pass with no warnings.
- [ ] 7.2 Run `cargo build --release` and verify the release binary builds
  (the plugin symlink picks it up automatically for a manual two-window
  smoke test: select Tasks in one window, confirm the other window shows
  Tasks within 1 second, then click a different agent and confirm the tab
  does not move).

## Workflow follow-up

- Run `openspec validate --change "sync-bottom-tab" --strict` and fix any
  reported issue.
- Archive the change after the project's review requirements are
  satisfied, and verify the archived result (`openspec archive` updates
  `openspec/specs/shared-view-settings/` and `openspec/specs/agent-tasks/`
  with the deltas).
