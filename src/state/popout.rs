//! Popping the sidebar out of the layout as a floating panel and putting
//! it back.
//!
//! The mode lives on the pane itself: `#{pane_floating_flag}` as tmux
//! reports it, refreshed on every tick into [`AppState::sidebar_pane`].
//! Nothing about the mode is remembered separately, so a panel floated
//! from the tmux command line reads as floating and can be popped back
//! in. The one thing that IS remembered is the docked width, captured on
//! the way out and restored with `resize-pane -x` on the way in, so a
//! sidebar the user resized by hand keeps its width across the round trip.
//!
//! The tmux calls themselves live in `tmux::float_pane` and
//! `tmux::dock_pane`; this module decides when to run them, with which
//! geometry, and what the outcome means for the state. The outcome
//! handlers ([`AppState::apply_pop_out`] / [`AppState::apply_pop_in`])
//! take the call's `Result` as a parameter so the refusal path is unit
//! testable without a real tmux.

use super::{AppState, LayoutKind};
use crate::cli::toggle::SidebarPosition;
use crate::tmux::{self, SidebarPaneInfo};

/// Width of the floating panel, in columns. Wide enough that the agent
/// list stays readable over the workspace; fixed by the spec, not
/// configurable, and shared with the pop-in's width restore via the
/// captured docked width.
pub const PANEL_WIDTH: u16 = 40;

/// Size and position of the floating panel in window coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelGeometry {
    pub width: u16,
    pub height: u16,
    /// Column of the panel's upper-left corner.
    pub x: u16,
    /// Row of the panel's upper-left corner.
    pub y: u16,
}

/// Compute the panel geometry for a window of the given size. A column
/// floats 40 columns wide, as tall as the window, anchored to the right
/// edge (the original rule). A strip floats as wide as the window, as
/// high as the docked strip was (`docked_size`), anchored to the edge
/// it was docked at. The strip's docked height is used as-is — the
/// floating third-panel limit ([`clamp_bottom_height`]) applies to a
/// column only, because a strip's agent list sits beside the tab panel.
///
/// `float_pane` takes the panel's OUTER size in `-x`/`-y` (the border
/// included) and the upper-left corner in `-X`/`-Y`, so the right-edge
/// anchor is `window_width - PANEL_WIDTH` and the bottom-edge anchor is
/// `window_height - docked_size`. A window narrower than the panel
/// anchors to the left edge instead of underflowing, and a window
/// shorter than the strip to the top edge.
pub fn panel_geometry(
    layout: LayoutKind,
    edge: SidebarPosition,
    window_width: u16,
    window_height: u16,
    docked_size: u16,
) -> PanelGeometry {
    match layout {
        LayoutKind::Column => PanelGeometry {
            width: PANEL_WIDTH,
            height: window_height,
            x: window_width.saturating_sub(PANEL_WIDTH),
            y: 0,
        },
        LayoutKind::Strip => PanelGeometry {
            width: window_width,
            height: docked_size,
            x: 0,
            y: match edge {
                SidebarPosition::Bottom => window_height.saturating_sub(docked_size),
                _ => 0,
            },
        },
    }
}

/// The edge a docked strip sits on, read off the pane probe's
/// `#{pane_at_top}`: a pane touching the top edge is a top strip, any
/// other reading a bottom strip. A column has no strip edge — `None`.
/// The docked size a pop-out captures for the pop-in to restore: the
/// size on the axis the layout docks along — the width of a column,
/// the height of a strip. The other axis's size is the window's, not
/// the sidebar's, and restoring it would size the pane wrong (a
/// 40-column sidebar in a 30-row window would pop back in 30 columns
/// wide).
fn capture_docked_size(layout: LayoutKind, info: &SidebarPaneInfo) -> u16 {
    match layout {
        LayoutKind::Column => info.width,
        LayoutKind::Strip => info.height,
    }
}

fn strip_docked_edge(layout: LayoutKind, at_top: bool) -> Option<SidebarPosition> {
    match layout {
        LayoutKind::Strip => Some(if at_top {
            SidebarPosition::Top
        } else {
            SidebarPosition::Bottom
        }),
        LayoutKind::Column => None,
    }
}

