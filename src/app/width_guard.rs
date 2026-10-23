//! One width-lock step per refresh tick: publish a locally resized docked
//! width to the shared option, or adopt the shared width onto the pane.
//!
//! While the width lock is on, every docked sidebar converges on one width
//! (design D4). tmux never reports who resized a pane, so the step works
//! from what an instance can observe: its own pane width (`W`), the shared
//! docked width (`S`), and the floor. A width that changed since the last
//! recorded observation is a local resize and is PUBLISHED as the new
//! shared width; a pane sitting off the shared width ADOPTS it with
//! `resize-pane -x`, capped at half the window and floored at
//! `@sidebar_min_width`. Both sides act only while the lock is on — while
//! it is off every sidebar keeps its own width, and the stale shared width
//! an unlock leaves behind must not drag panes back (design D5).
//!
//! Unlike [`super::size_guard`], the step is NOT gated on the window being
//! active: the spec requires convergence in hidden windows within one
//! refresh tick. It runs after the active-window guards, so when an
//! adoption and a size-guard grow fire in the same tick, the adopt resize
//! lands last and its target — never below the floor — wins.

use crate::state::AppState;
use crate::state::GlobalState;
use crate::tmux::{self, SidebarPaneInfo};

use super::size_guard::DEFAULT_MIN_WIDTH;

/// Cross-tick state of the width-lock step. Lives on [`AppState`] as the
/// `width_guard` field, beside the other per-feature state, so the pop-in
/// restore (`state/popout.rs`) can arm
/// [`WidthGuardState::pending_adopt_observation`] for the width change its
/// own resize will produce.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WidthGuardState {
    /// The docked width observed on the previous tick the step ran.
    /// `None` until the first docked reading PRIMES it: a `None` tracker
    /// records `W` and can never publish, so a fixed initial value cannot
    /// read the pane's (possibly tmux-clamped) startup width as a change
    /// and demote `S` server-wide.
    pub(crate) last_observed_width: Option<u16>,
    /// One-shot echo suppressor: set whenever this instance issues a
    /// `resize-pane` (the adopt step below; the pop-in restore sets it
    /// too), then consumed by the first observed width change — that
    /// change is the instance's own adoption landing and is never
    /// published — or cleared unconsumed on the next tick the step runs
    /// (the resize was refused). Because it is consumed-or-cleared on
    /// every running tick, a flag armed while unlocked cannot linger and
    /// eat the first resize after a later enable.
    pub(crate) pending_adopt_observation: bool,
}

/// What one width-lock step decided for its pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WidthAction {
    /// Nothing to do this tick.
    Idle,
    /// The pane's width changed locally: `shared_docked_width` already
    /// holds the new width; the caller persists it with
    /// `GlobalState::save_shared_docked_width`.
    Publish,
    /// The pane is off the shared width: resize it to `target`.
    Adopt { target: u16 },
}

