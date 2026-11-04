# Tasks

## 1. Options and state plumbing

- [ ] 1.1 Add the `@sidebar_docked_height` constant in `src/tmux/options.rs`, and in `GlobalState` a `shared_docked_height: Option<u32>` field with its own latch beside the width latch — `save_shared_docked_height` follows the latch rules (only an accepted write advances `last_saved`; a rejected intent stays in the field), and `retry_pending_saves` retries both latches independently. Verify with unit tests mirroring the width-latch suite (rejected write keeps the latch, the two latches are independent, a retry reaches both).
- [ ] 1.2 Make the tick's shared-options read parse `@sidebar_docked_height` into the field each refresh, the same path `@sidebar_docked_width` rides. Verify with the options-map test used for the width option (unset/empty/non-numeric fall back to `None`).
- [ ] 1.3 Add `DEFAULT_MIN_STRIP_HEIGHT: u16 = 4` and a `strip_height_floor()` helper (constant, u16-clamped) beside `width_floor` in `src/state/width.rs`. Verify with a unit test of the helper.

## 2. The lock toggle gains the strip axis

- [ ] 2.1 Remove the strip early-return from `AppState::toggle_width_lock`: a strip enabling the lock seeds `shared_docked_height` with `max(live rows, floor)` — a floating strip seeds its captured docked height, and a floating strip with no capture, or a failed probe (`window_height == 0`), enables unseeded; the seed write runs before the flag write. Verify with unit tests: seed from a docked strip, seed the floor from a squeezed strip, seed from a floating strip with a capture, unseeded enable with no capture or a failed probe, a rejected seed leaving the flag write attempted, disable writing the flag alone.

## 3. The guard step gains the strip axis

- [ ] 3.1 Generalize `width_guard::step`/`enforce` per design D5: the step reads the pane's docked size on its axis (`width`/`window_width` for a column, `height`/`window_height` for a strip), the floor comes from the axis's rule (the width option floor, the 4-row constant for a strip), the cap is half the matching window dimension, publish goes through the axis's latch, and `enforce` issues `resize-pane -x` or `-y` from the layout. Verify with the width step's test suite mirrored on height: cold start primes without publishing, adoption both directions, cap and floor scenarios, the publisher's own resize uncapped, echo suppression and the short-landing retry, a squeeze below the floor not published, a floating strip and a failed probe as no-ops, a pending publish blocking the adopt side, and the tracker staying current while unlocked.

## 4. Creation starts at the shared height

- [ ] 4.1 In the strip branch of `cmd_toggle` (`src/cli/toggle.rs`), read the lock flag and `@sidebar_docked_height` once at creation and start the pane at the shared rows when the lock is on with a shared value present; unlocked, `resolve_height_arg` keeps governing. Verify with the column creation test mirrored (`shared_creation_height` pure function: lock off → `None`, lock on without a value → `None`, lock on with a value → the rows), plus a `split-window`/`resize-pane` argv assertion for the strip path.

## 5. Pop-in adopts the shared height

- [ ] 5.1 In the strip dock path of `state/popout.rs`, target `@sidebar_docked_height` while the lock is on and arm `pending_adopt_observation` for the landing, leaving the unlocked capture-restore unchanged. Verify with the strip pop-in tests: locked pop-in docks at the shared height and arms the suppressor, unlocked pop-in restores the captured height.

## 6. Documentation and integration gates

- [ ] 6.1 Update `docs/state-management.md`: the `@sidebar_docked_height` option row, the widened meaning of the lock flag, and the strip-side guard in the per-feature table. Verify the documented option name and default match the code.
- [ ] 6.2 Run `cargo fmt`, `cargo clippy`, and `cargo test`; then live-check with two strip windows: enable the lock in one, resize it, and confirm the other window's strip converges within a refresh tick and survives a toggle off/on.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
