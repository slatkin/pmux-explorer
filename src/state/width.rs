//! The width lock's enable/disable action.
//!
//! One method — [`AppState::toggle_width_lock`] — is the single writer
//! behind the `w` key and both lock glyphs' clicks (the column's
//! filter-row control and the strip's filter-bar control). It flips
//! [`GlobalState::width_lock`] and persists the intent to the shared
//! tmux options: the enabling axis's seed first, the other axis's
//! clear second, the lock flag last, so a peer adopting `lock=on`
//! mid-tick always finds its axis already seeded or cleared (design
//! D8, D10).

use super::AppState;
use crate::app::size_guard::DEFAULT_MIN_WIDTH;
use crate::state::popout::PANEL_WIDTH;

impl AppState {
    /// Flip the sidebar size lock and persist the intent to the shared
    /// tmux options. The flag is one server-wide lock for both axes: a
    /// column seeds the shared docked width, a strip the shared docked
    /// height, each with the floored docked size of this instance on
    /// its own axis — `max(size, floor)` from this pane's reading, the
    /// captured `docked_size` floored while floating, or (a column
    /// only) `PANEL_WIDTH` when no capture exists (a pane floated from
    /// the tmux command line) — and never the live panel size. A
    /// floating strip with no capture has no fixed size to fall back
    /// to and enables unseeded (design D3).
    ///
    /// The seed is written only on a valid probe: a `window_width` of
    /// 0 (a column) or a `window_height` of 0 (a strip) means the last
    /// reading was the FALLBACK (the query failed or the pane is gone)
    /// and its size is not a real size, so the lock is enabled
    /// unseeded rather than propagated.
    ///
    /// Enabling also clears the other axis's shared value (design
    /// D10): the field becomes `None` and its latch writes `set -gu`,
    /// seeded or not — a stale value from an earlier lock session
    /// would make the other axis's peers jump to it. Disabling writes
    /// the `off` flag alone; the stale shared values stay behind
    /// harmlessly and are reused when the lock returns.
    ///
    /// Every write runs even if the previous one fails: a stubbornly
    /// failing writer must not leave every instance locking onto (or
    /// seeding over, or clearing and keeping) a value nobody wrote.
    pub fn toggle_width_lock(&mut self) {
        if !self.global.width_lock {
            // Design D8's order: the enabling axis's seed first, the
            // other axis's clear (D10) second, the lock flag last — a
            // peer adopting `lock=on` mid-tick must find the enabling
            // axis already seeded and the other already cleared.
            match self.layout_kind {
                crate::state::LayoutKind::Column => {
                    // Seed only on a valid probe — see the doc comment.
                    if self.sidebar_pane.window_width > 0 {
                        // The u16-clamped floor that `@sidebar_min_width`
                        // resolves to — the shared `width_floor` helper,
                        // also used by the width guard and the pop-in
                        // restore.
                        let floor = width_floor(self.global.min_width);
                        let live_width = if self.sidebar_pane.floating {
                            self.docked_size.unwrap_or(PANEL_WIDTH)
                        } else {
                            self.sidebar_pane.width
                        };
                        let seed = live_width.max(floor);
                        self.global.shared_docked_width = Some(seed);
                        self.global.save_shared_docked_width();
                    }
                    self.global.shared_docked_height = None;
                    self.global.save_shared_docked_height();
                }
                crate::state::LayoutKind::Strip => {
                    // Seed only on a valid probe — see the doc comment.
                    // The floor is the strip-height constant, not an
                    // option (design D3).
                    if self.sidebar_pane.window_height > 0 {
                        let live_height = if self.sidebar_pane.floating {
                            self.docked_size
                        } else {
                            Some(self.sidebar_pane.height)
                        };
                        if let Some(live_height) = live_height {
                            let seed = live_height.max(strip_height_floor());
                            self.global.shared_docked_height = Some(seed);
                            self.global.save_shared_docked_height();
                        }
                    }
                    self.global.shared_docked_width = None;
                    self.global.save_shared_docked_width();
                }
            }
            self.global.width_lock = true;
        } else {
            self.global.width_lock = false;
        }
        self.global.save_width_lock();
    }
}

/// The width-lock floor for the tick's parsed `@sidebar_min_width` in
/// columns ([`GlobalState::min_width`]). `None` falls back to
/// [`DEFAULT_MIN_WIDTH`], and a floor beyond pane geometry is clamped
/// into the u16 range — the u32 option value cannot silently wrap in
/// the u16 math of the three users below (seed, adopt, pop-in).
pub(crate) fn width_floor(min_width: Option<u32>) -> u16 {
    min_width.unwrap_or(DEFAULT_MIN_WIDTH).min(u16::MAX as u32) as u16
}

