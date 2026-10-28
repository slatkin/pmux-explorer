//! Switching the docked sidebar's side, for every window at once.
//!
//! The side lives in the global `@sidebar_position` tmux option — the
//! one source of truth `cli/toggle.rs` creates sidebars from. The
//! action reads this pane's own edge (`#{pane_left}` — the option
//! only as a fallback) so the direction is always "opposite of where
//! this sidebar sits", even in a mixed layout a skipped earlier
//! switch left behind. It then moves every window's sidebar pane to
//! the opposite edge with `join-pane`, in one sweep over a single
//! `list-panes -a` listing (see `tmux::switch_sidebar_sides`), and
//! writes the option even when some windows were skipped, so sidebars
//! created later follow the new side; skipped windows keep their old
//! side until the next switch.
//!
//! While the sidebar floats the action does nothing: the floating
//! panel keeps its hard-coded right-edge anchor, the control glyph is
//! drawn dimmed, and no tmux call is made at all. The flash text
//! reporting the moved and skipped counts is applied here, from the
//! sweep's result, the way `popout.rs` applies its outcomes — the
//! applying half takes the results as parameters so tests can drive
//! them without a live tmux.

use crate::cli::toggle::SidebarPosition;
use crate::tmux;

/// The flash text for a finished sweep: how many windows moved and,
/// only when some were skipped, how many. A switch that moved or
/// skipped at least one window always reports; a sweep over a server
/// with no sidebars stays silent.
fn switch_flash_text(side: SidebarPosition, moved: usize, skipped: usize) -> String {
    let windows = if moved == 1 { "window" } else { "windows" };
    if skipped == 0 {
        format!(
            "Sidebar side switched to {}: {moved} {windows} moved",
            side.as_setting()
        )
    } else {
        format!(
            "Sidebar side switched to {}: {moved} {windows} moved, {skipped} skipped",
            side.as_setting()
        )
    }
}

impl AppState {
    /// The action behind the side-switch control and tmux prefix +
    /// Ctrl-O: move the docked sidebar of every window to the opposite
    /// edge and point the global `@sidebar_position` at the new side.
    /// A floating sidebar does nothing — the panel keeps its
    /// right-edge anchor and no tmux call is made.
    pub fn switch_sidebar_side(&mut self) {
        if self.sidebar_pane.floating {
            return;
        }
        // The direction comes off this pane's own edge: a window an
        // earlier switch skipped still docks on the old side while the
        // option already names the new one, and "opposite edge" is
        // the pane's. The option is the fallback for a pane whose
        // geometry tmux will not say (a dead socket).
        let current = SidebarPosition::from_pane_left(&tmux::display_message(
            &self.tmux_pane,
            "#{pane_left}",
        ))
        .unwrap_or_else(|| {
            SidebarPosition::from_setting(&tmux::display_message(
                &self.tmux_pane,
                &format!("#{{{}}}", tmux::SIDEBAR_POSITION),
            ))
        });
        let new_side = current.opposite();
        let (moved, skipped) = tmux::switch_sidebar_sides(new_side);
        tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_POSITION, new_side.as_setting()]);
        self.apply_side_switch(new_side, moved, skipped);
    }

    /// Apply the sweep's outcome: flash the moved and skipped counts.
    /// The option write already happened in [`Self::switch_sidebar_side`]
    /// — even with skips, so new windows follow the new side.
    pub(crate) fn apply_side_switch(
        &mut self,
        new_side: SidebarPosition,
        moved: usize,
        skipped: usize,
    ) {
        if moved + skipped == 0 {
            return;
        }
        self.set_flash(switch_flash_text(new_side, moved, skipped));
    }
}

use super::AppState;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── apply_side_switch ───────────────────────────────────────────

    #[test]
    fn apply_side_switch_reports_moved_and_skipped_counts() {
        let mut state = AppState::new("%99".into());
        state.apply_side_switch(SidebarPosition::Right, 3, 1);
        assert_eq!(
            state.flash.as_ref().map(|(msg, _)| msg.as_str()),
            Some("Sidebar side switched to right: 3 windows moved, 1 skipped")
        );
    }

    #[test]
    fn apply_side_switch_with_no_skips_names_no_skip_count() {
        let mut state = AppState::new("%99".into());
        state.apply_side_switch(SidebarPosition::Left, 4, 0);
        assert_eq!(
            state.flash.as_ref().map(|(msg, _)| msg.as_str()),
            Some("Sidebar side switched to left: 4 windows moved")
        );
    }

    #[test]
    fn apply_side_switch_singularizes_one_moved_window() {
        let mut state = AppState::new("%99".into());
        state.apply_side_switch(SidebarPosition::Right, 1, 2);
        assert_eq!(
            state.flash.as_ref().map(|(msg, _)| msg.as_str()),
            Some("Sidebar side switched to right: 1 window moved, 2 skipped")
        );
    }

    #[test]
    fn apply_side_switch_over_an_empty_server_stays_silent() {
        // No sidebars anywhere: nothing moved, nothing skipped, and
        // the spec's flash only covers a switch that did at least one
        // of the two.
        let mut state = AppState::new("%99".into());
        state.apply_side_switch(SidebarPosition::Right, 0, 0);
        assert!(state.flash.is_none());
    }

    // ─── the floating guard ──────────────────────────────────────────

    #[test]
    fn a_floating_sidebar_does_nothing() {
        // While floating the action is a no-op before any tmux call:
        // no sweep, no option write, no flash. The isolated test tmux
        // would swallow the calls anyway, so the flash is what proves
        // the action never ran — a floating switch that somehow swept
        // an empty server would look identical, but a floating switch
        // that swept the test fixture's records would flash.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane.floating = true;
        state.switch_sidebar_side();
        assert!(state.flash.is_none());
    }

    #[test]
    fn a_docked_sidebar_over_an_unreachable_server_stays_silent() {
        // The probe cannot read the side (dead socket in tests), the
        // sweep finds no sidebars and the option write goes nowhere —
        // nothing observable happened, so nothing is flashed.
        let mut state = AppState::new("%4294967295".into());
        assert!(!state.sidebar_pane.floating);
        state.switch_sidebar_side();
        assert!(state.flash.is_none());
    }
}