/// Which dock a strip's pop-in runs: the edge join against the
/// workspace pane the window listing named when the pop-out captured a
/// docked edge, or the bare join the column pop-in uses when it did
/// not — no captured edge (the pane floated before the sidebar
/// started) and no listing target (the window has no other pane) both
/// land on the fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StripDockPlan {
    EdgeJoin {
        target: String,
        edge: SidebarPosition,
    },
    Bare,
}

/// The pure decision behind [`Self::pop_in_strip`]'s dock call.
pub(crate) fn strip_dock_plan(
    docked_edge: Option<SidebarPosition>,
    edge_target: Option<String>,
) -> StripDockPlan {
    match (docked_edge, edge_target) {
        (Some(edge), Some(target)) => StripDockPlan::EdgeJoin { target, edge },
        _ => StripDockPlan::Bare,
    }
}

/// Bottom panel height clamped to a third of the floating panel's height,
/// so the agent list keeps the rest of the panel (a 30-row panel gives a
/// bottom panel of at most 10 rows). A panel too small for a third hides
/// the bottom panel entirely. While docked the caller passes the height
/// through unchanged.
pub fn clamp_bottom_height(bottom_height: u16, panel_height: u16) -> u16 {
    bottom_height.min(panel_height / 3)
}

/// The geometry to float the pane with, or `None` when the pane probe
/// reported no usable size. A `0` width or height means the query
/// failed (or the pane is gone); asking tmux to float a pane with a 0
/// height is refused ("position too small"), which would flash a
/// confusing refusal for what is really a failed probe — so an unknown
/// size means "do not act" (with an accurate flash, not silence), the
/// same rule the guard modules apply. A strip whose pane height did
/// not come back (`0`, an older tmux) is refused the same way.
fn float_geometry(
    layout: LayoutKind,
    docked_edge: Option<SidebarPosition>,
    info: SidebarPaneInfo,
) -> Option<PanelGeometry> {
    if info.window_width == 0 || info.window_height == 0 {
        return None;
    }
    match layout {
        LayoutKind::Column => Some(panel_geometry(
            LayoutKind::Column,
            SidebarPosition::Left,
            info.window_width,
            info.window_height,
            info.height,
        )),
        LayoutKind::Strip => {
            let edge = docked_edge?;
            if info.height == 0 {
                return None;
            }
            Some(panel_geometry(
                layout,
                edge,
                info.window_width,
                info.window_height,
                info.height,
            ))
        }
    }
}

impl AppState {
    /// The action behind the pop-out control and tmux prefix + Ctrl-P:
    /// float the pane when it is docked, dock it when it is floating.
    /// The mode is read from the pane here, so the action follows the
    /// pane's actual state even when the pane was floated by hand.
    pub fn toggle_popout(&mut self) {
        let info = tmux::get_sidebar_pane_reading(&self.tmux_pane).info;
        self.toggle_popout_with(info);
    }

    /// The dispatch behind [`Self::toggle_popout`], taking the pane probe
    /// as a parameter so tests can drive both directions — including the
    /// refusal paths — without a live tmux server.
    pub(crate) fn toggle_popout_with(&mut self, info: SidebarPaneInfo) {
        if info.floating {
            self.pop_in_with(info);
        } else {
            self.pop_out_with(info);
        }
    }

    fn pop_out_with(&mut self, info: SidebarPaneInfo) {
        // Precondition: `toggle_popout` only routes a docked pane here.
        debug_assert!(!info.floating, "pop_out_with requires a docked pane");
        let docked_edge = strip_docked_edge(self.layout_kind, info.at_top);
        let Some(geometry) = float_geometry(self.layout_kind, docked_edge, info) else {
            // Unknown window size: the probe failed (pane gone, tmux
            // unreachable) or tmux never reported the size (older than
            // the floating flag). No tmux call — a 0-size float is
            // refused with "position too small" — but the user who
            // used the shortcut must learn why nothing happened.
            self.set_flash(
                "Pop-out unavailable: could not read the window size, so the sidebar stays docked.",
            );
            return;
        };
        let result = tmux::float_pane(
            &self.tmux_pane,
            geometry.width,
            geometry.height,
            geometry.x,
            geometry.y,
        );
        // The probe is mirrored into the state before the capture reads
        // the docked size back from it, so the state's copy is the one
        // the pop-out acts on.
        self.sidebar_pane = info;
        let docked_size = capture_docked_size(self.layout_kind, &info);
        self.apply_pop_out(docked_size, docked_edge, result);
    }

