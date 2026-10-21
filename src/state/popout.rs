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

use super::AppState;
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

/// Compute the panel geometry for a window of the given size: 40 columns
/// wide, as tall as the window, anchored to the right edge.
///
/// `float_pane` takes the panel's OUTER size in `-x`/`-y` (the border
/// included) and the upper-left corner in `-X`/`-Y`, so the right-edge
/// anchor is `window_width - PANEL_WIDTH`. A window narrower than the
/// panel anchors to the left edge instead of underflowing.
pub fn panel_geometry(window_width: u16, window_height: u16) -> PanelGeometry {
    PanelGeometry {
        width: PANEL_WIDTH,
        height: window_height,
        x: window_width.saturating_sub(PANEL_WIDTH),
        y: 0,
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

impl AppState {
    /// The single action behind the pop-out control and the `p` key:
    /// float the pane when it is docked, dock it when it is floating.
    /// The mode is read from the pane here, so the action follows the
    /// pane's actual state even when the pane was floated by hand.
    pub fn toggle_popout(&mut self) {
        let info = tmux::get_sidebar_pane_info(&self.tmux_pane);
        if info.floating {
            self.pop_in_with(info);
        } else {
            self.pop_out_with(info);
        }
    }

    fn pop_out_with(&mut self, info: SidebarPaneInfo) {
        if info.floating {
            return;
        }
        let geometry = panel_geometry(info.window_width, info.window_height);
        let result = tmux::float_pane(
            &self.tmux_pane,
            geometry.width,
            geometry.height,
            geometry.x,
            geometry.y,
        );
        self.apply_pop_out(info.width, result);
    }

    fn pop_in_with(&mut self, info: SidebarPaneInfo) {
        if !info.floating {
            return;
        }
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

    /// Apply the outcome of the `float_pane` call: on success record the
    /// docked width for the pop-in to restore, on failure flash the
    /// refusal and change nothing — the mode lives on the pane, which
    /// stayed docked.
    pub(crate) fn apply_pop_out(&mut self, docked_width: u16, result: Result<(), String>) {
        match result {
            Ok(()) => self.docked_width = Some(docked_width),
            Err(err) => self.set_flash(format!("Pop-out refused: {err}")),
        }
    }

    /// Apply the outcome of the `dock_pane` call. Returns the docked
    /// width to restore with `resize-pane -x`, or `None` when the dock
    /// was refused (flashed) or no width was captured because the pane
    /// was already floating when the sidebar started.
    pub(crate) fn apply_pop_in(&mut self, result: Result<(), String>) -> Option<u16> {
        match result {
            Ok(()) => self.docked_width.take(),
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
    fn panel_geometry_anchors_the_panel_to_the_right_edge() {
        assert_eq!(
            panel_geometry(120, 40),
            PanelGeometry {
                width: 40,
                height: 40,
                x: 80,
                y: 0,
            }
        );
    }

    #[test]
    fn panel_geometry_anchors_to_the_left_edge_in_a_narrow_window() {
        let geometry = panel_geometry(30, 20);
        assert_eq!(
            geometry.x, 0,
            "a window narrower than the panel must not underflow"
        );
        assert_eq!(geometry.width, PANEL_WIDTH);
        assert_eq!(geometry.height, 20);
    }

    // ─── apply_pop_out ───────────────────────────────────────────────

    #[test]
    fn apply_pop_out_success_captures_the_docked_width() {
        let mut state = AppState::new("%99".into());
        state.apply_pop_out(25, Ok(()));
        assert_eq!(state.docked_width, Some(25));
        assert!(state.flash.is_none(), "a successful pop-out says nothing");
    }

    #[test]
    fn apply_pop_out_refusal_flashes_and_stays_docked() {
        let mut state = AppState::new("%99".into());
        state.apply_pop_out(25, Err("can't float a pane while window is zoomed".into()));
        let flash = state.flash.as_ref().map(|(msg, _)| msg.clone());
        assert_eq!(
            flash.as_deref(),
            Some("Pop-out refused: can't float a pane while window is zoomed"),
        );
        // The mode lives on the pane, not in AppState: a refused pop-out
        // must not record anything that would make the next tick or the
        // pop-in believe the panel is floating.
        assert_eq!(state.docked_width, None);
    }

    // ─── apply_pop_in ────────────────────────────────────────────────

    #[test]
    fn apply_pop_in_returns_the_captured_width_for_the_restore() {
        let mut state = AppState::new("%99".into());
        state.docked_width = Some(25);
        assert_eq!(state.apply_pop_in(Ok(())), Some(25));
        assert_eq!(state.docked_width, None, "the capture is spent");
    }

    #[test]
    fn apply_pop_in_without_a_capture_restores_nothing() {
        // The pane was already floating when the sidebar started, so no
        // docked width was ever captured: the layout's width is left alone.
        let mut state = AppState::new("%99".into());
        assert_eq!(state.apply_pop_in(Ok(())), None);
        assert!(state.flash.is_none());
    }

    #[test]
    fn apply_pop_in_refusal_flashes_and_restores_nothing() {
        let mut state = AppState::new("%99".into());
        state.docked_width = Some(25);
        assert_eq!(state.apply_pop_in(Err("pane not found".into())), None);
        assert!(
            state
                .flash
                .as_ref()
                .is_some_and(|(msg, _)| msg.starts_with("Pop-in refused:"))
        );
        // A refused pop-in must not spend the capture — the width is
        // still there for the next successful pop-in.
        assert_eq!(state.docked_width, Some(25));
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