/// Decide publish vs adopt for one docked sidebar pane on one tick
/// (design D4). Pure — no tmux calls; [`enforce`] executes the returned
/// action.
///
/// `pane` is the tick's reading of this instance's own sidebar;
/// `global` carries the shared settings adopted at this tick's refresh.
/// The floor is the tick's `global.min_width`, falling back to
/// [`DEFAULT_MIN_WIDTH`] when the option is unset.
pub(crate) fn step(
    tracker: &mut WidthGuardState,
    pane: SidebarPaneInfo,
    global: &mut GlobalState,
) -> WidthAction {
    // An unknown window size means the probe failed (the reading is the
    // FALLBACK), so nothing sizes from it; a floating panel neither
    // publishes nor adopts, so the fixed 40-column panel can never feed
    // itself into the shared width. Both leave the tracker and the echo
    // suppressor untouched — the step did not run.
    if pane.window_width == 0 || pane.floating {
        return WidthAction::Idle;
    }

    // Only a width that changed since a recorded observation counts as a
    // change. The tracker updates on every docked tick, locked or not —
    // one that slept while unlocked would read a stale width as a fresh
    // change on the tick the lock arrives.
    let observed_change = match tracker.last_observed_width {
        None => false,
        Some(last) => last != pane.width,
    };
    tracker.last_observed_width = Some(pane.width);

    // The echo suppressor is evaluated on every tick the step runs,
    // locked or not: a flag armed while unlocked (a pop-in) is consumed
    // by its own landing or dropped, never carried into a later enable.
    let mut suppress_publish = false;
    if tracker.pending_adopt_observation {
        tracker.pending_adopt_observation = false;
        suppress_publish = observed_change;
    }

    let locked = global.width_lock;
    // A floor beyond pane geometry cannot be honored anyway; the same
    // clamp the enabling seed uses keeps the u32 option value from
    // silently wrapping in the u16 math below.
    let floor = global
        .min_width
        .unwrap_or(DEFAULT_MIN_WIDTH)
        .min(u16::MAX as u32) as u16;
    let cap = floor.max(pane.window_width / 2);

    // Publish: something resized the pane to a usable width. The field is
    // assigned BEFORE the adopt check below reads it — ordering is
    // load-bearing: a just-published width makes the adopt check see the
    // fresh `S`, whose `W != S` term then blocks any cap, so the pane the
    // user resized is never grown or shrunk back in the same tick.
    let mut action = WidthAction::Idle;
    if locked && observed_change && !suppress_publish && pane.width >= floor {
        global.shared_docked_width = Some(pane.width);
        action = WidthAction::Publish;
    }

    // Adopt: the pane sits off the shared width. `W != S` exempts the
    // publisher's own pane from the cap — a deliberate drag past half the
    // window publishes and stays where it happened, while adopting peers
    // clamp; the term goes dead as soon as `S` moves again. The latch
    // check stops the adopt side from reverting a pane whose publish
    // write failed: tmux still holds the stale shared value, and adopting
    // it would resize the pane over the user's hand.
    if action == WidthAction::Idle
        && locked
        && let Some(shared) = global.shared_docked_width
        && !global.width_save_pending
    {
        let target = shared.max(floor).min(cap);
        if pane.width != target && pane.width != shared {
            tracker.pending_adopt_observation = true;
            return WidthAction::Adopt { target };
        }
    }
    action
}

