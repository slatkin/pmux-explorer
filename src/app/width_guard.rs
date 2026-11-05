//! One sidebar-size-lock step per refresh tick: publish a locally
//! resized docked size to the shared option, or adopt the shared size
//! onto the pane — on the axis the instance's fixed layout gave it
//! (design D5 of the strip-height-sync change).
//!
//! While the lock is on, every docked sidebar converges on one docked
//! size on its own axis: a column on the shared width, a strip on the
//! shared height. tmux never reports who resized a pane, so the step
//! works from what an instance can observe: its own docked size (`W`,
//! the pane width for a column, the pane height for a strip), the
//! shared docked size on that axis (`S`), and the floor. A size that
//! changed since the last recorded observation is a local resize and
//! is PUBLISHED as the new shared size; a pane sitting off the shared
//! size ADOPTS it with `resize-pane -x` (a column) or `-y` (a strip),
//! capped at half the window's matching dimension and floored at
//! `@sidebar_min_width` (a column) or the 4-row strip constant (a
//! strip). Both sides act only while the lock is on — while it is off
//! every sidebar keeps its own size, and the stale shared size an
//! unlock leaves behind must not drag panes back (design D5).
//!
//! Unlike [`super::size_guard`], the step is NOT gated on the window being
//! active: the spec requires convergence in hidden windows within one
//! refresh tick. It runs after the active-window guards, so when an
//! adoption and a size-guard grow fire in the same tick, the adopt resize
//! lands last and its target — never below the floor — wins.

use crate::state::{
    AppState, GlobalState, LayoutKind, adopt_target, resize_pane_args, strip_height_floor,
    width_floor,
};
use crate::tmux::{self, SidebarPaneInfo};

/// Cross-tick state of the size-lock step. Lives on [`AppState`] as the
/// `width_guard` field, beside the other per-feature state, so the pop-in
/// restore (`state/popout.rs`) can arm
/// [`WidthGuardState::pending_adopt_observation`] for the size change its
/// own resize will produce.
///
/// One tracker per instance holds for both axes: the layout is fixed for
/// the life of the pane, so one echo suppressor per instance stays
/// correct no matter which axis the instance serves.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WidthGuardState {
    /// The docked size observed on the previous tick the step ran, on
    /// the axis this instance's layout serves. `None` until the first
    /// docked reading PRIMES it: a `None` tracker records the size and
    /// can never publish, so a fixed initial value cannot read the
    /// pane's (possibly tmux-clamped) startup size as a change and
    /// demote `S` server-wide.
    pub(crate) last_observed_size: Option<u16>,
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

/// What one size-lock step decided for its pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WidthAction {
    /// Nothing to do this tick.
    Idle,
    /// The pane's docked size changed locally: the shared option on the
    /// step's axis already holds the new size; the caller persists it
    /// with `GlobalState::save_shared_docked_width` (a column) or
    /// `GlobalState::save_shared_docked_height` (a strip).
    Publish,
    /// The pane is off the shared size on the step's axis: resize it to
    /// `target`.
    Adopt { target: u16 },
}

