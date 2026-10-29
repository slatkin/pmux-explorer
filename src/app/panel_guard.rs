//! Keep the floating panel as tall and as wide as its rule asks, anchored
//! to its edge.
//!
//! A floating pane keeps its size and position when the window resizes.
//! When the window grows, tmux leaves the pane's left (or top) edge where
//! it was, so a panel that was flush with an edge drifts away from the new
//! edge; when the window shrinks, tmux pulls the pane back into the
//! window and clamps it, so the panel stays inside on its own. On
//! every refresh tick, next to the width guard ([`super::size_guard`]),
//! the guard re-asserts the rule with `resize-pane` and `move-pane` when
//! the window size changed — but only when the size actually changed
//! since the previous tick, so a panel the user resized by hand is not
//! fought while the window stays the same size.
//!
//! The rule depends on the layout ([`LayoutKind`]): a floating column is
//! 40 columns wide and as tall as the window, anchored to the right edge
//! (a height change re-asserts the height alone, a width change the width
//! and the anchor). A floating strip is as wide as the window and as high
//! as the docked strip was, anchored to the edge it was docked at — the
//! height is left as the user set it, and both a width and a height
//! change re-assert the width and the anchor, because a bottom strip's
//! anchor moves with the window height.

use crate::cli::toggle::SidebarPosition;
use crate::state::{LayoutKind, PanelGeometry, panel_geometry};
use crate::tmux;

/// The layout rule the floating-panel guard re-asserts: the layout kind
/// plus the strip's captured anchor (the docked edge and the docked
/// height, both captured on pop-out). A column needs no capture — its
/// edge is always the right edge and its size the window height — so its
/// `edge` is `None` and its `docked_size` ignored. A strip whose edge was
/// never captured (the pane floated before the sidebar started) acts on
/// no anchor, so the guard leaves it alone; see [`position_target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelRule {
    pub layout: LayoutKind,
    pub edge: Option<SidebarPosition>,
    pub docked_size: u16,
}

/// Width and position to re-assert for the floating panel, or `None` to
/// leave the pane alone.
///
/// The guard acts only while the pane is floating, only once the previous
/// window width is known, and only when the window size changed: for a
/// column a width change, for a strip a width or a height change (the
/// bottom-edge anchor moves with the window height). A width of 0 (the
/// query failed, or tmux is too old to report the floating flag
/// alongside the window size) is ignored so a bad probe can never trigger
/// a resize or a move.
///
/// A strip whose docked edge was never captured (the pane floated before
/// the sidebar started) acts on no anchor, so it is left alone: the strip
/// rule is not knowable without the edge. `docked_size` is the captured
/// docked height of a strip and is ignored by the column rule, which
/// takes the window height instead.
pub fn position_target(
    window_width: u16,
    window_height: u16,
    last_window_width: Option<u16>,
    last_window_height: Option<u16>,
    floating: bool,
    rule: PanelRule,
) -> Option<PanelGeometry> {
    if !floating || window_width == 0 {
        return None;
    }
    let last = last_window_width?;
    match rule.layout {
        LayoutKind::Column => {
            if window_width == last {
                return None;
            }
            Some(panel_geometry(
                LayoutKind::Column,
                SidebarPosition::Left,
                window_width,
                window_height,
                0,
            ))
        }
        LayoutKind::Strip => {
            let edge = rule.edge?;
            let height_changed = last_window_height.is_some_and(|last| window_height != last);
            if window_width == last && !height_changed {
                return None;
            }
            Some(panel_geometry(
                rule.layout,
                edge,
                window_width,
                window_height,
                rule.docked_size,
            ))
        }
    }
}

