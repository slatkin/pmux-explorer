//! The width lock's enable/disable action.
//!
//! One method — [`AppState::toggle_width_lock`] — is the single writer
//! behind the `w` key and the lock glyph's click (both wired in task 4).
//! It flips [`GlobalState::width_lock`] and persists the intent to the
//! shared tmux options: the width seed first, the lock flag second, so
//! a peer adopting `lock=on` mid-tick always finds a width already in
//! place (design D5).

use super::AppState;
use crate::app::size_guard::DEFAULT_MIN_WIDTH;
use crate::state::popout::PANEL_WIDTH;

impl AppState {
    /// Flip the width lock and persist the intent to the shared tmux
    /// options. Enabling seeds the shared docked width with the
    /// floored docked width of this instance — `max(W, floor)` from
    /// this pane's reading, the captured `docked_width` floored while
    /// floating, or `PANEL_WIDTH` when no capture exists (a pane
    /// floated from the tmux command line) — and never the live
    /// 40-column pane width of the floating panel.
    ///
    /// The seed is written only on a valid probe: a `window_width` of 0
    /// means the last reading was the FALLBACK (the query failed or the
    /// pane is gone) and its 28 columns are not a real size, so the
    /// lock is enabled unseeded rather than propagated. Disabling
    /// writes the `off` flag alone; the stale `@sidebar_docked_width`
    /// stays behind harmlessly and is reused when the lock returns.
    ///
    /// Both writes run even if the first fails: a stubbornly failing
    /// writer must not leave every instance locking onto (or seeding
    /// over) a width nobody wrote.
    pub fn toggle_width_lock(&mut self) {
        if !self.global.width_lock {
            // Seed only on a valid probe — see the doc comment.
            if self.sidebar_pane.window_width > 0 {
                // A floor beyond pane geometry cannot be honored
                // anyway; clamping keeps the u32 option value from
                // silently wrapping in the u16 cast.
                let floor_u32 = self
                    .global
                    .min_width
                    .unwrap_or(DEFAULT_MIN_WIDTH)
                    .min(u16::MAX as u32);
                let floor = floor_u32 as u16;
                let live_width = if self.sidebar_pane.floating {
                    self.docked_width.unwrap_or(PANEL_WIDTH)
                } else {
                    self.sidebar_pane.width
                };
                let seed = live_width.max(floor);
                self.global.shared_docked_width = Some(seed);
                self.global.save_shared_docked_width();
            }
            self.global.width_lock = true;
        } else {
            self.global.width_lock = false;
        }
        self.global.save_width_lock();
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
        assert!(state.global.width_save_pending);
        assert!(state.global.lock_save_pending);
        assert!(
            !state.global.last_saved_width_lock,
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
        state.docked_width = Some(55);

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
            state.global.lock_save_pending,
            "the flag write is still attempted"
        );
        assert!(
            !state.global.width_save_pending,
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

        assert!(state.global.width_save_pending);
        assert!(state.global.lock_save_pending);
        assert_eq!(
            state.global.last_saved_shared_docked_width, None,
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
            !state.global.width_save_pending,
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
    fn a_rejected_write_leaves_last_saved_at_the_old_value_width() {
        // Filter-latch shape: only an accepted write may advance the
        // `last_saved_*` guard; the rejected intent stays in the field
        // for the retry latched onto the next ticks.
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);

        g.save_shared_docked_width();

        assert_eq!(
            g.last_saved_shared_docked_width, None,
            "only an accepted write may advance last_saved_shared_docked_width"
        );
        assert!(g.width_save_pending);
        assert_eq!(
            g.shared_docked_width,
            Some(45),
            "a rejected write must not drop the intent"
        );
    }

    #[test]
    fn a_rejected_write_leaves_last_saved_at_the_old_value_lock() {
        let mut g = GlobalState::new();
        g.width_lock = true;

        g.save_width_lock();

        assert!(
            !g.last_saved_width_lock,
            "only an accepted write may advance last_saved_width_lock"
        );
        assert!(g.lock_save_pending);
    }

    #[test]
    fn retry_pending_width_save_keeps_the_latch_while_tmux_rejects() {
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);
        g.width_save_pending = true;

        g.retry_pending_width_save();

        assert!(g.width_save_pending, "a rejected retry keeps the latch set");
        assert_eq!(g.last_saved_shared_docked_width, None);
        assert_eq!(
            g.shared_docked_width,
            Some(45),
            "the retried intent stays in the field"
        );
    }

    #[test]
    fn retry_pending_lock_save_keeps_the_latch_while_tmux_rejects() {
        let mut g = GlobalState::new();
        g.width_lock = true;
        g.lock_save_pending = true;

        g.retry_pending_lock_save();

        assert!(g.lock_save_pending, "a rejected retry keeps the latch set");
        assert!(!g.last_saved_width_lock);
    }

    #[test]
    fn the_two_width_latches_are_independent() {
        let mut g = GlobalState::new();
        g.width_save_pending = true;

        g.retry_pending_lock_save();

        assert!(
            g.width_save_pending,
            "a lock retry must not clear the width latch"
        );

        g.lock_save_pending = true;
        g.width_save_pending = false;
        g.retry_pending_width_save();

        assert!(
            g.lock_save_pending,
            "a width retry must not clear the lock latch"
        );
    }

    #[test]
    fn retry_pending_saves_reaches_both_width_latches() {
        let mut g = GlobalState::new();
        g.shared_docked_width = Some(45);
        g.width_save_pending = true;
        g.lock_save_pending = true;

        g.retry_pending_saves();

        // Both retries ran; tmux rejected again, so both latches stay.
        assert!(g.width_save_pending);
        assert!(g.lock_save_pending);
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
}