/// The minimum docked strip height in rows. The strip floor is the
/// fixed constant, not an option: the floor's job is stopping
/// collapse, not readability, and a strip has no
/// `@sidebar_min_height` option (design D3 of the strip-height-sync
/// change). It binds the lock's seed, the guard's publishes and
/// adoptions, and the pop-in restore, and wins over the adoption cap
/// where the two cannot both hold — mirroring `width_floor`.
pub const DEFAULT_MIN_STRIP_HEIGHT: u16 = 4;

/// The strip-height floor for the lock's math — the height-axis twin
/// of [`width_floor`]. The strip floor is constant, so the u16 value
/// needs no clamp; the helper keeps the axis rules uniform at the
/// call sites the width option's floor and this constant feed.
pub fn strip_height_floor() -> u16 {
    DEFAULT_MIN_STRIP_HEIGHT
}

/// The size an adopting sidebar resizes to with `resize-pane`: the
/// shared docked size clamped to at least `floor` — the floor binds
/// both ways — and at most `cap`, normally half the adopting window's
/// matching dimension. The cap binds adopted sizes only; a publishing
/// sidebar may stay past half its window. The floor comes resolved:
/// the width option's floor on the column axis, the strip-height
/// constant on the strip axis. The two users per axis: the lock
/// step's adopt side (`app/width_guard::step`, design D5) and the
/// pop-in restore (`state/popout::pop_in_restore_size`, design D6).
pub(crate) fn adopt_target(shared: u16, floor: u16, cap: u16) -> u16 {
    shared.max(floor).min(cap)
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn width_floor_none_falls_back_to_default() {
        assert_eq!(width_floor(None), DEFAULT_MIN_WIDTH as u16);
        assert_eq!(width_floor(Some(24)), 24);
        assert_eq!(width_floor(Some(u32::MAX)), u16::MAX, "clamped at u16::MAX");
    }

    #[test]
    fn adopt_target_floors_and_caps() {
        assert_eq!(adopt_target(80, 20, 100), 80);
        assert_eq!(adopt_target(8, 20, 100), 20, "the floor binds both ways");
        assert_eq!(adopt_target(80, 20, 50), 50, "the cap clamps adopted sizes");
    }

    #[test]
    fn strip_height_floor_is_the_documented_constant() {
        assert_eq!(strip_height_floor(), 4);
        assert_eq!(strip_height_floor(), DEFAULT_MIN_STRIP_HEIGHT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GlobalState;
    use crate::tmux::SidebarPaneInfo;

    fn docked_pane(width: u16, window_width: u16) -> SidebarPaneInfo {
        SidebarPaneInfo {
            pane_active: true,
            window_active: true,
            width,
            window_width,
            window_height: 40,
            floating: false,
            height: 40,
            at_top: false,
        }
    }

    #[test]
    fn enabling_docks_seed_to_the_pane_width_floored() {
        let mut state = AppState::new("%99".into());
        state.global.min_width = Some(20);
        state.sidebar_pane = docked_pane(45, 120);

        state.toggle_width_lock();

        // W above the floor: the seed is the pane's own width.
        assert_eq!(state.global.shared_docked_width, Some(45));
        assert!(state.global.width_lock);
        // The rejections prove the enable attempt wrote BOTH options:
        // the width seed ran before the lock flag (D5 order), so this
        // pane's squeeze-free width is on the wire for whichever peer
        // next adopts lock=on.
        assert!(state.global.width_latch.save_pending);
        assert!(state.global.lock_latch.save_pending);
        assert!(
            !state.global.lock_latch.last_saved,
            "a rejected flag write must not advance the guard"
        );
    }

    #[test]
    fn enabling_seeds_the_floor_when_the_pane_is_squeezed_below_it() {
        // A squeezed pane never propagates its squeeze: the seed is
        // `max(W, floor)`, so the enabling instance itself gets grown
        // back by the adopt path to the floor it just wrote.
        let mut state = AppState::new("%99".into());
        state.global.min_width = Some(20);
        state.sidebar_pane = docked_pane(8, 120);

        state.toggle_width_lock();

        assert_eq!(state.global.shared_docked_width, Some(20));
    }

    #[test]
    fn enabling_while_floating_seeds_the_captured_docked_width_floored() {
        // `w` is reachable while floating; the seed must be the docked
        // width captured on the way out, never the live 40-column
        // panel width.
        let mut state = AppState::new("%99".into());
        state.global.min_width = Some(20);
        state.sidebar_pane = SidebarPaneInfo {
            floating: true,
            ..docked_pane(PANEL_WIDTH, 80)
        };
        state.docked_size = Some(55);

        state.toggle_width_lock();

        assert_eq!(state.global.shared_docked_width, Some(55));
    }

    #[test]
    fn enabling_while_floating_with_no_capture_seeds_the_panel_width() {
        // A pane floated from the tmux command line has nothing to
        // restore; `PANEL_WIDTH` is the deterministic seed there.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane = SidebarPaneInfo {
            floating: true,
            ..docked_pane(PANEL_WIDTH, 80)
        };

        state.toggle_width_lock();

        assert_eq!(state.global.shared_docked_width, Some(PANEL_WIDTH));
    }

    #[test]
    fn enabling_on_a_failed_probe_enables_unseeded() {
        // `window_width == 0` marks the FALLBACK reading (query failed
        // or pane gone): its 28 columns are not a real size and must
        // not become the server's shared width.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane = SidebarPaneInfo::FALLBACK;
        assert_eq!(state.sidebar_pane.window_width, 0);
        let before = state.global.shared_docked_width;

        state.toggle_width_lock();

        assert!(state.global.width_lock, "the lock itself is still enabled");
        assert_eq!(
            state.global.shared_docked_width, before,
            "the width stays unseeded on a failed probe"
        );
        assert!(
            state.global.lock_latch.save_pending,
            "the flag write is still attempted"
        );
        assert!(
            !state.global.width_latch.save_pending,
            "no width write was ever attempted"
        );
    }

    #[test]
    fn a_rejected_enable_seed_latches_and_leaves_the_flag_write_running() {
        // In tests tmux rejects every write. An enable must apply BOTH
        // writes: a width write that fails alone must not stop the
        // lock flag from going out.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane = docked_pane(45, 120);

        state.toggle_width_lock();

        assert!(state.global.width_latch.save_pending);
        assert!(state.global.lock_latch.save_pending);
        assert_eq!(
            state.global.width_latch.last_saved, None,
            "a rejected seed write must not advance the guard"
        );
        assert_eq!(
            state.global.shared_docked_width,
            Some(45),
            "the intent stays in the field for the retry"
        );
    }

    #[test]
    fn disabling_writes_the_flag_alone() {
        let mut state = AppState::new("%99".into());
        state.global.width_lock = true;
        state.global.shared_docked_width = Some(45);
        state.sidebar_pane = docked_pane(45, 120);

        state.toggle_width_lock();

        assert!(!state.global.width_lock);
        assert_eq!(
            state.global.shared_docked_width,
            Some(45),
            "unlocking leaves the stale shared width behind (reused on re-enable)"
        );
        assert!(
            !state.global.width_latch.save_pending,
            "no width write happens on disable"
        );
    }

    #[test]
    fn toggling_follows_the_field_the_adoption_may_have_set() {
        // The toggle branches on the field, not on a local shadow: a
        // peer's adopted `on` makes the next toggle a disable, the same
        // contract the pop-out follows by reading the pane's mode.
        let mut state = AppState::new("%99".into());
        state.global.width_lock = true; // adopted or written earlier

        state.toggle_width_lock();
        assert!(!state.global.width_lock);

        state.toggle_width_lock();
        assert!(state.global.width_lock);
    }

    #[test]
    fn a_rejected_write_leaves_last_saved_at_the_old_value() {
        // Filter-latch shape, driven through both width-side saves (the
        // width and lock latches ride the same `Latch::record_write`):
        // only an accepted write may advance the `last_saved` guard; the
        // rejected intent stays in the field for the retry over the next
        // ticks.
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);

        g.save_shared_docked_width();

        assert_eq!(
            g.width_latch.last_saved, None,
            "only an accepted write may advance the width latch's last_saved"
        );
        assert!(g.width_latch.save_pending);
        assert_eq!(
            g.shared_docked_width,
            Some(45),
            "a rejected write must not drop the intent"
        );

        let mut g = GlobalState::new();
        g.width_lock = true;

        g.save_width_lock();

        assert!(
            !g.lock_latch.last_saved,
            "only an accepted write may advance the lock latch's last_saved"
        );
        assert!(g.lock_latch.save_pending);
    }

    #[test]
    fn retry_pending_saves_keep_the_latch_while_tmux_rejects() {
        // A retried write that tmux rejects again keeps the latch (the
        // next tick retries again) and leaves both the guard and the
        // live field untouched — through either width-side retry.
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);
        g.width_latch.save_pending = true;

        g.retry_pending_saves();

        assert!(
            g.width_latch.save_pending,
            "a rejected retry keeps the latch set"
        );
        assert_eq!(g.width_latch.last_saved, None);
        assert_eq!(
            g.shared_docked_width,
            Some(45),
            "the retried intent stays in the field"
        );

        let mut g = GlobalState::new();
        g.width_lock = true;
        g.lock_latch.save_pending = true;

        g.retry_pending_saves();

        assert!(
            g.lock_latch.save_pending,
            "a rejected retry keeps the latch set"
        );
        assert!(!g.lock_latch.last_saved);
    }

    #[test]
    fn the_two_width_latches_are_independent() {
        // The unified retry touches only pending latches, so a retry
        // that leaves one latch pending cannot have cleared the other.
        let mut g = GlobalState::new();
        // A width must sit behind the pending flag: the `None` guard in
        // `save_shared_docked_width` clears the flag without a write,
        // which is the behavior this test is not about.
        g.shared_docked_width = Some(45);
        g.width_latch.save_pending = true;

        g.retry_pending_saves();

        assert!(
            g.width_latch.save_pending,
            "a rejected retry keeps the width latch"
        );
        assert!(
            !g.lock_latch.save_pending,
            "the width retry must not flag the lock latch"
        );

        g.lock_latch.save_pending = true;
        g.width_latch.save_pending = false;
        g.retry_pending_saves();

        assert!(
            g.lock_latch.save_pending,
            "a rejected retry keeps the lock latch"
        );
        assert!(
            !g.width_latch.save_pending,
            "the lock retry must not flag the width latch"
        );
    }

    #[test]
    fn retry_pending_saves_reaches_both_width_latches() {
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);
        g.width_latch.save_pending = true;
        g.lock_latch.save_pending = true;

        g.retry_pending_saves();

        // Both retries ran; tmux rejected again, so both latches stay.
        assert!(g.width_latch.save_pending);
        assert!(g.lock_latch.save_pending);
    }

    #[test]
    fn the_seed_floor_falls_back_to_the_default_when_the_option_is_unset() {
        // `min_width` is `None` before any tick adopts the option; the
        // seed floor must then be DEFAULT_MIN_WIDTH, exactly like the
        // size guard's unset-option fallback.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane = docked_pane(8, 120);

        state.toggle_width_lock();

        assert_eq!(
            state.global.shared_docked_width,
            Some(DEFAULT_MIN_WIDTH as u16),
            "an unset @sidebar_min_width falls back to the default floor"
        );
    }

    // ─── the strip axis (design D2, D3, D8, D10) ────────────

    fn docked_strip(height: u16, window_height: u16) -> SidebarPaneInfo {
        SidebarPaneInfo {
            pane_active: true,
            window_active: true,
            width: 120,
            window_width: 120,
            window_height,
            floating: false,
            height,
            at_top: false,
        }
    }

    fn strip_state() -> AppState {
        let mut state = AppState::new("%99".into());
        state.layout_kind = crate::state::LayoutKind::Strip;
        state
    }

    #[test]
    fn enabling_from_a_docked_strip_seeds_the_live_height() {
        let mut state = strip_state();
        state.sidebar_pane = docked_strip(12, 40);

        state.toggle_width_lock();

        assert_eq!(state.global.shared_docked_height, Some(12));
        assert!(state.global.width_lock);
        // The rejections prove the enable attempt wrote the seed: the
        // height latch carries the seed write and the lock latch the
        // flag write (D8 order).
        assert!(state.global.height_latch.save_pending);
        assert!(state.global.lock_latch.save_pending);
    }

    #[test]
    fn enabling_from_a_squeezed_strip_seeds_the_floor() {
        // A squeezed strip never propagates its squeeze: the seed is
        // `max(rows, floor)`, so the enabling instance itself gets grown
        // back by the adopt path to the floor it just wrote.
        let mut state = strip_state();
        state.sidebar_pane = docked_strip(2, 40);

        state.toggle_width_lock();

        assert_eq!(
            state.global.shared_docked_height,
            Some(strip_height_floor()),
            "the 4-row minimum strip height floor is the seed"
        );
    }

    #[test]
    fn enabling_from_a_floating_strip_seeds_the_captured_docked_height() {
        // `w` is reachable while floating; the seed must be the docked
        // height captured on the way out, never the live panel height.
        let mut state = strip_state();
        state.sidebar_pane = SidebarPaneInfo {
            floating: true,
            ..docked_strip(24, 40)
        };
        state.docked_size = Some(10);

        state.toggle_width_lock();

        assert_eq!(
            state.global.shared_docked_height,
            Some(10),
            "the captured docked height seeds, not the panel's 24 rows"
        );
    }

    #[test]
    fn enabling_from_a_floating_strip_with_no_capture_enables_unseeded() {
        // A strip has no fixed panel height to fall back to (design
        // D3): a pane floated from the tmux command line has no
        // capture and the enable seeds nothing.
        let mut state = strip_state();
        state.sidebar_pane = SidebarPaneInfo {
            floating: true,
            ..docked_strip(24, 40)
        };

        state.toggle_width_lock();

        assert!(state.global.width_lock, "the lock itself is still enabled");
        assert_eq!(state.global.shared_docked_height, None);
        assert!(state.global.width_latch.save_pending, "the clear still ran");
    }

    #[test]
    fn enabling_from_a_strip_on_a_failed_probe_enables_unseeded() {
        // `window_height == 0` marks the FALLBACK reading (query
        // failed or pane gone): its rows are not a real size and must
        // not become the server's shared height.
        let mut state = strip_state();
        state.sidebar_pane = SidebarPaneInfo::FALLBACK;
        assert_eq!(state.sidebar_pane.window_height, 0);
        let before = state.global.shared_docked_height;

        state.toggle_width_lock();

        assert!(state.global.width_lock, "the lock itself is still enabled");
        assert_eq!(
            state.global.shared_docked_height, before,
            "the height stays unseeded on a failed probe"
        );
        assert!(state.global.lock_latch.save_pending);
        assert!(state.global.width_latch.save_pending, "the clear still ran");
    }

    #[test]
    fn a_rejected_strip_seed_leaves_the_flag_write_running() {
        // In tests tmux rejects every write. An enable must apply all
        // three writes: a height write that fails alone must not stop
        // the clear or the lock flag from going out.
        let mut state = strip_state();
        state.sidebar_pane = docked_strip(12, 40);

        state.toggle_width_lock();

        assert!(state.global.height_latch.save_pending);
        assert!(state.global.width_latch.save_pending, "the clear ran");
        assert!(state.global.lock_latch.save_pending);
        assert_eq!(
            state.global.height_latch.last_saved, None,
            "a rejected seed write must not advance the guard"
        );
    }

    #[test]
    fn a_column_enable_clears_the_shared_docked_height() {
        // A stale `@sidebar_docked_height` from an earlier lock session
        // must not make every strip jump to it when the lock returns
        // from a column (design D10).
        let mut state = AppState::new("%99".into());
        state.global.shared_docked_height = Some(14);
        state.sidebar_pane = docked_pane(45, 120);

        state.toggle_width_lock();

        assert_eq!(
            state.global.shared_docked_height, None,
            "the column enable cleared the other axis"
        );
        assert!(state.global.height_latch.save_pending, "the unset went out");
    }

    #[test]
    fn a_strip_enable_clears_the_shared_docked_width() {
        let mut state = strip_state();
        state.global.shared_docked_width = Some(45);
        state.sidebar_pane = docked_strip(12, 40);

        state.toggle_width_lock();

        assert_eq!(
            state.global.shared_docked_width, None,
            "the strip enable cleared the other axis"
        );
        assert!(state.global.width_latch.save_pending, "the unset went out");
    }

    #[test]
    fn an_unseeded_enable_still_clears_the_other_axis() {
        // The clear is not part of the seed: a failed probe (or a
        // floating strip with no capture) enables unseeded but still
        // clears the other axis, per D10.
        let mut state = strip_state();
        state.sidebar_pane = SidebarPaneInfo::FALLBACK;
        state.global.shared_docked_width = Some(45);

        state.toggle_width_lock();

        assert!(state.global.width_lock);
        assert_eq!(state.global.shared_docked_width, None);
        assert!(state.global.width_latch.save_pending);
    }

    #[test]
    fn a_strip_disable_writes_the_flag_alone() {
        let mut state = strip_state();
        state.global.width_lock = true;
        state.global.shared_docked_height = Some(12);
        state.sidebar_pane = docked_strip(12, 40);

        state.toggle_width_lock();

        assert!(!state.global.width_lock);
        assert_eq!(
            state.global.shared_docked_height,
            Some(12),
            "unlocking leaves the stale shared height behind (reused on re-enable)"
        );
        assert!(
            !state.global.height_latch.save_pending,
            "no height write happens on disable"
        );
    }
}
