use super::commands::{display_message, run_tmux};

/// The sidebar's own pane state, read from tmux on every refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarPaneInfo {
    pub pane_active: bool,
    pub window_active: bool,
    pub width: u16,
    /// Width of the pane's window (`#{window_width}`). `0` when the query
    /// failed or tmux is too old to report the floating flag.
    pub window_width: u16,
    /// Height of the pane's window (`#{window_height}`). In a default
    /// session this counts the status-line row, so the pane area can be
    /// one row shorter.
    pub window_height: u16,
    /// Whether the pane is a floating pane (`#{pane_floating_flag}`,
    /// tmux 3.8+). Always `false` when tmux is too old to report the
    /// flag, so the sidebar behaves as docked there.
    pub floating: bool,
}

impl SidebarPaneInfo {
    /// Values used when the pane is gone or the query failed: not focused,
    /// not the active window, and the historical default pane width. The
    /// window size is unknown (`0`), so nothing sizes from it.
    pub(crate) const FALLBACK: Self = Self {
        pane_active: false,
        window_active: false,
        width: 28,
        window_width: 0,
        window_height: 0,
        floating: false,
    };
}

/// Parse the `#{pane_active} #{window_active} #{pane_width} #{window_width}
/// #{window_height} #{pane_floating_flag}` output. Fewer than the three
/// core fields (the pane is gone, or the query failed) yields
/// [`SidebarPaneInfo::FALLBACK`]; missing window fields or a missing
/// floating flag (tmux older than 3.8 has no `pane_floating_flag`) keep
/// the parsed fields and report a zero window size / not-floating.
pub(crate) fn parse_sidebar_pane_info(out: &str) -> SidebarPaneInfo {
    let parts: Vec<&str> = out.split_whitespace().collect();
    if parts.len() < 3 {
        return SidebarPaneInfo::FALLBACK;
    }
    SidebarPaneInfo {
        pane_active: parts[0] == "1",
        window_active: parts[1] == "1",
        width: parts[2].parse().unwrap_or(SidebarPaneInfo::FALLBACK.width),
        window_width: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
        window_height: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
        floating: parts.get(5).copied() == Some("1"),
    }
}

pub fn get_sidebar_pane_info(tmux_pane: &str) -> SidebarPaneInfo {
    let out = display_message(
        tmux_pane,
        "#{pane_active} #{window_active} #{pane_width} \
         #{window_width} #{window_height} #{pane_floating_flag}",
    );
    parse_sidebar_pane_info(&out)
}

pub fn get_pane_path(pane_id: &str) -> Option<String> {
    Some(display_message(pane_id, "#{pane_current_path}")).filter(|s| !s.is_empty())
}

/// Query tmux for all panes in the active window, returning (pane_id, pane_active, path).
/// This queries tmux directly and is NOT filtered by agent type, so it includes
/// all panes (shell, editor, etc.) — not just agent panes.
pub fn query_active_window_panes() -> Vec<(String, bool, String)> {
    // List panes in the current (active) window across all sessions
    let output = match run_tmux(&[
        "list-panes",
        "-F",
        "#{pane_id}|#{pane_active}|#{pane_current_path}",
    ]) {
        Some(s) => s,
        None => return vec![],
    };
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() < 3 {
                return None;
            }
            Some((parts[0].to_string(), parts[1] == "1", parts[2].to_string()))
        })
        .collect()
}

/// Find the focused (non-sidebar) pane ID and path by querying tmux directly.
/// Returns all panes regardless of agent type, so activity/git info can be shown
/// even for non-agent panes.
pub fn find_active_pane(sidebar_pane: &str) -> Option<(String, String)> {
    pick_active_pane(sidebar_pane, &query_active_window_panes())
}

/// Pure logic: pick the active non-sidebar pane from a list.
/// Returns the pane with pane_active=true (excluding sidebar) if one exists.
/// Returns None when the sidebar itself is active or no valid pane is found,
/// so callers can preserve the previously focused pane.
pub(crate) fn pick_active_pane(
    sidebar_pane: &str,
    panes: &[(String, bool, String)],
) -> Option<(String, String)> {
    let valid = |p: &&(String, bool, String)| p.0 != sidebar_pane && !p.2.is_empty();
    panes
        .iter()
        .find(|p| p.1 && valid(p))
        .map(|p| (p.0.clone(), p.2.clone()))
}

/// Find the focused pane's working directory by querying tmux directly.
/// Used by the background git thread which doesn't have access to AppState.
/// Queries all panes (not just agent panes) so git info is available
/// even when the focused pane has no agent running.
pub fn focused_pane_path(sidebar_pane: &str) -> Option<String> {
    find_active_pane(sidebar_pane).map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_active_pane_returns_active_non_sidebar() {
        let panes = vec![
            ("%1".into(), false, "/home".into()),
            ("%2".into(), true, "/work".into()),
            ("%3".into(), false, "/tmp".into()),
        ];
        assert_eq!(
            pick_active_pane("%99", &panes),
            Some(("%2".into(), "/work".into()))
        );
    }

    #[test]
    fn pick_active_pane_skips_sidebar_even_when_marked_active() {
        let panes = vec![("%99".into(), true, "/a".into())];
        assert!(pick_active_pane("%99", &panes).is_none());
    }

    #[test]
    fn pick_active_pane_skips_panes_with_empty_path() {
        let panes = vec![
            ("%1".into(), true, "".into()),
            ("%2".into(), true, "/ok".into()),
        ];
        assert_eq!(
            pick_active_pane("%99", &panes),
            Some(("%2".into(), "/ok".into()))
        );
    }

    #[test]
    fn pick_active_pane_returns_none_for_empty_list() {
        assert!(pick_active_pane("%99", &[]).is_none());
    }

    #[test]
    fn pick_active_pane_returns_none_when_no_active() {
        let panes = vec![
            ("%1".into(), false, "/x".into()),
            ("%2".into(), false, "/y".into()),
        ];
        assert!(pick_active_pane("%99", &panes).is_none());
    }

    // ─── parse_sidebar_pane_info ──────────────────────────────────

    #[test]
    fn parse_sidebar_pane_info_parses_floating_flag() {
        let info = parse_sidebar_pane_info("1 1 40 100 30 1");
        assert!(info.floating);
        assert!(info.pane_active);
        assert!(info.window_active);
        assert_eq!(info.width, 40);
        assert_eq!(info.window_width, 100);
        assert_eq!(info.window_height, 30);

        let docked = parse_sidebar_pane_info("0 1 80 120 40 0");
        assert!(!docked.floating);
        assert!(!docked.pane_active);
        assert_eq!(docked.width, 80);
        assert_eq!(docked.window_width, 120);
        assert_eq!(docked.window_height, 40);
    }

    #[test]
    fn parse_sidebar_pane_info_without_floating_field_reports_docked() {
        // tmux older than 3.8 has no `#{pane_floating_flag}`: the core
        // fields and the window size must survive and the pane must read
        // as docked.
        let info = parse_sidebar_pane_info("0 1 80 120 40");
        assert!(!info.floating);
        assert!(!info.pane_active);
        assert!(info.window_active);
        assert_eq!(info.width, 80);
        assert_eq!(info.window_width, 120);
        assert_eq!(info.window_height, 40);
    }

    #[test]
    fn parse_sidebar_pane_info_falls_back_when_query_fails() {
        let info = parse_sidebar_pane_info("");
        assert_eq!(info, SidebarPaneInfo::FALLBACK);
        assert!(!info.floating);
    }
}