/// Decide publish vs adopt for one docked sidebar pane on one tick,
/// on the axis `axis` names (design D4, D5). Pure — no tmux calls;
/// [`enforce`] executes the returned action.
///
/// `pane` is the tick's reading of this instance's own sidebar;
/// `global` carries the shared settings adopted at this tick's
/// refresh. The floor is the axis's rule: the tick's
/// `global.min_width`, floored by [`width_floor`], for a column, and
/// the constant [`strip_height_floor`] for a strip — the same helpers
/// the enabling seeds and the pop-in restore clamp use.
pub(crate) fn step(
    tracker: &mut WidthGuardState,
    pane: SidebarPaneInfo,
    global: &mut GlobalState,
    axis: LayoutKind,
) -> WidthAction {
    // The docked size and the matching window dimension, read on the
    // step's axis.
    let (size, window_size) = match axis {
        LayoutKind::Column => (pane.width, pane.window_width),
        LayoutKind::Strip => (pane.height, pane.window_height),
    };
    // An unknown window size means the probe failed (the reading is the
    // FALLBACK), so nothing sizes from it; a floating panel neither
    // publishes nor adopts, so the fixed-size panel can never feed
    // itself into the shared size. Both leave the tracker and the echo
    // suppressor untouched — the step did not run.
    if window_size == 0 || pane.floating {
        return WidthAction::Idle;
    }

    // Only a size that changed since a recorded observation counts as a
    // change. The tracker updates on every docked tick, locked or not —
    // one that slept while unlocked would read a stale size as a fresh
    // change on the tick the lock arrives.
    let observed_change = match tracker.last_observed_size {
        None => false,
        Some(last) => last != size,
    };
    tracker.last_observed_size = Some(size);

    // The echo suppressor is evaluated on every tick the step runs,
    // locked or not: a flag armed while unlocked (a pop-in) is consumed
    // by its own landing or dropped, never carried into a later enable.
    let mut suppress_publish = false;
    if tracker.pending_adopt_observation {
        tracker.pending_adopt_observation = false;
        suppress_publish = observed_change;
    }

    let locked = global.width_lock;
    // The floored-and-clamped floor the lock sites share, in
    // `state/width.rs` — the width option's floor for a column, the
    // constant for a strip.
    let floor = match axis {
        LayoutKind::Column => width_floor(global.min_width),
        LayoutKind::Strip => strip_height_floor(),
    };
    let cap = floor.max(window_size / 2);

    // Publish: something resized the pane to a usable size. The field
    // is assigned BEFORE the adopt check below reads it — ordering is
    // load-bearing: a just-published size makes the adopt check see the
    // fresh `S`, whose `W != S` term then blocks any cap, so the pane the
    // user resized is never grown or shrunk back in the same tick.
    let mut action = WidthAction::Idle;
    if locked && observed_change && !suppress_publish && size >= floor {
        match axis {
            LayoutKind::Column => global.shared_docked_width = Some(size),
            LayoutKind::Strip => global.shared_docked_height = Some(size),
        }
        action = WidthAction::Publish;
    }

    // Adopt: the pane sits off the shared size. `W != S` exempts the
    // publisher's own pane from the cap — a deliberate drag past half the
    // window publishes and stays where it happened, while adopting peers
    // clamp; the term goes dead as soon as `S` moves again. The latch
    // check stops the adopt side from reverting a pane whose publish
    // write failed: tmux still holds the stale shared value, and adopting
    // it would resize the pane over the user's hand.
    if action == WidthAction::Idle && locked {
        let shared = match axis {
            LayoutKind::Column => global.shared_docked_width,
            LayoutKind::Strip => global.shared_docked_height,
        };
        let save_pending = match axis {
            LayoutKind::Column => global.width_latch.save_pending,
            LayoutKind::Strip => global.height_latch.save_pending,
        };
        if let Some(shared) = shared
            && !save_pending
        {
            let target = adopt_target(shared, floor, cap);
            if size != target && size != shared {
                tracker.pending_adopt_observation = true;
                return WidthAction::Adopt { target };
            }
        }
    }
    action
}

