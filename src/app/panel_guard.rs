//! Keep the floating panel as tall and as wide as its window, anchored
//! to the window's right edge.
//!
//! A floating pane keeps its size and position when the window resizes, so
//! a panel popped out of a 120-column, 40-row window would keep both after
//! the window shrinks — and on a narrower window it hangs off the side. On
//! every refresh tick, next to the width guard ([`super::size_guard`]),
//! the guard re-asserts the panel height with `resize-pane -y` when the
//! window height changed, and the panel width with `resize-pane -x` plus
//! its right-edge position with `move-pane` when the window width changed
//! — but only when the window size actually changed since the previous
//! tick, so a panel the user resized by hand is not fought while the
//! window stays the same size.

use crate::state::{PanelGeometry, panel_geometry};
use crate::tmux;

/// Width and position to re-assert for the floating panel after a window
/// width change, taken from [`popout::panel_geometry`] so the guard and
/// the pop-out share one source for the panel's size and right-edge
/// anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelPosition {
    pub width: u16,
    /// Column of the panel's upper-left corner.
    pub x: u16,
    /// Row of the panel's upper-left corner.
    pub y: u16,
}

/// Width and position to re-assert for the floating panel, or `None` to
/// leave the pane alone.
///
/// The guard acts only while the pane is floating, only once the previous
/// window width is known, and only when the window width changed. A width
/// of 0 (the query failed, or tmux is too old to report the floating flag
/// alongside the window size) is ignored so a bad probe can never trigger
/// a resize or a move.
pub fn position_target(
    window_width: u16,
    window_height: u16,
    last_window_width: Option<u16>,
    floating: bool,
) -> Option<PanelPosition> {
    if !floating || window_width == 0 {
        return None;
    }
    let last = last_window_width?;
    if window_width == last {
        return None;
    }
    let PanelGeometry { width, x, y, .. } = panel_geometry(window_width, window_height);
    Some(PanelPosition { width, x, y })
}

/// The guard acts only while the pane is floating, only once the previous
/// window height is known, and only when the window height changed. A
/// height of 0 (the query failed, or tmux is too old to report the
/// floating flag alongside the window size) is ignored so a bad probe can
/// never trigger a resize.
pub fn height_target(
    window_height: u16,
    last_window_height: Option<u16>,
    floating: bool,
) -> Option<u16> {
    if !floating || window_height == 0 {
        return None;
    }
    let last = last_window_height?;
    if window_height == last {
        return None;
    }
    Some(window_height)
}

/// Re-assert the floating panel's height, width and right-edge position
/// after a window size change. Runs once per refresh tick while the
/// sidebar's window is active; the run loop's tick-to-tick memory of the
/// window size is updated here on every call, floating or not, so a
/// resize that happened while docked does not count as a change right
/// after a pop-out.
pub fn enforce(
    pane_id: &str,
    window_width: u16,
    window_height: u16,
    last_window_width: &mut Option<u16>,
    last_window_height: &mut Option<u16>,
    floating: bool,
) {
    if let Some(height) = height_target(window_height, *last_window_height, floating) {
        let _ = tmux::run_tmux(&["resize-pane", "-t", pane_id, "-y", &height.to_string()]);
    }
    if let Some(position) =
        position_target(window_width, window_height, *last_window_width, floating)
    {
        let _ = tmux::run_tmux(&[
            "resize-pane",
            "-t",
            pane_id,
            "-x",
            &position.width.to_string(),
        ]);
        // A floating pane keeps its position across a window resize, so
        // the width re-assert alone would leave the panel hanging off the
        // right edge; `move-pane` (never a re-float — `break-pane -W`
        // fails on an already floating pane) puts it back flush.
        let _ = tmux::move_pane(pane_id, position.x, position.y);
    }
    *last_window_width = Some(window_width);
    *last_window_height = Some(window_height);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_target_reasserts_after_a_window_height_change() {
        assert_eq!(height_target(40, Some(30), true), Some(40));
    }

    #[test]
    fn height_target_leaves_an_unchanged_window_alone() {
        assert_eq!(height_target(40, Some(40), true), None);
    }

    #[test]
    fn height_target_waits_for_a_known_height_on_the_first_tick() {
        // No previous height yet: nothing to compare against, so a
        // hand-resized panel is not snapped on the first tick.
        assert_eq!(height_target(40, None, true), None);
    }

    #[test]
    fn height_target_ignores_a_docked_pane() {
        assert_eq!(height_target(40, Some(30), false), None);
    }

    #[test]
    fn height_target_ignores_an_unknown_window_height() {
        assert_eq!(height_target(0, Some(30), true), None);
    }

    // ─── position_target ─────────────────────────────────────────────

    #[test]
    fn position_target_reasserts_after_a_window_width_change() {
        assert_eq!(
            position_target(100, 40, Some(120), true),
            Some(PanelPosition {
                width: 40,
                x: 60,
                y: 0,
            })
        );
    }

    #[test]
    fn position_target_leaves_an_unchanged_window_alone() {
        assert_eq!(position_target(100, 40, Some(100), true), None);
    }

    #[test]
    fn position_target_waits_for_a_known_width_on_the_first_tick() {
        // No previous width yet: nothing to compare against, so a
        // hand-moved panel is not snapped on the first tick.
        assert_eq!(position_target(100, 40, None, true), None);
    }

    #[test]
    fn position_target_ignores_a_docked_pane() {
        assert_eq!(position_target(100, 40, Some(120), false), None);
    }

    #[test]
    fn position_target_ignores_an_unknown_window_width() {
        assert_eq!(position_target(0, 40, Some(120), true), None);
    }

    #[test]
    fn position_target_anchors_to_the_left_edge_in_a_narrow_window() {
        assert_eq!(
            position_target(30, 20, Some(80), true),
            Some(PanelPosition {
                width: 40,
                x: 0,
                y: 0,
            })
        );
    }
}
