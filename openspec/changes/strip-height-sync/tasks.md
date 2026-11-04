# Tasks

## 1. Options and state plumbing

- [x] 1.1 Add the `@sidebar_docked_height` constant in `src/tmux/options.rs`, and in `GlobalState` a `shared_docked_height: Option<u16>` field with its own latch beside the width latch — `save_shared_docked_height` follows the latch rules (only an accepted write advances `last_saved`; a rejected intent stays in the field), and `retry_pending_saves` retries both latches independently. For both axes, a cleared intent (`None`) writes `set -gu` through its latch instead of writing nothing (design D10). Verify with unit tests: a rejected height write keeps its latch, a retry reaches the height latch independently of the width latch, and a cleared value of either axis issues the unset and latches on rejection.
- [x] 1.2 Make the tick's shared-options read parse `@sidebar_docked_height` into the field each refresh, the same path `@sidebar_docked_width` rides. For both options an absent key now adopts as `None` (today a missing key adopts nothing, which would leave a cleared value cached on peers), and a non-numeric value still adopts nothing. Verify with the options-map tests: an absent key after an earlier value adopts `None`, a non-numeric value leaves the field unchanged, a number adopts its rows or columns.
- [x] 1.3 Add `DEFAULT_MIN_STRIP_HEIGHT: u16 = 4` and a `strip_height_floor()` helper (constant, u16-clamped) beside `width_floor` in `src/state/width.rs`. Verify with a unit test of the helper.

## 2. The lock toggle gains the strip axis

- [x] 2.1 Remove the strip early-return from `AppState::toggle_width_lock`: a strip enabling the lock seeds `shared_docked_height` with `max(live rows, floor)` — a floating strip seeds its captured docked height, and a floating strip with no capture, or a failed probe (`window_height == 0`), enables unseeded; the seed write runs before the flag write. Verify with unit tests: seed from a docked strip, seed the floor from a squeezed strip, seed from a floating strip with a capture, unseeded enable with no capture or a failed probe, a rejected seed leaving the flag write attempted, disable writing the flag alone. Enabling also clears the other axis (design D10) in the order seed → clear → flag (D8): a column enable clears `shared_docked_height`, a strip enable clears `shared_docked_width`, and the clear also runs on an unseeded enable. Verify with unit tests for each clear direction and the unseeded case.
- [x] 2.2 Add the strip lock control (design D9): `draw_strip_filter_bar` draws `☑`/`☐` with the column's lock styles at column 3 of the control row, and `handle_filter_column_click` maps column 3 of that row to `toggle_width_lock`. Verify with inline-snapshot tests of the strip filter bar locked and unlocked, and a click test that toggles the lock.

## 3. The guard step gains the strip axis

- [x] 3.1 Generalize `width_guard::step`/`enforce` per design D5: the step reads the pane's docked size on its axis (`width`/`window_width` for a column, `height`/`window_height` for a strip), the floor comes from the axis's rule (the width option floor, the 4-row constant for a strip), the cap is half the matching window dimension, publish goes through the axis's latch, and `enforce` issues `resize-pane -x` or `-y` from the layout. Verify only the axis selection on height — the step reads `height`/`window_height`, applies the 4-row floor and the half-window cap in rows, publishes through the height latch, and `enforce` issues `resize-pane -y` — plus one adopt and one publish end to end, and a floating strip and a failed probe (`window_height == 0`) as no-ops. The width suite keeps covering the axis-independent logic (echo suppression, publish-before-adopt ordering, the latch block, the tracker prime); do not mirror it.

## 4. Creation starts at the shared height

- [ ] 4.1 In the strip branch of `cmd_toggle` (`src/cli/toggle.rs`), read the lock flag and `@sidebar_docked_height` once at creation and start the pane at the shared rows when the lock is on with a shared value present; unlocked, `resolve_height_arg` keeps governing. Verify with the column creation test mirrored (`shared_creation_height` pure function: lock off → `None`, lock on without a value → `None`, lock on with a value → the rows), plus a `split-window`/`resize-pane` argv assertion for the strip path.

## 5. Pop-in adopts the shared height

- [ ] 5.1 In the strip dock path of `state/popout.rs`, target `@sidebar_docked_height` while the lock is on and arm `pending_adopt_observation` for the landing, leaving the unlocked capture-restore unchanged. Verify with the strip pop-in tests: locked pop-in docks at the shared height and arms the suppressor, unlocked pop-in restores the captured height.

## 6. Documentation and integration gates

- [ ] 6.1 Update `docs/state-management.md`: the `@sidebar_docked_height` option row, the widened meaning of the lock flag, the clear of the other axis on enable and the absent-key-adopts-`None` rule, the strip lock control, and the strip-side guard in the per-feature table. Verify the documented option name and default match the code.
- [ ] 6.2 Run `cargo fmt`, `cargo clippy`, and `cargo test`; then live-check with two strip windows: enable the lock in one with the strip lock glyph, resize it, and confirm the other window's strip converges within a refresh tick and survives a toggle off/on.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