    fn pop_in_with(&mut self, info: SidebarPaneInfo) {
        // Precondition: `toggle_popout` only routes a floating pane here.
        debug_assert!(info.floating, "pop_in_with requires a floating pane");
        match self.layout_kind {
            LayoutKind::Strip => self.pop_in_strip(),
            LayoutKind::Column => self.pop_in_column(),
        }
    }

    /// Dock a floating column back into the tiled layout: the bare join
    /// puts the pane back where its layout had it, and `resize-pane -x`
    /// restores the captured docked width.
    fn pop_in_column(&mut self) {
        if let Some(width) = self.apply_pop_in(tmux::dock_pane(&self.tmux_pane)) {
            let _ = tmux::run_tmux(&[
                "resize-pane",
                "-t",
                &self.tmux_pane,
                "-x",
                &width.to_string(),
            ]);
        }
    }

    /// Dock a floating strip back into the tiled layout. The bare join
    /// has no target edge, so the docked pane is not sure to come back
    /// full width — the strip docks with the edge join of the side
    /// switch against the workspace pane at its captured edge instead,
    /// and `resize-pane -y` restores the captured docked height. With
    /// no edge target (the window has no other pane, or no edge was
    /// captured because the pane floated before the sidebar started)
    /// the bare join is the fallback.
    fn pop_in_strip(&mut self) {
        let edge_target = self
            .docked_edge
            .and_then(|edge| tmux::strip_dock_target(&self.tmux_pane, edge));
        let result = match strip_dock_plan(self.docked_edge, edge_target) {
            StripDockPlan::EdgeJoin { target, edge } => {
                tmux::dock_strip_pane(&self.tmux_pane, &target, edge)
            }
            StripDockPlan::Bare => tmux::dock_pane(&self.tmux_pane),
        };
        if let Some(height) = self.apply_pop_in(result) {
            let _ = tmux::run_tmux(&[
                "resize-pane",
                "-t",
                &self.tmux_pane,
                "-y",
                &height.to_string(),
            ]);
        }
    }