/// Run one size-lock step for this instance's own sidebar and execute
/// the decision, on the axis the instance's fixed layout gave it
/// (design D5). Runs every refresh tick for every docked pane — NOT
/// gated on the window being active, so hidden windows converge
/// before the user switches to them. A floating panel and an unknown
/// window size are no-ops ([`step`] gates them).
pub(crate) fn enforce(state: &mut AppState) {
    let axis = state.layout_kind;
    let pane = state.sidebar_pane;
    match step(&mut state.width_guard, pane, &mut state.global, axis) {
        WidthAction::Idle => {}
        WidthAction::Publish => match axis {
            LayoutKind::Column => state.global.save_shared_docked_width(),
            LayoutKind::Strip => state.global.save_shared_docked_height(),
        },
        WidthAction::Adopt { target } => {
            let args = resize_pane_args(&state.tmux_pane, axis, target);
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let _ = tmux::run_tmux(&arg_refs);
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
            height: 30,
            at_top: false,
        }
    }

    /// A tracker that has already observed `size` on a previous tick,
    /// so a different size on the next call counts as a change.
    fn primed(size: u16) -> WidthGuardState {
        WidthGuardState {
            last_observed_size: Some(size),
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
        g.width_latch.save_pending = pending;
        g
    }

    #[test]
    fn a_cold_start_primes_without_publishing() {
        // First docked reading ever: the tracker is primed and can never
        // publish, so a tmux-clamped creation width cannot demote the
        // server's shared width on the first locked tick.
        let mut tracker = WidthGuardState::default();
        let mut global = locked_global(None, None, false);

        let action = step(&mut tracker, pane(80, 200), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(
            global.shared_docked_width, None,
            "a priming tick never publishes"
        );
        assert_eq!(tracker.last_observed_size, Some(80));
    }

    #[test]
    fn a_cold_start_with_a_shared_width_adopts_instead_of_publishing() {
        // A fresh instance on an already-locked server starts at its
        // creation width and must converge on `S`, not publish 28 over it.
        let mut tracker = WidthGuardState::default();
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(28, 200), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Adopt { target: 45 });
        assert_eq!(global.shared_docked_width, Some(45));
    }

    #[test]
    fn a_manual_resize_publishes() {
        // The pane was at the shared width and the user dragged it: the
        // new width becomes the shared one.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(60, 200), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Publish);
        assert_eq!(global.shared_docked_width, Some(60));
    }

    #[test]
    fn a_squeeze_below_the_floor_does_not_publish() {
        // A layout squeeze reads as a squeeze, not a choice: a width below
        // the floor is never published (size_guard grows the pane back).
        let mut tracker = primed(45);
        let mut global = locked_global(None, None, false);

        let action = step(&mut tracker, pane(10, 200), &mut global, LayoutKind::Column);

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
            last_observed_size: Some(80),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(50), None, false);

        let action = step(&mut tracker, pane(50, 200), &mut global, LayoutKind::Column);

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
            last_observed_size: Some(80),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(80), None, false);

        let action = step(&mut tracker, pane(48, 100), &mut global, LayoutKind::Column);

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
            last_observed_size: Some(45),
            pending_adopt_observation: true,
        };
        let mut global = locked_global(Some(45), None, false);

        // Tick 1: the pane never moved — the resize was refused, the flag
        // clears unconsumed.
        assert_eq!(
            step(&mut tracker, pane(45, 200), &mut global, LayoutKind::Column),
            WidthAction::Idle
        );
        assert!(!tracker.pending_adopt_observation);

        // Tick 2: the user's next resize publishes normally.
        assert_eq!(
            step(&mut tracker, pane(60, 200), &mut global, LayoutKind::Column),
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

        let action = step(&mut tracker, pane(45, 100), &mut global, LayoutKind::Column);

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
        let mut global = locked_global(Some(8), Some(20), false);

        let action = step(&mut tracker, pane(45, 200), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Adopt { target: 20 });
    }

    #[test]
    fn an_explicit_floor_option_lifts_a_low_shared_width() {
        // The floor rides the tick's options map (`global.min_width`), not
        // a fresh tmux read.
        let mut tracker = primed(45);
        let mut global = locked_global(Some(10), Some(24), false);

        let action = step(&mut tracker, pane(45, 200), &mut global, LayoutKind::Column);

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

        let action = step(&mut tracker, pane(70, 100), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Publish);
        assert_eq!(global.shared_docked_width, Some(70));
    }

    #[test]
    fn a_narrower_shared_width_shrinks_the_pane() {
        // A peer published a smaller width; this pane adopts it.
        let mut tracker = primed(80);
        let mut global = locked_global(Some(30), Some(20), false);

        let action = step(&mut tracker, pane(80, 100), &mut global, LayoutKind::Column);

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

        let action = step(&mut tracker, pane(80, 200), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Idle);
    }

    #[test]
    fn an_unlocked_resize_neither_publishes_nor_adopts() {
        // While the lock is off every sidebar keeps its own width, and the
        // stale shared width an unlock leaves behind must not drag panes.
        let mut tracker = primed(45);
        let mut global = GlobalState::new();
        global.shared_docked_width = Some(45);

        let action = step(&mut tracker, pane(80, 200), &mut global, LayoutKind::Column);

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
        step(&mut tracker, pane(80, 200), &mut global, LayoutKind::Column);

        // The lock arrives (adopted by `apply_all` at the refresh) with a
        // shared width already set.
        global.width_lock = true;
        global.shared_docked_width = Some(45);

        let action = step(&mut tracker, pane(80, 200), &mut global, LayoutKind::Column);

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
        let action = step(&mut tracker, floating, &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_size, Some(45));
        assert!(tracker.pending_adopt_observation);
    }

    #[test]
    fn an_unknown_window_size_is_a_no_op() {
        // `window_width == 0` means the probe failed; the step does not
        // act and does not touch the tracker or the suppressor.
        let mut tracker = primed(45);
        tracker.pending_adopt_observation = true;
        let mut global = locked_global(Some(45), None, false);

        let action = step(&mut tracker, pane(0, 0), &mut global, LayoutKind::Column);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_size, Some(45));
        assert!(tracker.pending_adopt_observation);
    }

    // ─── the strip axis (design D5) ────────────
    //
    // Only the axis selection is mirrored here: the strip step reads
    // `height`/`window_height`, floors at the 4-row strip constant, caps
    // at half the window's rows, and publishes through the height field.
    // The axis-independent logic — echo suppression, the
    // publish-before-adopt ordering, the latch block, the tracker prime
    // — stays covered by the column suite above and is not repeated.

    /// A docked strip reading of the given pane/window height.
    fn strip(height: u16, window_height: u16) -> SidebarPaneInfo {
        SidebarPaneInfo {
            pane_active: false,
            window_active: false,
            width: 120,
            window_width: 120,
            window_height,
            floating: false,
            height,
            at_top: false,
        }
    }

    /// A locked `GlobalState` with the given shared height and
    /// height-latch state — the strip inputs the step reads.
    fn locked_strip_global(shared: Option<u16>, pending: bool) -> GlobalState {
        let mut g = GlobalState::new();
        g.width_lock = true;
        g.shared_docked_height = shared;
        g.height_latch.save_pending = pending;
        g
    }

    #[test]
    fn a_strip_step_reads_the_height_axis() {
        // The axis selection: a changed pane height publishes the shared
        // docked HEIGHT, never the width field a column's step writes.
        let mut tracker = primed(12);
        let mut global = locked_strip_global(None, false);

        let action = step(&mut tracker, strip(14, 40), &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Publish);
        assert_eq!(global.shared_docked_height, Some(14));
        assert_eq!(
            global.shared_docked_width, None,
            "the strip step never touches the width axis"
        );
    }

    #[test]
    fn a_strip_squeeze_below_the_floor_does_not_publish() {
        // The strip's floor is the 4-row constant, not an option: a
        // horizontal split that squeezes the strip below it is never
        // published as a choice.
        let mut tracker = primed(12);
        let mut global = locked_strip_global(None, false);

        let action = step(&mut tracker, strip(2, 40), &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(global.shared_docked_height, None);
    }

    #[test]
    fn a_strip_adopts_at_half_the_window_s_rows() {
        // The cap runs in rows: a shared 16 in a 30-row window adopts 15,
        // and the shared value itself stays 16 for taller windows.
        let mut tracker = primed(10);
        let mut global = locked_strip_global(Some(16), false);

        let action = step(&mut tracker, strip(10, 30), &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Adopt { target: 15 });
        assert_eq!(
            global.shared_docked_height,
            Some(16),
            "the cap clamps the pane, never the shared height"
        );
    }

    #[test]
    fn a_shared_height_below_the_strip_floor_adopts_at_the_floor() {
        // The 4-row floor binds adoptions both ways, and it wins over the
        // cap where the two cannot both hold — a 5-row window caps at 2,
        // but the floor still gives 4.
        let mut tracker = primed(10);
        let mut global = locked_strip_global(Some(2), false);

        let action = step(&mut tracker, strip(10, 40), &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Adopt { target: 4 });
    }

    #[test]
    fn a_strip_height_publish_lands_in_the_height_latch_end_to_end() {
        // Through `enforce`: the strip's publish goes to
        // `save_shared_docked_height` — the height latch carries the
        // write (rejected in tests, so it stays pending) and the width
        // latch is untouched.
        let mut state = AppState::new("%99".into());
        state.layout_kind = LayoutKind::Strip;
        state.global.width_lock = true;
        state.width_guard.last_observed_size = Some(12);
        state.sidebar_pane = strip(14, 40);

        enforce(&mut state);

        assert_eq!(state.global.shared_docked_height, Some(14));
        assert!(state.global.height_latch.save_pending);
        assert!(!state.global.width_latch.save_pending);
    }

    #[test]
    fn a_strip_adopt_resizes_through_enforce_end_to_end() {
        // Through `enforce`: the strip's adopt arms the echo suppressor
        // for the landing, and the resize argv itself is pinned by the
        // shared `resize_pane_args` test in `state/width.rs`.
        let mut state = AppState::new("%99".into());
        state.layout_kind = LayoutKind::Strip;
        state.global.width_lock = true;
        state.global.shared_docked_height = Some(14);
        state.width_guard.last_observed_size = Some(10);
        state.sidebar_pane = strip(10, 40);

        enforce(&mut state);

        assert!(state.width_guard.pending_adopt_observation);
    }

    #[test]
    fn a_floating_strip_is_skipped_entirely() {
        // The floating panel must never feed its height into the shared
        // value, and a floating tick moves neither the tracker nor the
        // suppressor.
        let mut tracker = primed(12);
        tracker.pending_adopt_observation = true;
        let mut global = locked_strip_global(Some(12), false);

        let mut floating = strip(24, 40);
        floating.floating = true;
        let action = step(&mut tracker, floating, &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_size, Some(12));
        assert!(tracker.pending_adopt_observation);
    }

    #[test]
    fn an_unknown_window_height_is_a_no_op() {
        // `window_height == 0` means the probe failed; the step does not
        // act and does not touch the tracker or the suppressor.
        let mut tracker = primed(12);
        tracker.pending_adopt_observation = true;
        let mut global = locked_strip_global(Some(12), false);

        let action = step(&mut tracker, strip(0, 0), &mut global, LayoutKind::Strip);

        assert_eq!(action, WidthAction::Idle);
        assert_eq!(tracker.last_observed_size, Some(12));
        assert!(tracker.pending_adopt_observation);
    }
}