/// The guard acts only while the pane is floating, only once the previous
/// window height is known, and only when the window height changed. A
/// height of 0 (the query failed, or tmux is too old to report the
/// floating flag alongside the window size) is ignored so a bad probe can
/// never trigger a resize.
///
/// A strip gets no height re-assert: its floating height is the docked
/// height the user had, not the window height, and the design leaves the
/// strip's height as the user set it.
pub fn height_target(
    window_height: u16,
    last_window_height: Option<u16>,
    floating: bool,
    rule: PanelRule,
) -> Option<u16> {
    if rule.layout == LayoutKind::Strip {
        return None;
    }
    if !floating || window_height == 0 {
        return None;
    }
    let last = last_window_height?;
    if window_height == last {
        return None;
    }
    Some(window_height)
}

/// Re-assert the floating panel's rule after a window size change: the
/// height a column floats at, then the width and the anchored position
/// for both layouts. Runs once per refresh tick while the sidebar's
/// window is active; the run loop's tick-to-tick memory of the window
/// size is updated here on every call, floating or not, so a resize that
/// happened while docked does not count as a change right after a
/// pop-out.
pub fn enforce(
    pane_id: &str,
    window_width: u16,
    window_height: u16,
    last_window_width: &mut Option<u16>,
    last_window_height: &mut Option<u16>,
    floating: bool,
    rule: PanelRule,
) {
    if let Some(height) = height_target(window_height, *last_window_height, floating, rule) {
        let _ = tmux::run_tmux(&["resize-pane", "-t", pane_id, "-y", &height.to_string()]);
    }
    if let Some(position) = position_target(
        window_width,
        window_height,
        *last_window_width,
        *last_window_height,
        floating,
        rule,
    ) {
        let _ = tmux::run_tmux(&[
            "resize-pane",
            "-t",
            pane_id,
            "-x",
            &position.width.to_string(),
        ]);
        // A floating pane keeps its left edge across a window resize, so
        // after a window grows the width re-assert alone would leave the
        // panel short of the new right edge (or, for a bottom strip, the
        // window's new bottom edge); `move-pane` (never a re-float —
        // `break-pane -W` fails on an already floating pane) puts it
        // back flush.
        let _ = tmux::move_pane(pane_id, position.x, position.y);
    }
    *last_window_width = Some(window_width);
    *last_window_height = Some(window_height);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_rule() -> PanelRule {
        PanelRule {
            layout: LayoutKind::Column,
            edge: None,
            docked_size: 0,
        }
    }

    fn strip_rule(edge: Option<SidebarPosition>, docked_size: u16) -> PanelRule {
        PanelRule {
            layout: LayoutKind::Strip,
            edge,
            docked_size,
        }
    }

    // ─── height_target ───────────────────────────────────────────────

    #[test]
    fn height_target_reasserts_after_a_window_height_change() {
        assert_eq!(height_target(40, Some(30), true, column_rule()), Some(40));
    }

    #[test]
    fn height_target_leaves_an_unchanged_window_alone() {
        assert_eq!(height_target(40, Some(40), true, column_rule()), None);
    }

    #[test]
    fn height_target_waits_for_a_known_height_on_the_first_tick() {
        // No previous height yet: nothing to compare against, so a
        // hand-resized panel is not snapped on the first tick.
        assert_eq!(height_target(40, None, true, column_rule()), None);
    }

    #[test]
    fn height_target_ignores_a_docked_pane() {
        assert_eq!(height_target(40, Some(30), false, column_rule()), None);
    }

    #[test]
    fn height_target_ignores_an_unknown_window_height() {
        assert_eq!(height_target(0, Some(30), true, column_rule()), None);
    }

    #[test]
    fn height_target_leaves_a_strip_alone() {
        // A floating strip's height is the docked height the user had,
        // not the window height: a `resize-pane -y` to the window height
        // would stretch it, so the strip's height is left as the user
        // set it.
        assert_eq!(
            height_target(40, Some(30), true, strip_rule(None, 10)),
            None
        );
        assert_eq!(height_target(40, None, true, strip_rule(None, 10)), None);
    }

    // ─── position_target ─────────────────────────────────────────────

    #[test]
    fn position_target_reasserts_after_a_window_width_change() {
        assert_eq!(
            position_target(100, 40, Some(120), Some(40), true, column_rule()),
            Some(PanelGeometry {
                width: 40,
                height: 40,
                x: 60,
                y: 0,
            })
        );
    }

    #[test]
    fn position_target_leaves_an_unchanged_window_alone() {
        assert_eq!(
            position_target(100, 40, Some(100), Some(40), true, column_rule()),
            None
        );
    }

    #[test]
    fn position_target_waits_for_a_known_width_on_the_first_tick() {
        // No previous width yet: nothing to compare against, so a
        // hand-moved panel is not snapped on the first tick.
        assert_eq!(
            position_target(100, 40, None, Some(40), true, column_rule()),
            None
        );
    }

    #[test]
    fn position_target_ignores_a_docked_pane() {
        assert_eq!(
            position_target(100, 40, Some(120), Some(40), false, column_rule()),
            None
        );
    }

    #[test]
    fn position_target_ignores_an_unknown_window_width() {
        assert_eq!(
            position_target(0, 40, Some(120), Some(40), true, column_rule()),
            None
        );
    }

    #[test]
    fn position_target_anchors_to_the_left_edge_in_a_narrow_window() {
        assert_eq!(
            position_target(30, 20, Some(80), Some(20), true, column_rule()),
            Some(PanelGeometry {
                width: 40,
                height: 20,
                x: 0,
                y: 0,
            })
        );
    }

    #[test]
    fn position_target_keeps_a_floating_strip_as_wide_as_the_window_at_its_edge() {
        // A width change re-asserts the strip rule: as wide as the
        // window, the captured docked height, flush with the captured
        // edge — here a bottom strip of 10 rows in a 30-row window.
        assert_eq!(
            position_target(
                100,
                30,
                Some(120),
                Some(30),
                true,
                strip_rule(Some(SidebarPosition::Bottom), 10)
            ),
            Some(PanelGeometry {
                width: 100,
                height: 10,
                x: 0,
                y: 20,
            })
        );
    }

    #[test]
    fn position_target_moves_a_bottom_strip_after_a_window_height_change() {
        // A height-only change moves the window's bottom edge, which a
        // bottom strip's anchor follows — the width is already right, but
        // the move still re-asserts both.
        assert_eq!(
            position_target(
                100,
                35,
                Some(100),
                Some(30),
                true,
                strip_rule(Some(SidebarPosition::Bottom), 10)
            ),
            Some(PanelGeometry {
                width: 100,
                height: 10,
                x: 0,
                y: 25,
            })
        );
    }

    #[test]
    fn position_target_keeps_a_top_strip_flush_with_the_top_edge() {
        assert_eq!(
            position_target(
                90,
                30,
                Some(120),
                Some(30),
                true,
                strip_rule(Some(SidebarPosition::Top), 12)
            ),
            Some(PanelGeometry {
                width: 90,
                height: 12,
                x: 0,
                y: 0,
            })
        );
    }

    #[test]
    fn position_target_leaves_an_unchanged_strip_window_alone() {
        assert_eq!(
            position_target(
                100,
                30,
                Some(100),
                Some(30),
                true,
                strip_rule(Some(SidebarPosition::Bottom), 10)
            ),
            None
        );
    }

    #[test]
    fn position_target_leaves_a_strip_without_a_captured_edge_alone() {
        // The pane floated before the sidebar started, so no docked edge
        // was ever captured and the strip's anchor is unknowable — the
        // guard leaves the pane alone rather than guess an edge.
        assert_eq!(
            position_target(100, 30, Some(120), Some(30), true, strip_rule(None, 10)),
            None
        );
    }

    #[test]
    fn position_target_ignores_a_docked_strip() {
        assert_eq!(
            position_target(
                100,
                30,
                Some(120),
                Some(30),
                false,
                strip_rule(Some(SidebarPosition::Bottom), 10)
            ),
            None
        );
    }
}