    /// Apply the outcome of the `float_pane` call: on success record the
    /// docked size and edge for the pop-in to restore and mirror the new
    /// mode so the control glyph and the bottom-panel clamp reflect it at
    /// once (the next refresh tick would otherwise leave them stale for
    /// up to a second), on failure flash the refusal and change nothing —
    /// the mode lives on the pane, which stayed docked.
    pub(crate) fn apply_pop_out(
        &mut self,
        docked_size: u16,
        docked_edge: Option<SidebarPosition>,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                self.docked_size = Some(docked_size);
                self.docked_edge = docked_edge;
                self.sidebar_pane.floating = true;
            }
            Err(err) => self.set_flash(format!("Pop-out refused: {err}")),
        }
    }

    /// Apply the outcome of the dock call (bare join for a column, edge
    /// join or bare fallback for a strip). Returns the docked size to
    /// restore with `resize-pane -x` (a column) or `resize-pane -y` (a
    /// strip), or `None` when the dock was refused (flashed) or no size
    /// was captured because the pane was already floating when the
    /// sidebar started. On success the mirrored mode is cleared at once,
    /// so the control glyph and the bottom-panel clamp do not wait for
    /// the next refresh tick.
    pub(crate) fn apply_pop_in(&mut self, result: Result<(), String>) -> Option<u16> {
        match result {
            Ok(()) => {
                let size = self.docked_size.take();
                self.docked_edge = None;
                self.sidebar_pane.floating = false;
                size
            }
            Err(err) => {
                self.set_flash(format!("Pop-in refused: {err}"));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── panel_geometry ──────────────────────────────────────────────

    #[test]
    fn panel_geometry_anchors_a_column_to_the_right_edge() {
        // The column rule is unchanged: 40 columns wide, as tall as the
        // window, anchored to the right edge. The captured docked size
        // (a column's width) plays no part in the float geometry.
        assert_eq!(
            panel_geometry(LayoutKind::Column, SidebarPosition::Left, 120, 40, 25),
            PanelGeometry {
                width: 40,
                height: 40,
                x: 80,
                y: 0,
            }
        );
    }

    #[test]
    fn panel_geometry_anchors_a_column_to_the_left_edge_in_a_narrow_window() {
        let geometry = panel_geometry(LayoutKind::Column, SidebarPosition::Left, 30, 20, 0);
        assert_eq!(
            geometry.x, 0,
            "a window narrower than the panel must not underflow"
        );
        assert_eq!(geometry.width, PANEL_WIDTH);
        assert_eq!(geometry.height, 20);
    }

    #[test]
    fn panel_geometry_floats_a_top_strip_flush_with_the_top_edge() {
        assert_eq!(
            panel_geometry(LayoutKind::Strip, SidebarPosition::Top, 120, 40, 12),
            PanelGeometry {
                width: 120,
                height: 12,
                x: 0,
                y: 0,
            }
        );
    }

    #[test]
    fn panel_geometry_floats_a_bottom_strip_flush_with_the_bottom_edge() {
        assert_eq!(
            panel_geometry(LayoutKind::Strip, SidebarPosition::Bottom, 120, 40, 12),
            PanelGeometry {
                width: 120,
                height: 12,
                x: 0,
                y: 28,
            }
        );
    }

    #[test]
    fn panel_geometry_keeps_a_strips_docked_height_unclamped() {
        // The floating third-panel limit is a column rule: a strip's
        // agent list sits beside the tab panel, so its docked height
        // comes back as it was — 30 rows stay 30 rows, not a third.
        assert_eq!(
            panel_geometry(LayoutKind::Strip, SidebarPosition::Bottom, 120, 40, 30).height,
            30
        );
    }

    // ─── capture_docked_size ─────────────────────────────────────────

    #[test]
    fn capture_docked_size_takes_the_width_of_a_column_and_the_height_of_a_strip() {
        // A 40-column sidebar in a 30-row window docks along the width
        // axis: the pop-in's `resize-pane -x` must get 40 back, not the
        // 30-row pane height the probe also carries (which would pop
        // the column back in 30 columns wide). The strip's docked size
        // is the height — the same probe, the other axis.
        let mut info = SidebarPaneInfo::FALLBACK;
        info.width = 40;
        info.height = 30;
        assert_eq!(capture_docked_size(LayoutKind::Column, &info), 40);
        assert_eq!(capture_docked_size(LayoutKind::Strip, &info), 30);
    }

    // ─── apply_pop_out ───────────────────────────────────────────────

    #[test]
    fn apply_pop_out_success_captures_the_docked_size_and_mirrors_the_mode() {
        let mut state = AppState::new("%99".into());
        state.apply_pop_out(25, None, Ok(()));
        assert_eq!(state.docked_size, Some(25));
        assert!(state.flash.is_none(), "a successful pop-out says nothing");
        // The mirrored mode is updated at once so the control glyph and
        // the bottom-panel clamp reflect the pop-out on the next frame,
        // not only after the next refresh tick.
        assert!(state.sidebar_pane.floating);
    }

    #[test]
    fn apply_pop_out_captures_the_docked_edge_of_a_strip() {
        let mut state = AppState::new("%99".into());
        state.apply_pop_out(12, Some(SidebarPosition::Bottom), Ok(()));
        assert_eq!(state.docked_size, Some(12));
        assert_eq!(state.docked_edge, Some(SidebarPosition::Bottom));
        // A column captures no edge — its pop-in never needs one.
        state.docked_edge = None;
        state.sidebar_pane.floating = false;
        state.apply_pop_out(25, None, Ok(()));
        assert_eq!(state.docked_edge, None);
    }

    #[test]
    fn apply_pop_out_refusal_flashes_and_stays_docked() {
        let mut state = AppState::new("%99".into());
        state.apply_pop_out(
            25,
            Some(SidebarPosition::Top),
            Err("can't float a pane while window is zoomed".into()),
        );
        let flash = state.flash.as_ref().map(|(msg, _)| msg.clone());
        assert_eq!(
            flash.as_deref(),
            Some("Pop-out refused: can't float a pane while window is zoomed"),
        );
        // The mode lives on the pane, not in AppState: a refused pop-out
        // must not record anything that would make the next tick or the
        // pop-in believe the panel is floating.
        assert_eq!(state.docked_size, None);
        assert_eq!(state.docked_edge, None);
        assert!(!state.sidebar_pane.floating);
    }

    // ─── apply_pop_in ────────────────────────────────────────────────

    #[test]
    fn apply_pop_in_returns_the_captured_size_for_the_restore() {
        let mut state = AppState::new("%99".into());
        state.docked_size = Some(25);
        state.sidebar_pane.floating = true;
        assert_eq!(state.apply_pop_in(Ok(())), Some(25));
        assert_eq!(state.docked_size, None, "the capture is spent");
        // The mirrored mode is cleared at once so the control glyph and
        // the bottom-panel clamp go back to docked on the next frame.
        assert!(!state.sidebar_pane.floating);
    }

    #[test]
    fn apply_pop_in_spending_the_size_also_clears_the_edge() {
        let mut state = AppState::new("%99".into());
        state.docked_size = Some(12);
        state.docked_edge = Some(SidebarPosition::Top);
        state.sidebar_pane.floating = true;
        assert_eq!(state.apply_pop_in(Ok(())), Some(12));
        assert_eq!(state.docked_edge, None, "the edge is spent with the size");
    }

    #[test]
    fn apply_pop_in_without_a_capture_restores_nothing() {
        // The pane was already floating when the sidebar started, so no
        // docked size was ever captured: the layout's size is left alone.
        let mut state = AppState::new("%99".into());
        assert_eq!(state.apply_pop_in(Ok(())), None);
        assert!(state.flash.is_none());
    }

    #[test]
    fn apply_pop_in_refusal_flashes_and_restores_nothing() {
        let mut state = AppState::new("%99".into());
        state.docked_size = Some(25);
        state.sidebar_pane.floating = true;
        assert_eq!(state.apply_pop_in(Err("pane not found".into())), None);
        assert!(
            state
                .flash
                .as_ref()
                .is_some_and(|(msg, _)| msg.starts_with("Pop-in refused:"))
        );
        // A refused pop-in must not spend the capture — the size is
        // still there for the next successful pop-in — and the pane
        // stayed floating, so the mirrored mode must not change either.
        assert_eq!(state.docked_size, Some(25));
        assert!(state.sidebar_pane.floating);
    }

    // ─── strip_dock_plan ─────────────────────────────────────────────

    #[test]
    fn strip_dock_plan_takes_the_edge_join_when_an_edge_target_exists() {
        assert_eq!(
            strip_dock_plan(Some(SidebarPosition::Bottom), Some("%3".into())),
            StripDockPlan::EdgeJoin {
                target: "%3".into(),
                edge: SidebarPosition::Bottom,
            }
        );
        assert_eq!(
            strip_dock_plan(Some(SidebarPosition::Top), Some("%1".into())),
            StripDockPlan::EdgeJoin {
                target: "%1".into(),
                edge: SidebarPosition::Top,
            }
        );
    }

    #[test]
    fn strip_dock_plan_falls_back_to_the_bare_join_without_an_edge_target() {
        // No workspace pane named by the listing, no captured edge, or
        // neither — every hole lands on the bare join the column uses.
        assert_eq!(
            strip_dock_plan(Some(SidebarPosition::Top), None),
            StripDockPlan::Bare
        );
        assert_eq!(
            strip_dock_plan(None, Some("%3".into())),
            StripDockPlan::Bare
        );
        assert_eq!(strip_dock_plan(None, None), StripDockPlan::Bare);
    }

    // ─── float_geometry ──────────────────────────────────────────────

    #[test]
    fn float_geometry_is_none_when_the_window_size_is_unknown() {
        // A failed pane probe reports a 0 window size: tmux would refuse
        // the 0-height float request, so the pop-out must not act at all.
        let info = SidebarPaneInfo::FALLBACK;
        assert_eq!(float_geometry(LayoutKind::Column, None, info), None);

        let mut half_probed = info;
        half_probed.window_width = 100;
        assert_eq!(
            float_geometry(LayoutKind::Column, None, half_probed),
            None,
            "a 0 height alone blocks"
        );

        let mut probed = half_probed;
        probed.window_height = 30;
        assert_eq!(
            float_geometry(LayoutKind::Column, None, probed),
            Some(panel_geometry(
                LayoutKind::Column,
                SidebarPosition::Left,
                100,
                30,
                0
            ))
        );
    }

    #[test]
    fn float_geometry_floats_a_strip_at_its_captured_edge() {
        let mut info = SidebarPaneInfo::FALLBACK;
        info.window_width = 100;
        info.window_height = 30;
        info.height = 12;
        info.at_top = false;
        assert_eq!(
            float_geometry(LayoutKind::Strip, Some(SidebarPosition::Bottom), info),
            Some(panel_geometry(
                LayoutKind::Strip,
                SidebarPosition::Bottom,
                100,
                30,
                12
            ))
        );
    }

    #[test]
    fn float_geometry_blocks_a_strip_whose_height_is_unknown() {
        // `#{pane_height}` missing (an older tmux, a failed probe) gives
        // a 0 docked height: tmux would refuse the float with "position
        // too small", so the pop-out must not act at all.
        let mut info = SidebarPaneInfo::FALLBACK;
        info.window_width = 100;
        info.window_height = 30;
        assert_eq!(
            float_geometry(LayoutKind::Strip, Some(SidebarPosition::Bottom), info),
            None
        );
    }

    #[test]
    fn toggle_popout_with_a_docked_pane_tmux_cannot_find_flashes() {
        // A real-shaped docked pane that tmux cannot find: the float call
        // runs with a real geometry and is refused — the flash proves the
        // pop-out action ran, and the pane stays docked.
        let mut state = AppState::new("%4294967295".into());
        state.toggle_popout_with(SidebarPaneInfo {
            pane_active: false,
            window_active: false,
            width: 28,
            window_width: 100,
            window_height: 30,
            floating: false,
            height: 30,
            at_top: false,
        });
        assert!(
            state
                .flash
                .as_ref()
                .is_some_and(|(msg, _)| msg.starts_with("Pop-out refused:")),
            "a refused float must flash"
        );
        assert!(!state.sidebar_pane.floating);
    }

    #[test]
    fn toggle_popout_with_a_docked_strip_floats_at_its_edge_and_refusal_flashes() {
        // The same refused-float probe for a strip: the pop-out action
        // runs the strip geometry (docked height, docked edge), and a
        // tmux refusal leaves the pane docked with nothing captured.
        let mut state = AppState::new("%4294967295".into());
        state.layout_kind = LayoutKind::Strip;
        state.toggle_popout_with(SidebarPaneInfo {
            pane_active: false,
            window_active: false,
            width: 100,
            window_width: 100,
            window_height: 30,
            floating: false,
            height: 12,
            at_top: false,
        });
        assert!(
            state
                .flash
                .as_ref()
                .is_some_and(|(msg, _)| msg.starts_with("Pop-out refused:")),
            "a refused strip float must flash"
        );
        assert!(!state.sidebar_pane.floating);
        assert_eq!(state.docked_size, None);
        assert_eq!(state.docked_edge, None);
    }

    #[test]
    fn toggle_popout_with_an_unknown_window_size_says_so_and_does_not_act() {
        // The probe failed (pane gone, tmux unreachable): no tmux call is
        // made and the pane stays docked, but the user is told why the
        // sidebar did not float — a failed probe is not a refusal, so the
        // flash is an accurate notice, not "Pop-out refused".
        let mut state = AppState::new("%4294967295".into());
        state.toggle_popout_with(SidebarPaneInfo::FALLBACK);
        assert_eq!(
            state.flash.as_ref().map(|(msg, _)| msg.clone()).as_deref(),
            Some(
                "Pop-out unavailable: could not read the window size, so the sidebar stays docked."
            ),
        );
        assert_eq!(state.docked_size, None);
        assert!(!state.sidebar_pane.floating);
    }

    // ─── clamp_bottom_height ─────────────────────────────────────────

    #[test]
    fn clamp_bottom_height_clamps_to_a_third_of_the_panel() {
        // The spec's example: a 30-row floating panel gives a bottom
        // panel of at most 10 rows.
        assert_eq!(clamp_bottom_height(20, 30), 10);
        assert_eq!(clamp_bottom_height(20, 45), 15);
    }

    #[test]
    fn clamp_bottom_height_passes_a_smaller_height_through() {
        assert_eq!(clamp_bottom_height(5, 30), 5);
        assert_eq!(clamp_bottom_height(0, 30), 0);
    }
}
