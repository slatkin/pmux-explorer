# Tasks

## 1. Shared option plumbing

- [x] 1.1 Add the `SIDEBAR_WIDTH_LOCK` (`@sidebar_width_lock`) and
  `SIDEBAR_DOCKED_WIDTH` (`@sidebar_docked_width`) global option constants
  in `src/tmux/options.rs` beside `SIDEBAR_BOTTOM_TAB`, and verify with
  `cargo build` that it compiles.
- [x] 1.2 Insert `SIDEBAR_DOCKED_WIDTH`, `SIDEBAR_WIDTH_LOCK` and the
  existing `SIDEBAR_MIN_WIDTH` into `SHARED_OPTION_FIELDS` in
  `src/tmux/panes.rs` before `SIDEBAR_REPO_FILTER` (the repo filter must
  stay last for its `/eot/` terminator parse) and extend the
  `parse_sidebar_pane_reading` zip to the seven-field layout — the floor
  must ride the per-tick read because the lock needs it in hidden windows
  too; verify with `cargo test parse_sidebar_pane` that existing cases
  pass plus a new case asserting `options[SIDEBAR_DOCKED_WIDTH]`,
  `options[SIDEBAR_WIDTH_LOCK]` and `options[SIDEBAR_MIN_WIDTH]`
  round-trip a seven-field line (`45`, `on`, `20`), are absent when
  unset, and the repo filter still keeps spaces. Migrate the existing
  four-option fixtures in the same tests to the new layout — "existing
  cases pass" means updated to the seven-field wire shape, not kept as
  they are.

## 2. Shared width state in GlobalState

