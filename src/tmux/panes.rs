use std::collections::HashMap;

use super::commands::{display_message, run_tmux};
use super::options::{SIDEBAR_CURSOR, SIDEBAR_FILTER, SIDEBAR_REPO_FILTER};

/// The sidebar's own pane state, read from tmux on every refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarPaneInfo {
    pub pane_active: bool,
    pub window_active: bool,
    pub width: u16,
    /// Width of the pane's window (`#{window_width}`). `0` when the
    /// query failed.
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

/// The fixed part of the format `get_sidebar_pane_reading` expands in
/// one `display-message` call: the pane fields and the shared options,
/// each separated by one space. The repo filter is last because a repo
/// name can contain spaces, so the parse takes the rest of the line for
/// it; [`SIDEBAR_REPO_FIELD_END`] is appended after it.
const SIDEBAR_PANE_FORMAT: &str = "#{pane_active} #{window_active} #{pane_width} \
#{window_width} #{window_height} #{pane_floating_flag} #{@sidebar_filter} \
#{@sidebar_cursor} #{@sidebar_repo_filter} ";

/// The end-of-fields terminator appended after `#{@sidebar_repo_filter}`.
/// `display_message` trims its output, which would strip trailing
/// whitespace from a repo name on every tick, so the repo field is cut at
/// this token instead of at the end of the line. `/` cannot appear in a
/// repo directory name, so a value can never contain the token.
const SIDEBAR_REPO_FIELD_END: &str = "/eot/";

/// One `display-message` reading: the sidebar pane state plus the shared
/// sidebar options seen in the same call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarPaneReading {
    pub info: SidebarPaneInfo,
    /// The `@sidebar_*` options that were set at read time, keyed by
    /// option name. An option that expands to an empty string is not
    /// set and is left out, which preserves the unset behavior.
    pub options: HashMap<String, String>,
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
/// #{window_height} #{pane_floating_flag} #{@sidebar_filter} #{@sidebar_cursor}
/// #{@sidebar_repo_filter} /eot/` output. Fewer than the three core fields
/// (the pane is gone, or the query failed) yields
/// [`SidebarPaneInfo::FALLBACK`] with no options; missing window fields or
/// a missing floating flag (tmux older than 3.8 has no
/// `pane_floating_flag`) keep the parsed fields and report a zero window
/// size / not-floating.
///
/// The fields are split on single spaces so an option that expands to an
/// empty string keeps its position, and the repo filter (last field) keeps
/// the rest of the line up to the [`SIDEBAR_REPO_FIELD_END`] terminator,
/// spaces included; the single separator space before the terminator is
/// dropped so a repo name with trailing spaces round-trips intact.
pub(crate) fn parse_sidebar_pane_reading(out: &str) -> SidebarPaneReading {
    if out.split_whitespace().count() < 3 {
        return SidebarPaneReading {
            info: SidebarPaneInfo::FALLBACK,
            options: HashMap::new(),
        };
    }
    let mut fields = out.splitn(9, ' ');
    let pane_active = fields.next() == Some("1");
    let window_active = fields.next() == Some("1");
    let width = fields
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(SidebarPaneInfo::FALLBACK.width);
    let window_width = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let window_height = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let floating = fields.next() == Some("1");
    let filter = fields.next().unwrap_or("");
    let cursor = fields.next().unwrap_or("");
    let repo_field = fields.next().unwrap_or("");
    // Cut at the terminator so the `trim()` inside `display_message`
    // cannot eat trailing whitespace from the repo name. The field is
    // `value` + the single separator space + the terminator, so exactly
    // one trailing space is dropped with it.
    let repo_filter = match repo_field.rfind(SIDEBAR_REPO_FIELD_END) {
        Some(end) => repo_field[..end]
            .strip_suffix(' ')
            .unwrap_or(&repo_field[..end]),
        None => "",
    };

    let mut options = HashMap::new();
    for (key, value) in [
        (SIDEBAR_FILTER, filter),
        (SIDEBAR_CURSOR, cursor),
        (SIDEBAR_REPO_FILTER, repo_filter),
    ] {
        if !value.is_empty() {
            options.insert(key.to_string(), value.to_string());
        }
    }
    SidebarPaneReading {
        info: SidebarPaneInfo {
            pane_active,
            window_active,
            width,
            window_width,
            window_height,
            floating,
        },
        options,
    }
}

