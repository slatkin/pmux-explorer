# Tasks

## 1. The percent as a shared setting

- [ ] 1.1 Add the percent parse (0 to 100 is the percent; unset, empty or any other value gives `DEFAULT_BOTTOM_PERCENT` = 30) and verify unit tests for `30`, `0`, `100`, `abc`, `150`, and an empty value
- [ ] 1.2 Add `GlobalState::bottom_percent` with `last_saved_bottom_percent`, `percent_save_pending`, `save_bottom_percent`, adoption in `apply_all`, and the retry in `retry_pending_saves`, as a copy of the bottom-tab latch (design decision 1); verify `global.rs` tests copied from the tab latch tests pass
- [ ] 1.3 Add `SIDEBAR_BOTTOM_HEIGHT` to `SHARED_OPTION_FIELDS` before `SIDEBAR_REPO_FILTER` and verify the `panes.rs` reading tests, updated for five options, pass

## 2. The split layout and the rule

- [ ] 2.1 Add `split_rows(percent, height)` and `percent_for_rows(rows, height)` (design decision 2) and verify unit tests with the spec numbers: 30% of 40 is 12, 30% of 60 is 18, 1% of 40 is 3, 100% of 40 is 37, height 5 is 0, 20 rows of 40 is 50%, 37 rows of 40 gives 37 rows again
- [ ] 2.2 Change `ui::draw` to use `split_rows` with `state.global.bottom_percent`, keep `clamp_bottom_height` after it while floating, remove the divider row, and take the pet band from the agent part only when it leaves 3 agent rows. In the same task remove `AppState::bottom_panel_height`, `BOTTOM_PANEL_HEIGHT`, and the startup read in `app/setup.rs`, and move every test that set a line count to `state.global.bottom_percent`. Verify that `FrameLayout::bottom_panel_height` is the drawn row count in a state test and that `cargo test` compiles (snapshot failures are fixed in 2.5)
- [ ] 2.3 Update `docs/state-management.md` so the percent is a shared option read every tick: add a `bottom_percent` row to the shared-options table (line 18), remove `bottom_panel_height` from the startup row (line 151), and add the bottom height to the `apply_all()` list (line 193); verify that `grep -n bottom_panel_height docs/state-management.md` finds nothing and that no text calls the bottom height a line count or a startup read
- [ ] 2.4 Change `draw_bottom` to draw only the rule (`─ ` + tabs + ` ` + `─` fill, accent color when the panel has focus) and give the content the full width under it; update the `top_line_border` unit test to the new rule text at widths 30, 22, 20 and 18 and verify it passes
- [ ] 2.5 Add inline snapshots for the rule with full-width content, for the focused rule, and for `bottom_percent = 0`; accept the existing snapshots that show the old box after a check that each diff shows only the box-to-rule change and the one extra agent-list row from the removed divider; verify `cargo test` passes
- [ ] 2.6 Delete `test_pet_enabled_preserves_bottom_panel_border` and verify `cargo test` passes

## 3. Mouse resize and tab click

- [ ] 3.1 Narrow the Activity hit area in `handle_bottom_tab_click` to the label columns 2–9 (design decision 3) and verify `tab.rs` tests: a click on columns 0 and 1 changes no tab, a click on column 2 selects Activity
- [ ] 3.2 Add `AppState::split_drag` and the press, drag and release handlers (design decision 4): a press on the rule starts a drag, a drag to another row resizes within the limits, a release after a move saves the percent, a release with no move calls `handle_bottom_tab_click` with the press column; verify state tests for a drag from row 28 to row 20 (percent 50), a drag to row 1 (stops at row 3), a click on `Git`, a click on the dash fill, and a drag that starts on a label
- [ ] 3.3 Make `apply_all` skip the shared percent while `split_drag` is set (the caller passes the drag state in), and end an active drag at its last rows when a new press arrives (a moved drag saves its rows, an unmoved press selects no tab); verify `GlobalState` and `AppState` tests for each case
- [ ] 3.4 Wire `Drag(Left)` and `Up(Left)` in `app/input.rs` and move the git-poller sync to the release path; verify that a click on the `Git` label in a state test sets the git tab, and that `cargo clippy` passes

## 4. Configuration and docs

- [ ] 4.1 Change `pmux-explorer.conf` so the comment says percent and the empty default is `30`; verify with `grep bottom_height pmux-explorer.conf`
- [ ] 4.2 Update `website/src/content/docs/reference/tmux-options.md` (default `30`, unit percent, `0` hides, mouse resize, unit change note) and `website/src/content/docs/features/activity-log.mdx` (drag the rule to resize; the tab list at line 52 says three tabs, Activity, Tasks and Git), and the `Shift+Tab` row in `website/src/content/docs/reference/keybindings.md` (Activity, Tasks, Git); verify the table row reads correctly and that `grep -rn "Activity ⇄ Git\|two tabs" website/src/content/docs` finds nothing
- [ ] 4.3 Update `fixtures/scenarios/common/_lib.sh` so the `BOTTOM_HEIGHT` comment says percent; verify with `grep BOTTOM_HEIGHT fixtures/scenarios/common/_lib.sh`

## 5. Integration

- [ ] 5.1 Run `cargo fmt --check`, `cargo clippy`, `cargo test` and `cargo build --release`, and verify that all pass
- [ ] 5.2 In tmux with two windows, drag the rule in one sidebar, release, and verify that the other sidebar shows the same percent within 1 second, that a click on a tab label still selects the tab, and that `tmux show -g @sidebar_bottom_height` shows the new percent

## Workflow follow-up

- Run `openspec validate --change "split-bottom-panel" --strict` and fix any
  reported issue.
- Archive the change after the project's review requirements are
  satisfied, and verify the archived result (`openspec archive` creates
  `openspec/specs/bottom-panel/` and updates
  `openspec/specs/shared-view-settings/` with the deltas).