/// Run one width-lock step for this instance's own sidebar and execute the
/// decision. Runs every refresh tick for every docked pane — NOT gated on
/// the window being active, so hidden windows converge before the user
/// switches to them. A floating panel and an unknown window size are
/// no-ops ([`step`] gates them).
pub(crate) fn enforce(state: &mut AppState) {
    let pane = state.sidebar_pane;
    match step(&mut state.width_guard, pane, &mut state.global) {
        WidthAction::Idle => {}
        WidthAction::Publish => state.global.save_shared_docked_width(),
        WidthAction::Adopt { target } => {
            let _ = tmux::run_tmux(&[
                "resize-pane",
                "-t",
                &state.tmux_pane,
                "-x",
                &target.to_string(),
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A docked pane reading of the given pane/window width.
    fn pane(width: u16, window_width: u16) -> SidebarPaneInfo {
        SidebarPaneInfo {
            pane_active: false,
            window_active: false,
            width,
            window_width,
            window_height: 30,
            floating: false,
        }
    }

    /// A tracker that has already observed `width` on a previous tick,
    /// so a different width on the next call counts as a change.
    fn primed(width: u16) -> WidthGuardState {
        WidthGuardState {
            last_observed_width: Some(width),
            pending_adopt_observation: false,
        }
    }

    /// A locked `GlobalState` with the given shared width, floor and
    /// width-latch state — the inputs the step reads.
    fn locked_global(shared: Option<u16>, min_width: Option<u32>, pending: bool) -> GlobalState {
        let mut g = GlobalState::new();
        g.width_lock = true;
        g.shared_docked_width = shared;
        g.min_width = min_width;
        g.width_save_pending = pending;
        g
    }

    #[test]
    fn a_cold_start_primes_without_publishing() {
        // First docked reading ever: the tracker is primed and can never
        // publish, so a tmux-clamped creation width cannot demote the
        // server's shared width on the first locked tick.
        let mut tracker = WidthGuardState::default();
        let mut global = locked_global(None, None, false);

        let action = step(&mut tracker, pane(80, 200), &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(
            global.shared_docked_width, None,
            "a priming tick never publishes"
        );
        assert_eq!(tracker.last_observed_width, Some(80));
    }

    #[test]
    fn a_cold_start_with_a_shared_width_adopts_instead_of_publishing() {
        // A fresh instance on an already-locked server starts at its
        // creation width and must converge on `S`, not publish 28 over it.
        let mut tracker = WidthGuardState::default();
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(28, 200), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 45 });
        assert_eq!(global.shared_docked_width, Some(45));
    }

    #[test]
    fn a_manual_resize_publishes() {
        // The pane was at the shared width and the user dragged it: the
        // new width becomes the shared one.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(60, 200), &mut global);

        assert_eq!(action, WidthAction::Publish);
        assert_eq!(global.shared_docked_width, Some(60));
    }

    #[test]
    fn a_squeeze_below_the_floor_does_not_publish() {
        // A layout squeeze reads as a squeeze, not a choice: a width below
        // the floor is never published (size_guard grows the pane back).
        let mut tracker = primed(45);
        let mut global = locked_global(None, None, false);

        let action = step(&mut tracker, pane(10, 200), &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(
            global.shared_docked_width, None,
            "a squeezed width must not become the shared width"
        );
    }

    #[test]
    fn an_exact_adoption_landing_does_not_echo() {
        // This instance issued a resize to the target last tick; the
        // landing is observed as a width change and must be consumed, not
        // published back over the shared width.
        let mut tracker = WidthGuardState {
            last_observed_width: Some(80),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(50), None, false);

        let action = step(&mut tracker, pane(50, 200), &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(
            global.shared_docked_width,
            Some(50),
            "the landing must not be published back"
        );
        assert!(
            !tracker.pending_adopt_observation,
            "the flag is consumed by the landing"
        );
    }

    #[test]
    fn a_short_adoption_landing_does_not_echo_but_is_retried() {
        // Shared 80 in a 100-column window: the issued resize to the cap
        // of 50 landed short at 48. The landing is suppressed as a publish,
        // the shared value stays 80, and the step retries the resize.
        let mut tracker = WidthGuardState {
            last_observed_width: Some(80),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(80), None, false);

        let action = step(&mut tracker, pane(48, 100), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 50 });
        assert_eq!(
            global.shared_docked_width,
            Some(80),
            "a short landing must never demote the shared width"
        );
        assert!(
            tracker.pending_adopt_observation,
            "the retry re-arms the suppressor"
        );
    }

    #[test]
    fn a_refused_resize_frees_the_suppression_for_the_next_manual_resize() {
        let mut tracker = WidthGuardState {
            last_observed_width: Some(45),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(45), None, false);

        // Tick 1: the pane never moved — the resize was refused, the flag
        // clears unconsumed.
        assert_eq!(
            step(&mut tracker, pane(45, 200), &mut global),
            WidthAction::Idle
        );
        assert!(!tracker.pending_adopt_observation);

        // Tick 2: the user's next resize publishes normally.
        assert_eq!(
            step(&mut tracker, pane(60, 200), &mut global),
            WidthAction::Publish
        );
        assert_eq!(global.shared_docked_width, Some(60));
    }

    #[test]
    fn a_100_column_window_caps_a_shared_80_at_50_without_clobbering_the_shared_value() {
        // An adopting peer in a 100-column window takes the cap (half the
        // window), and the shared value itself stays 80 for wide windows.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(80), None, false);

        let action = step(&mut tracker, pane(45, 100), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 50 });
        assert_eq!(
            global.shared_docked_width,
            Some(80),
            "the cap clamps the pane, never the shared value"
        );
    }

    #[test]
    fn a_shared_8_under_a_floor_of_20_adopts_20() {
        // The floor binds both ways: a shared width below the floor (a
        // seed written while the enabling pane was squeezed) is adopted at
        // the floor, not at `S`.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(8), None, false);

        let action = step(&mut tracker, pane(45, 200), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 20 });
    }

    #[test]
    fn an_explicit_floor_option_lifts_a_low_shared_width() {
        // The floor rides the tick's options map (`global.min_width`), not
        // a fresh tmux read.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(10), Some(24), false);

        let action = step(&mut tracker, pane(45, 200), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 24 });
    }

    #[test]
    fn the_publisher_s_own_resize_is_not_clamped_by_its_own_adopt() {
        // A deliberate drag to 70 in a 100-column window (cap 50)
        // publishes and stays: the adopt check reads the freshly assigned
        // `S` and its `W != S` term blocks the cap. The ordering between
        // the publish assignment and the adopt check is load-bearing.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(70, 100), &mut global);

        assert_eq!(action, WidthAction::Publish);
        assert_eq!(global.shared_docked_width, Some(70));
    }

    #[test]
    fn a_narrower_shared_width_shrinks_the_pane() {
        // A peer published a smaller width; this pane adopts it.
        let mut tracker = primed(80);
        let mut global = locked_global(Some(30), None, false);

        let action = step(&mut tracker, pane(80, 100), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 30 });
    }

    #[test]
    fn a_pending_publish_blocks_the_adopt_side() {
        // The publish write failed, so tmux still holds the stale shared
        // value this instance is replacing; adopting it would resize the
        // pane over the user's hand (spec: "a failed width publish is not
        // reverted").
        let mut tracker = primed(80);
        let mut global = locked_global(Some(45), None, true);

        let action = step(&mut tracker, pane(80, 200), &mut global);

        assert_eq!(action, WidthAction::Idle);
    }

    #[test]
    fn an_unlocked_resize_neither_publishes_nor_adopts() {
        // While the lock is off every sidebar keeps its own width, and the
        // stale shared width an unlock leaves behind must not drag panes.
        let mut tracker = primed(45);
        let mut global = GlobalState::new();
        global.shared_docked_width = Some(45);

        let action = step(&mut tracker, pane(80, 200), &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(
            global.shared_docked_width,
            Some(45),
            "an unlocked resize must not move the shared width"
        );
    }

    #[test]
    fn the_tracker_stays_current_while_unlocked() {
        // A tracker that slept while unlocked would read the width it
        // missed as a fresh change on the tick the lock arrives and
        // publish it over the shared value; staying current makes that
        // tick an adoption instead.
        let mut tracker = WidthGuardState::default();
        let mut global = GlobalState::new();
        step(&mut tracker, pane(80, 200), &mut global);

        // The lock arrives (adopted by `apply_all` at the refresh) with a
        // shared width already set.
        global.width_lock = true;
        global.shared_docked_width = Some(45);

        let action = step(&mut tracker, pane(80, 200), &mut global);

        assert_eq!(action, WidthAction::Adopt { target: 45 });
        assert_eq!(global.shared_docked_width, Some(45));
    }

    #[test]
    fn a_floating_pane_is_skipped_entirely() {
        // The fixed 40-column floating panel must never feed itself into
        // the shared width, and a floating tick moves neither the tracker
        // nor the suppressor.
        let mut tracker = primed(45);
        tracker.pending_adopt_observation = true;
        let mut global = locked_global(Some(45), None, false);

        let mut floating = pane(40, 200);
        floating.floating = true;
        let action = step(&mut tracker, floating, &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_width, Some(45));
        assert!(tracker.pending_adopt_observation);
    }

    #[test]
    fn an_unknown_window_size_is_a_no_op() {
        // `window_width == 0` means the probe failed; the step does not
        // act and does not touch the tracker or the suppressor.
        let mut tracker = primed(45);
        tracker.pending_adopt_observation = true;
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(0, 0), &mut global);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_width, Some(45));
        assert!(tracker.pending_adopt_observation);
    }
}