- [x] 2.1 Add `width_lock: bool` and `shared_docked_width: Option<u16>`
  to `GlobalState` (`src/state/global.rs`) — `shared_` because
  `AppState.docked_width` already names the pop-out capture — with
  `last_saved_width_lock`, `last_saved_shared_docked_width` and
  `width_save_pending` / `lock_save_pending` latches; add
  `save_shared_docked_width`, `save_width_lock`, and
  `toggle_width_lock` (enable writes the floored seed — `max(W, floor)`
  docked, the captured `docked_width` floored while floating, or
  `PANEL_WIDTH` with no capture — then `on`; disable writes `off`),
  seeding only on a valid probe (`sidebar_pane.window_width == 0` means
  the last reading was the FALLBACK, so the enable writes the lock flag
  alone rather than seeding the fallback's 28 columns); verify with unit
  tests in the filter-latch shape that a rejected write leaves
  `last_saved_*` at the old value and sets the latch.
- [x] 2.2 Extend `apply_all` to adopt `SIDEBAR_WIDTH_LOCK` (`on` → locked),
  `SIDEBAR_DOCKED_WIDTH` (parsed `u16`) under the `last_saved_*` guards
  (skipping adoption while the matching latch is set) plus
  `SIDEBAR_MIN_WIDTH` (parsed `u32`, same parse as `size_guard`'s) into a
  plain `min_width` field — adopted unconditionally since nothing ever
  writes it and it needs no latch, and RESET to unset when the key is
  absent so `set -gu @sidebar_min_width` falls back to
  `DEFAULT_MIN_WIDTH` exactly like today's per-tick `get_option` — and
  add `retry_pending_width_save` / `retry_pending_lock_save` to
  `retry_pending_saves`; verify with unit tests that (a) a peer's
  lock/width is adopted and the floor parses, (b) a local value whose
  write failed is not reverted, (c) a missing key leaves the defaults
  (unlocked, no shared width — spec scenario "a first instance on a
  fresh server"), and (d) a tick without `SIDEBAR_MIN_WIDTH` resets
  `min_width` to unset.
- [x] 2.3 Confirm `AppState::refresh` (`src/state/refresh.rs`) calls the
  extended `retry_pending_saves` and `apply_all` in the existing
  retry-then-read order; verify with `cargo test state_tests` that refresh
  tests still pass.

## 3. The publish/adopt tick step

- [x] 3.1 Add `src/app/width_guard.rs` with a pure decision function plus
  a thin `enforce` wrapper, implementing design D4: `last_observed_width:
  Option<u16>` PRIMED on the first docked reading (a `None` tracker
  records `W` and never publishes — a fixed init would publish a
  tmux-clamped creation width server-wide on the first locked tick) and
  updated every docked tick locked or not (a sleeping tracker would
  publish a diverged width over the seed the tick the lock arrives);
  `pending_adopt_observation` one-shot flag set on each issued
  `resize-pane` and evaluated EVERY tick the step runs — consumed by the
  first observed width change (never published), cleared unconsumed the
  next tick (refused resize), so a flag set by an unlocked pop-in cannot
  linger into a later enable; `target = min(max(S, floor), max(floor,
  window_width / 2))`; publish assigns `shared_docked_width = W` BEFORE
  the adopt check reads `S` (ordering is load-bearing); adopt via
  `resize-pane -x` on `W != target && W != S` ONLY while
  `width_save_pending` is clear — the `W != S` term exempts the
  publisher's own pane from the cap, and the latch check stops the adopt
  side from reverting a pane whose publish write failed (spec
  requirement "a failed width publish is not reverted"); no-op on
  `window_width == 0` or `floating`; verify with unit tests mirroring
  `size_guard`'s test style: cold start never publishes, manual resize
  publishes, a squeeze below the floor does not, an adoption landing
  does not echo back (exact target AND short landing both suppressed), a
  refused resize frees suppression for the next manual resize, a
  100-column window caps a shared 80 at 50 without clobbering the shared
  value, a shared 8 under a floor of 20 adopts 20, a publisher's own
  70-column resize on a 100-column window is not clamped by its own
  adopt step, a narrower shared width shrinks the pane, a pending
  publish blocks the adopt side, floating panes are skipped, and
  `window_width == 0` is a no-op.
- [x] 3.2 Wire the step into `app::run` after `state.refresh()`, running
  for every docked pane every tick regardless of `is_window_active`
  (hidden windows must converge before the user switches). `size_guard`
  keeps running UNCHANGED inside its `is_window_active` gate, locked or
  not — the locked-plus-unseeded state has no `S` to adopt, so the lock
  step alone could never recover a squeeze there (spec scenario "a
  squeeze with no shared width still recovers"); order the lock step
  after the `is_window_active` block so that when both fire, the adopt
  `resize-pane` lands last and `target` (never below the floor) wins.
  Both guards read `@sidebar_min_width` from the tick's options map so
  `size_guard`'s own `show -gv` call goes away; verify with `cargo test`
  and a reading-through of the refresh block that floating panes still
  reach `panel_guard` unchanged.

## 4. The lock control

- [x] 4.1 Bind the `w` key in `src/app/input.rs` to
  `state.toggle_width_lock()`; verify with a handle_key_event test that
  `w` flips `state.global.width_lock` intent (field set, save latch armed
  on tmux rejection — the testable contract of the other shared writes).
- [x] 4.2 Render a lock glyph beside the pop-out control in
  `src/ui/panes/filter_bar.rs`'s `render_filter_bar` (distinct glyphs
  for locked/unlocked) and extend the control column in
  `filter_bar_layout` / `handle_filter_click` (`src/state/layout.rs`)
  so a click on it calls the same `toggle_width_lock` — the pop-out
  control lives in the filter bar, not the status row; verify with
  `insta::assert_snapshot!` cases for both states per the UI test rule,
  plus a click-hitbox test asserting the glyph toggles on click. The
  existing `control_col = width - 2` layout/click fixtures shift by one
  cell for the second control — update them in this task.
- [ ] 4.3 Add the `w` key and the lock control to
  `website/src/content/docs/reference/keybindings.md` and
  `website/src/content/docs/features/floating-panel.mdx`; verify the
  listed keys match `src/app/input.rs`.

## 5. Pop-in restore and sidebar creation

- [ ] 5.1 Update `apply_pop_in` in `src/state/popout.rs` to restore
  `min(max(S, floor), cap)` when the lock is on and
  `global.shared_docked_width` is set — clamped, so a narrow window does
  not dock oversized for a tick — falling back to the captured
  `docked_width` otherwise, and set `pending_adopt_observation` so the
  restore's landing does not echo as a publish; verify with unit tests
  beside the existing pop-out tests: locked restores 45 from the shared
  value (and clamps), unlocked keeps the captured-25 scenario.
- [ ] 5.2 Update `cmd_toggle` in `src/cli/toggle.rs` to read
  `#{@sidebar_width_lock}` and `#{@sidebar_docked_width}` first and use
  the shared width for `split-window -l` when locked, else the existing
  `@sidebar_width` path; verify by extracting the width-resolution into a
  small helper and unit-testing locked/unlocked/unset-width cases.

## 6. Documentation

- [ ] 6.1 Update `docs/state-management.md`: `width_lock` and
  `docked_width` join the Global scope table with their tmux variables
  and update triggers, and the width latches are documented beside the
  filter/tab ones; verify `rg 'docked_width|width_lock' docs` shows the
  rows.
- [ ] 6.2 Document `@sidebar_width_lock` and `@sidebar_docked_width` in
  the `agent-sidebar.conf` comment block and in
  `website/src/content/docs/reference/tmux-options.md`; verify the option
  names match `src/tmux/options.rs` exactly.

## 7. Integration verification

- [ ] 7.1 Run `cargo fmt`, then `cargo test` and `cargo clippy`, and
  verify all pass with no warnings.
- [ ] 7.2 Run `cargo build --release` and verify the release binary builds,
  then smoke test with two windows: enable the lock with `w` in one,
  resize its sidebar, and confirm the other window's sidebar matches
  within ~1 second while hidden; then pop one out and confirm the panel
  stays 40 columns and pops back in at the shared width.

## Workflow follow-up

- Run `openspec validate --change "2026-10-23-sidebar-width-lock" --strict`
  and fix any reported issue.
- Archive the change after the project's review requirements are
  satisfied, and verify the archived result (`openspec archive` updates
  `openspec/specs/shared-view-settings/` and
  `openspec/specs/sidebar-panel/` with the deltas).
