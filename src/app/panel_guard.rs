//! Keep the floating panel as tall as its window.
//!
//! A floating pane keeps its size when the window resizes, so a panel
//! popped out of a 40-row window would keep that height after the window
//! grows. On every refresh tick, next to the width guard ([`super::
//! size_guard`]), the guard re-asserts the panel height with
//! `resize-pane -y` — but only when the window height actually changed
//! since the previous tick, so a panel the user resized by hand is not
//! fought while the window stays the same size.

use crate::tmux;

/// Height to re-assert for the floating panel, or `None` to leave the
/// pane alone.
///
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

/// Re-assert the floating panel's height after a window height change.
/// Runs once per refresh tick while the sidebar's window is active;
/// `last_window_height` is the run loop's tick-to-tick memory of the
/// window height and is updated here on every call, floating or not, so
/// a height change that happened while docked does not count as a change
/// right after a pop-out.
pub fn enforce(
    pane_id: &str,
    window_height: u16,
    last_window_height: &mut Option<u16>,
    floating: bool,
) {
    if let Some(height) = height_target(window_height, *last_window_height, floating) {
        let _ = tmux::run_tmux(&["resize-pane", "-t", pane_id, "-y", &height.to_string()]);
    }
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
}