/// Query the sidebar pane state and the shared sidebar options in one
/// `display-message` call. tmux resolves `#{@sidebar_*}` through pane,
/// window, session and then global scope, so the global value a peer
/// instance wrote comes back in the same call.
pub fn get_sidebar_pane_reading(tmux_pane: &str) -> SidebarPaneReading {
    let format = format!("{SIDEBAR_PANE_FORMAT}{SIDEBAR_REPO_FIELD_END}");
    let out = display_message(tmux_pane, &format);
    parse_sidebar_pane_reading(&out)
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

    // ─── parse_sidebar_pane_reading ───────────────────────────────

    #[test]
    fn parse_sidebar_pane_reading_parses_floating_flag() {
        let reading = parse_sidebar_pane_reading("1 1 40 100 30 1");
        assert!(reading.info.floating);
        assert!(reading.info.pane_active);
        assert!(reading.info.window_active);
        assert_eq!(reading.info.width, 40);
        assert_eq!(reading.info.window_width, 100);
        assert_eq!(reading.info.window_height, 30);
        assert!(reading.options.is_empty());

        let docked = parse_sidebar_pane_reading("0 1 80 120 40 0");
        assert!(!docked.info.floating);
        assert!(!docked.info.pane_active);
        assert_eq!(docked.info.width, 80);
        assert_eq!(docked.info.window_width, 120);
        assert_eq!(docked.info.window_height, 40);
    }

    #[test]
    fn parse_sidebar_pane_reading_without_floating_field_reports_docked() {
        // tmux older than 3.8 has no `#{pane_floating_flag}`: the core
        // fields and the window size must survive and the pane must read
        // as docked.
        let reading = parse_sidebar_pane_reading("0 1 80 120 40");
        assert!(!reading.info.floating);
        assert!(!reading.info.pane_active);
        assert!(reading.info.window_active);
        assert_eq!(reading.info.width, 80);
        assert_eq!(reading.info.window_width, 120);
        assert_eq!(reading.info.window_height, 40);
    }

    #[test]
    fn parse_sidebar_pane_reading_falls_back_when_query_fails() {
        let reading = parse_sidebar_pane_reading("");
        assert_eq!(reading.info, SidebarPaneInfo::FALLBACK);
        assert!(!reading.info.floating);
        assert!(reading.options.is_empty());
    }

    #[test]
    fn parse_sidebar_pane_reading_parses_all_set_options() {
        let reading = parse_sidebar_pane_reading("1 1 40 100 30 1 waiting 7 my-repo /eot/");
        assert_eq!(reading.options.get(SIDEBAR_FILTER), Some(&"waiting".into()));
        assert_eq!(reading.options.get(SIDEBAR_CURSOR), Some(&"7".into()));
        assert_eq!(
            reading.options.get(SIDEBAR_REPO_FILTER),
            Some(&"my-repo".into())
        );
        assert_eq!(reading.options.len(), 3);
    }

    #[test]
    fn parse_sidebar_pane_reading_leaves_unset_options_out_of_the_map() {
        // `#{@sidebar_filter}` expands to an empty string when the
        // option is not set, which collapses to nothing between the
        // spaces — the cursor must keep its position and the filter
        // key must stay out of the map.
        let reading = parse_sidebar_pane_reading("0 1 80 120 40 0  12 /eot/");
        assert!(!reading.options.contains_key(SIDEBAR_FILTER));
        assert_eq!(reading.options.get(SIDEBAR_CURSOR), Some(&"12".into()));
        assert!(!reading.options.contains_key(SIDEBAR_REPO_FILTER));

        // Trailing empty fields are trimmed away by `display-message`,
        // so a set filter with unset cursor and repo filter must still
        // land on the right key.
        let only_filter = parse_sidebar_pane_reading("0 1 80 120 40 0 busy  /eot/");
        assert_eq!(
            only_filter.options.get(SIDEBAR_FILTER),
            Some(&"busy".into())
        );
        assert_eq!(only_filter.options.len(), 1);
    }

    #[test]
    fn parse_sidebar_pane_reading_keeps_spaces_in_the_repo_filter() {
        let reading = parse_sidebar_pane_reading("0 1 80 120 40 0 all 2 My Repo Dir /eot/");
        assert_eq!(
            reading.options.get(SIDEBAR_REPO_FILTER),
            Some(&"My Repo Dir".into())
        );
        assert_eq!(reading.options.get(SIDEBAR_CURSOR), Some(&"2".into()));
        assert_eq!(reading.options.get(SIDEBAR_FILTER), Some(&"all".into()));
    }

    #[test]
    fn parse_sidebar_pane_reading_round_trips_trailing_space_in_repo_filter() {
        // `display_message` trims its output; the `/eot/` terminator
        // anchors the end of the repo field so a repo name with a
        // trailing space survives the read instead of being corrupted
        // tick after tick.
        let reading = parse_sidebar_pane_reading("1 1 40 100 30 1 busy 3 My Repo  /eot/");
        assert_eq!(
            reading.options.get(SIDEBAR_REPO_FILTER),
            Some(&"My Repo ".into())
        );
        assert_eq!(reading.options.get(SIDEBAR_FILTER), Some(&"busy".into()));
        assert_eq!(reading.options.get(SIDEBAR_CURSOR), Some(&"3".into()));
    }

    #[test]
    fn parse_sidebar_pane_reading_omits_unset_repo_filter_before_terminator() {
        // Repo filter unset: the leftover separator space between the
        // cursor field and the terminator must not become a repo value.
        let reading = parse_sidebar_pane_reading("1 1 40 100 30 1 waiting 7  /eot/");
        assert!(!reading.options.contains_key(SIDEBAR_REPO_FILTER));
        assert_eq!(reading.options.get(SIDEBAR_CURSOR), Some(&"7".into()));
        assert_eq!(reading.options.get(SIDEBAR_FILTER), Some(&"waiting".into()));
    }

    #[test]
    fn parse_sidebar_pane_reading_keeps_unset_filter_position_before_repo() {
        // Filter unset, cursor unset, repo set: both empty fields must
        // be skipped so the repo lands on its own key, not on the
        // filter's.
        let reading = parse_sidebar_pane_reading("1 1 40 100 30 1   repo-x /eot/");
        assert!(!reading.options.contains_key(SIDEBAR_FILTER));
        assert!(!reading.options.contains_key(SIDEBAR_CURSOR));
        assert_eq!(
            reading.options.get(SIDEBAR_REPO_FILTER),
            Some(&"repo-x".into())
        );
    }
}
