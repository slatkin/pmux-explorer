use std::collections::HashSet;

use crate::state::LayoutKind;
use crate::tmux;

pub(crate) fn cmd_toggle(args: &[String]) -> i32 {
    let mut create_only = false;
    let mut positional = Vec::new();

    for arg in args {
        if arg == "--create-only" {
            create_only = true;
        } else {
            positional.push(arg.as_str());
        }
    }

    let window_id = match positional.first() {
        Some(id) => *id,
        None => return 0,
    };
    let pane_path = positional.get(1).copied().unwrap_or("~");

    let sidebar_position = SidebarPosition::from_setting(&tmux::display_message(
        window_id,
        &format!("#{{{}}}", tmux::SIDEBAR_POSITION),
    ));
    let is_strip = sidebar_position.axis() == LayoutKind::Strip;

    // The pane size: a column sizes from `@sidebar_width` against the
    // window width, a strip from `@sidebar_height` against the window
    // height (the same row-count-or-percent parse).
    let sidebar_size = if is_strip {
        let setting = tmux::display_message(window_id, &format!("#{{{}}}", tmux::SIDEBAR_HEIGHT));
        let window_height: u32 = tmux::display_message(window_id, "#{window_height}")
            .parse()
            .unwrap_or(0);
        resolve_height_arg(&setting, window_height)
    } else {
        let sidebar_width_setting = {
            let s = tmux::display_message(window_id, &format!("#{{{}}}", tmux::SIDEBAR_WIDTH));
            if s.is_empty() { "30".to_string() } else { s }
        };

        if sidebar_width_setting.ends_with('%') {
            let window_width: u32 = tmux::display_message(window_id, "#{window_width}")
                .parse()
                .unwrap_or(0);
            let pct: u32 = sidebar_width_setting
                .trim_end_matches('%')
                .parse()
                .unwrap_or(DEFAULT_WIDTH_PERCENT);
            if window_width > 0 && pct > 0 {
                let w = window_width * pct / 100;
                if w < 1 {
                    "1".to_string()
                } else {
                    w.to_string()
                }
            } else {
                sidebar_width_setting
            }
        } else {
            sidebar_width_setting
        }
    };

    // Check for existing sidebar
    let pane_id_role_format = pane_id_role_format();
    let panes_output = tmux::run_tmux(&["list-panes", "-t", window_id, "-F", &pane_id_role_format])
        .unwrap_or_default();

    let existing_sidebar = panes_output.lines().find_map(|line| {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() >= 2 && parts[1] == "sidebar" {
            Some(parts[0].to_string())
        } else {
            None
        }
    });

    if let Some(sidebar_pane) = existing_sidebar {
        if create_only {
            return 0;
        }
        let _ = tmux::run_tmux(&["kill-pane", "-t", &sidebar_pane]);
        return 0;
    }

    let target_pane = if is_strip {
        let pane_geometry_output = tmux::run_tmux(&[
            "list-panes",
            "-t",
            window_id,
            "-F",
            "#{pane_top} #{pane_height} #{pane_id}",
        ])
        .unwrap_or_default();
        target_pane_for_strip_position(&pane_geometry_output, sidebar_position)
    } else {
        let pane_geometry_output = tmux::run_tmux(&[
            "list-panes",
            "-t",
            window_id,
            "-F",
            "#{pane_left} #{pane_width} #{pane_id}",
        ])
        .unwrap_or_default();
        target_pane_for_position(&pane_geometry_output, sidebar_position)
    }
    .unwrap_or_else(|| window_id.to_string());
    let split_flags = split_window_flags(sidebar_position);
    let layout_setting = sidebar_layout_setting(sidebar_position);

    // Remember active pane
    let active_pane = tmux::display_message(window_id, "#{pane_id}");

    // Find our own binary path
    let self_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "pmux-explorer".to_string());

    // Create sidebar pane
    let split_args = split_window_args(
        split_flags,
        &sidebar_size,
        &target_pane,
        pane_path,
        layout_setting,
        &self_bin,
    );
    let split_arg_refs: Vec<&str> = split_args.iter().map(String::as_str).collect();
    let sidebar_pane = tmux::run_tmux(&split_arg_refs)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !sidebar_pane.is_empty() {
        tmux::set_pane_option(&sidebar_pane, tmux::PANE_ROLE, "sidebar");
        tmux::set_pane_option(&sidebar_pane, tmux::PANE_SIDEBAR_LAYOUT, layout_setting);
    }

    // Restore focus
    if !active_pane.is_empty() {
        let _ = tmux::run_tmux(&["select-pane", "-t", &active_pane]);
    } else {
        let _ = tmux::run_tmux(&["select-pane", "-t", window_id, "-l"]);
    }

    0
}

pub(crate) fn cmd_toggle_all(_args: &[String]) -> i32 {
    let pane_id_role_format = pane_id_role_format();
    let has_sidebar = tmux::run_tmux(&["list-panes", "-a", "-F", &pane_id_role_format])
        .map(|output| any_sidebar_pane(&output))
        .unwrap_or(false);

    if has_sidebar {
        let all_panes =
            tmux::run_tmux(&["list-panes", "-a", "-F", &pane_id_role_format]).unwrap_or_default();
        for line in all_panes.lines() {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() >= 2 && parts[1] == "sidebar" {
                let _ = tmux::run_tmux(&["kill-pane", "-t", parts[0]]);
            }
        }
    } else {
        let all_windows = tmux::run_tmux(&[
            "list-panes",
            "-a",
            "-F",
            "#{window_id}|#{pane_current_path}",
        ])
        .unwrap_or_default();
        for (window_id, pane_path) in unique_window_paths(&all_windows) {
            let args = vec!["--create-only".to_string(), window_id, pane_path];
            cmd_toggle(&args);
        }
    }

    0
}

fn any_sidebar_pane(output: &str) -> bool {
    output.lines().any(|line| {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        parts.len() >= 2 && parts[1] == "sidebar"
    })
}

/// The percentage applied when `@sidebar_width` holds a percentage
/// whose number will not parse. The conf seeds the option with the
/// same number (see the pin test below), so the two must move
/// together.
const DEFAULT_WIDTH_PERCENT: u32 = 15;

fn unique_window_paths(output: &str) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut windows = Vec::new();

    for line in output.lines() {
        let Some((window_id, pane_path)) = line.split_once('|') else {
            continue;
        };
        if seen.insert(window_id.to_string()) {
            windows.push((window_id.to_string(), pane_path.to_string()));
        }
    }

    windows
}

/// Which side of the window the sidebar pane is created on, driven by
/// the `@sidebar_position` tmux option. `Left` and `Right` make a
/// full-height column; `Top` and `Bottom` make a full-width strip (the
/// `strip-layout` capability).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SidebarPosition {
    Left,
    Right,
    Top,
    Bottom,
}

impl SidebarPosition {
    /// Parse the raw `@sidebar_position` option value. An explicit
    /// (case-insensitive, whitespace-tolerant) `right`, `top` or
    /// `bottom` selects that side; everything else — including unset,
    /// empty, or invalid values — falls back to the historical default
    /// of `left`, so a typo never moves the sidebar somewhere unexpected.
    pub(crate) fn from_setting(setting: &str) -> Self {
        let value = setting.trim();
        if value.eq_ignore_ascii_case("right") {
            Self::Right
        } else if value.eq_ignore_ascii_case("top") {
            Self::Top
        } else if value.eq_ignore_ascii_case("bottom") {
            Self::Bottom
        } else {
            Self::Left
        }
    }

    /// The layout the position creates: `Top` and `Bottom` make a strip,
    /// `Left` and `Right` a column. The layout is fixed when the sidebar
    /// is made, and [`Self::opposite`] and the side switch stay on this
    /// axis.
    pub(crate) fn axis(self) -> LayoutKind {
        match self {
            Self::Top | Self::Bottom => LayoutKind::Strip,
            Self::Left | Self::Right => LayoutKind::Column,
        }
    }

    /// The side a sidebar pane docks on, read off its own
    /// `#{pane_left}`: column 0 is the left edge, anything else is
    /// the right. `None` when tmux did not answer with a column (a
    /// dead socket in tests), so the caller can fall back to the
    /// option. The side switch derives its direction from the pane,
    /// not the option — in a mixed layout the two can disagree, and
    /// the spec's "opposite edge" is the pane's.
    pub(crate) fn from_pane_left(pane_left: &str) -> Option<Self> {
        match pane_left.trim().parse::<u32>() {
            Ok(0) => Some(Self::Left),
            Ok(_) => Some(Self::Right),
            Err(_) => None,
        }
    }

    /// The side a strip pane docks on, read off its own
    /// `#{pane_at_top}`: a pane touching the window's top edge is a top
    /// strip, a `0` flag a bottom strip. `None` when tmux did not answer
    /// with the flag (a dead socket in tests, an older tmux), so the
    /// caller can fall back — the same contract as [`Self::from_pane_left`]
    /// for a column.
    pub(crate) fn from_pane_at_top(at_top: &str) -> Option<Self> {
        match at_top.trim() {
            "1" => Some(Self::Top),
            "0" => Some(Self::Bottom),
            _ => None,
        }
    }

    /// The option value this side writes back to `@sidebar_position`
    /// (the lowercase spelling the config seeds and `from_setting`
    /// parses).
    pub(crate) fn as_setting(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }

    /// The side across the window — on the same axis: left and right
    /// swap, top and bottom swap, and a position never crosses axes
    /// (a strip never becomes a column).
    pub(crate) fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// Horizontal placement of one pane, parsed from a
/// `#{pane_left} #{pane_width} #{pane_id}` formatted `list-panes` line.
#[derive(Debug, Eq, PartialEq)]
struct PaneGeometry {
    left: u32,
    width: u32,
    pane_id: String,
}

/// Parse a single `list-panes` output line into a [`PaneGeometry`].
/// Returns `None` for malformed lines so callers can simply skip them.
fn parse_pane_geometry(line: &str) -> Option<PaneGeometry> {
    let mut parts = line.split_whitespace();
    let left = parts.next()?.parse().ok()?;
    let width = parts.next()?.parse().ok()?;
    let pane_id = parts.next()?.to_string();
    Some(PaneGeometry {
        left,
        width,
        pane_id,
    })
}

/// Pick the pane the sidebar splits from: the leftmost pane for a left
/// sidebar, or the pane with the largest right edge (`left + width`) for
/// a right sidebar, so the new pane always lands at the window's outer
/// edge. Returns `None` when no line of `output` parses as geometry; a
/// strip picks its target from vertical geometry instead (see
/// [`target_pane_for_strip_position`]).
pub(crate) fn target_pane_for_position(output: &str, position: SidebarPosition) -> Option<String> {
    let panes = output.lines().filter_map(parse_pane_geometry);
    match position {
        SidebarPosition::Left => panes.min_by_key(|pane| pane.left),
        SidebarPosition::Right => panes.max_by_key(|pane| pane.left.saturating_add(pane.width)),
        SidebarPosition::Top | SidebarPosition::Bottom => None,
    }
    .map(|pane| pane.pane_id)
}

/// Vertical placement of one pane, parsed from a
/// `#{pane_top} #{pane_height} #{pane_id}` formatted `list-panes` line.
#[derive(Debug, Eq, PartialEq)]
struct PaneVerticalGeometry {
    top: u32,
    height: u32,
    pane_id: String,
}

/// Parse a single `list-panes` output line into a
/// [`PaneVerticalGeometry`]. Returns `None` for malformed lines so
/// callers can simply skip them.
fn parse_pane_vertical_geometry(line: &str) -> Option<PaneVerticalGeometry> {
    let mut parts = line.split_whitespace();
    let top = parts.next()?.parse().ok()?;
    let height = parts.next()?.parse().ok()?;
    let pane_id = parts.next()?.to_string();
    Some(PaneVerticalGeometry {
        top,
        height,
        pane_id,
    })
}

/// The vertical twin of [`target_pane_for_position`]: the topmost pane
/// for a top strip, or the pane with the largest bottom edge
/// (`top + height`) for a bottom strip, so the new pane always lands at
/// the window's outer edge. Returns `None` when no line of `output`
/// parses as geometry.
pub(crate) fn target_pane_for_strip_position(
    output: &str,
    position: SidebarPosition,
) -> Option<String> {
    let panes = output.lines().filter_map(parse_pane_vertical_geometry);
    match position {
        SidebarPosition::Top => panes.min_by_key(|pane| pane.top),
        SidebarPosition::Bottom => panes.max_by_key(|pane| pane.top.saturating_add(pane.height)),
        _ => None,
    }
    .map(|pane| pane.pane_id)
}

/// `split-window` flags for each placement. Columns: `-hfb` inserts the
/// new pane before the target (left of it), `-hf` after it (right of
/// it); strips: `-vfb` above the target, `-vf` below it. The `f`
/// variants span the window's other dimension in full, so a column is
/// full height and a strip is full width.
fn split_window_flags(position: SidebarPosition) -> &'static str {
    match position {
        SidebarPosition::Left => "-hfb",
        SidebarPosition::Right => "-hf",
        SidebarPosition::Top => "-vfb",
        SidebarPosition::Bottom => "-vf",
    }
}

/// The layout value handed to a new sidebar for `position`: written once
/// into the process environment (`-e PMUX_SIDEBAR_LAYOUT=<layout>`) and
/// once onto the pane (`@pane_sidebar_layout`). A strip for top/bottom,
/// a column for left/right.
fn sidebar_layout_setting(position: SidebarPosition) -> &'static str {
    match position {
        SidebarPosition::Top | SidebarPosition::Bottom => "strip",
        SidebarPosition::Left | SidebarPosition::Right => "column",
    }
}

/// Pure builder for the `split-window` argument list that creates the
/// sidebar pane, kept separate from the tmux call (the pattern
/// `commands.rs` uses for its builders) so a unit test can assert the
/// exact arguments for each position. `layout` reaches the new process
/// through `-e PMUX_SIDEBAR_LAYOUT=<layout>`: `split-window` starts the
/// TUI before `toggle` can write any pane option, so the environment is
/// the only way the TUI can learn its layout at startup.
fn split_window_args(
    flags: &str,
    size: &str,
    target: &str,
    pane_path: &str,
    layout: &str,
    self_bin: &str,
) -> Vec<String> {
    vec![
        "split-window".to_string(),
        flags.to_string(),
        "-l".to_string(),
        size.to_string(),
        "-t".to_string(),
        target.to_string(),
        "-c".to_string(),
        pane_path.to_string(),
        "-e".to_string(),
        format!("PMUX_SIDEBAR_LAYOUT={layout}"),
        "-P".to_string(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
        self_bin.to_string(),
    ]
}

/// The percentage applied when `@sidebar_height` is unset or empty, or
/// when it holds a percentage whose number will not parse. The conf
/// seeds the option with the same number (see the pin test below), so
/// the two must move together.
const DEFAULT_HEIGHT_PERCENT: u32 = 30;

/// Resolve the raw `@sidebar_height` setting into a `split-window -l`
/// argument for a strip, with the same rules as the width parse: a row
/// count passes through; a `N%` value becomes `window_height * N / 100`
/// rows (at least 1); a percent whose number will not parse falls back
/// to [`DEFAULT_HEIGHT_PERCENT`]. An unset or empty option is the
/// default percent of the window height, per the `strip-layout` spec.
/// When the window height is unknown (0) the raw setting passes through
/// untouched, like the width parse does.
fn resolve_height_arg(setting: &str, window_height: u32) -> String {
    let setting = setting.trim();
    if setting.is_empty() {
        return if window_height > 0 {
            (window_height * DEFAULT_HEIGHT_PERCENT / 100)
                .max(1)
                .to_string()
        } else {
            format!("{DEFAULT_HEIGHT_PERCENT}%")
        };
    }
    let Some(pct_str) = setting.strip_suffix('%') else {
        return setting.to_string();
    };
    let pct: u32 = pct_str.parse().unwrap_or(DEFAULT_HEIGHT_PERCENT);
    if window_height > 0 && pct > 0 {
        let h = window_height * pct / 100;
        if h < 1 {
            "1".to_string()
        } else {
            h.to_string()
        }
    } else {
        setting.to_string()
    }
}

/// Decide whether `cmd_auto_close` should kill the window, given the raw
/// outputs of the tmux queries it performs. Extracted as a pure function
/// so the guard logic is directly unit-testable without a running tmux
/// server.
///
/// - `list_panes_output`: `Some(stdout)` from `list-panes -F <pane role format>`,
///   or `None` if the tmux call failed.
/// - `session_windows`: parsed value of `#{session_windows}`, or `None`
///   if the tmux call failed or the value was unparseable.
/// - `session_attached`: parsed value of `#{session_attached}`, or `None`
///   if the tmux call failed or the value was unparseable.
fn should_kill_window(
    list_panes_output: Option<&str>,
    session_windows: Option<u32>,
    session_attached: Option<u32>,
) -> bool {
    // `list-panes` failed or returned nothing: the window is either gone
    // already or tmux is too busy to answer. Do NOT treat "no output"
    // as "no non-sidebar panes" — that would let us kill a live window
    // whose query happened to race with another tmux command.
    let Some(output) = list_panes_output else {
        return false;
    };
    if output.trim().is_empty() {
        return false;
    }

    let non_sidebar = output.lines().filter(|line| *line != "sidebar").count();
    if non_sidebar != 0 {
        return false;
    }

    let Some(windows) = session_windows else {
        return false;
    };

    // Last window in the session: killing it destroys the session and
    // drops every attached client. One attached client is fine — that
    // matches normal tmux `exit` behaviour on the last pane. Two or
    // more means a shared session (e.g. several terminal tabs attached
    // to `main`) where we cannot tell which clients are "wanted", so
    // preserve the sidebar instead. A missing `session_attached` errs
    // on the side of preservation.
    match windows {
        0 => false,
        1 => matches!(session_attached, Some(n) if n <= 1),
        _ => true,
    }
}

pub(crate) fn cmd_auto_close(args: &[String]) -> i32 {
    let window_id = match args.first() {
        Some(id) => id.as_str(),
        None => return 0,
    };

    let pane_role_format = format!("#{{{}}}", tmux::PANE_ROLE);
    let list_panes_output =
        tmux::run_tmux(&["list-panes", "-t", window_id, "-F", &pane_role_format]);

    let session_windows = tmux::run_tmux(&[
        "display-message",
        "-t",
        window_id,
        "-p",
        "#{session_windows}",
    ])
    .and_then(|s| s.trim().parse().ok());

    let session_attached = tmux::run_tmux(&[
        "display-message",
        "-t",
        window_id,
        "-p",
        "#{session_attached}",
    ])
    .and_then(|s| s.trim().parse().ok());

    if should_kill_window(
        list_panes_output.as_deref(),
        session_windows,
        session_attached,
    ) {
        let _ = tmux::run_tmux(&["kill-window", "-t", window_id]);
    }

    0
}

fn pane_id_role_format() -> String {
    format!("#{{pane_id}}|#{{{}}}", tmux::PANE_ROLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_sidebar_pane_detects_sidebar_anywhere() {
        let output = "%1|pane\n%2|sidebar\n%3|pane";
        assert!(any_sidebar_pane(output));
    }

    #[test]
    fn any_sidebar_pane_returns_false_without_sidebar() {
        let output = "%1|pane\n%2|main";
        assert!(!any_sidebar_pane(output));
    }

    #[test]
    fn unique_window_paths_deduplicates_windows_and_keeps_spaces() {
        let output = "%1|/Users/me/My Project\n%1|/Users/me/My Project\n%2|/tmp/another project";
        assert_eq!(
            unique_window_paths(output),
            vec![
                ("%1".to_string(), "/Users/me/My Project".to_string()),
                ("%2".to_string(), "/tmp/another project".to_string()),
            ]
        );
    }

    #[test]
    fn unique_window_paths_skips_malformed_lines() {
        let output = "bad-line\n%1|/tmp";
        assert_eq!(
            unique_window_paths(output),
            vec![("%1".to_string(), "/tmp".to_string())]
        );
    }

    // ─── sidebar placement ───────────────────────────────────────────

    #[test]
    fn sidebar_position_parses_right_only() {
        assert_eq!(
            SidebarPosition::from_setting("right"),
            SidebarPosition::Right
        );
        assert_eq!(
            SidebarPosition::from_setting(" RIGHT "),
            SidebarPosition::Right
        );
        assert_eq!(SidebarPosition::from_setting("left"), SidebarPosition::Left);
        assert_eq!(SidebarPosition::from_setting(""), SidebarPosition::Left);
        assert_eq!(
            SidebarPosition::from_setting("invalid"),
            SidebarPosition::Left
        );
    }

    #[test]
    fn sidebar_position_parses_the_strip_values() {
        assert_eq!(SidebarPosition::from_setting("top"), SidebarPosition::Top);
        assert_eq!(SidebarPosition::from_setting(" TOP "), SidebarPosition::Top);
        assert_eq!(
            SidebarPosition::from_setting("bottom"),
            SidebarPosition::Bottom
        );
        assert_eq!(
            SidebarPosition::from_setting("Bottom"),
            SidebarPosition::Bottom
        );
        // The spec's unknown-value scenario: any other value makes a
        // left column.
        assert_eq!(
            SidebarPosition::from_setting("middle"),
            SidebarPosition::Left
        );
    }

    #[test]
    fn every_position_round_trips_through_its_setting() {
        for (position, setting) in [
            (SidebarPosition::Left, "left"),
            (SidebarPosition::Right, "right"),
            (SidebarPosition::Top, "top"),
            (SidebarPosition::Bottom, "bottom"),
        ] {
            assert_eq!(position.as_setting(), setting);
            assert_eq!(SidebarPosition::from_setting(setting), position);
        }
    }

    #[test]
    fn opposite_stays_on_the_axis() {
        assert_eq!(SidebarPosition::Left.opposite(), SidebarPosition::Right);
        assert_eq!(SidebarPosition::Right.opposite(), SidebarPosition::Left);
        assert_eq!(SidebarPosition::Top.opposite(), SidebarPosition::Bottom);
        assert_eq!(SidebarPosition::Bottom.opposite(), SidebarPosition::Top);
    }

    #[test]
    fn axis_maps_the_positions_to_the_layouts() {
        assert_eq!(SidebarPosition::Left.axis(), LayoutKind::Column);
        assert_eq!(SidebarPosition::Right.axis(), LayoutKind::Column);
        assert_eq!(SidebarPosition::Top.axis(), LayoutKind::Strip);
        assert_eq!(SidebarPosition::Bottom.axis(), LayoutKind::Strip);
    }

    #[test]
    fn target_pane_for_left_position_uses_leftmost_pane() {
        let output = "40 80 %3\n0 20 %1\n20 20 %2";

        assert_eq!(
            target_pane_for_position(output, SidebarPosition::Left),
            Some("%1".to_string())
        );
    }

    #[test]
    fn target_pane_for_right_position_uses_largest_right_edge() {
        let output = "0 20 %1\n20 20 %2\n40 80 %3";

        assert_eq!(
            target_pane_for_position(output, SidebarPosition::Right),
            Some("%3".to_string())
        );
    }

    #[test]
    fn target_pane_for_position_skips_malformed_lines() {
        let output = "bad-line\n0 nope %1\n12 30 %2";

        assert_eq!(
            target_pane_for_position(output, SidebarPosition::Left),
            Some("%2".to_string())
        );
        assert_eq!(target_pane_for_position("", SidebarPosition::Right), None);
    }

    #[test]
    fn target_pane_for_top_strip_uses_the_topmost_pane() {
        let output = "10 20 %3\n0 10 %1\n5 15 %2";

        assert_eq!(
            target_pane_for_strip_position(output, SidebarPosition::Top),
            Some("%1".to_string())
        );
    }

    #[test]
    fn target_pane_for_bottom_strip_uses_the_largest_bottom_edge() {
        // Bottom edges: 20, 25 and 35 — %3 reaches lowest.
        let output = "0 20 %1\n10 15 %2\n5 30 %3";

        assert_eq!(
            target_pane_for_strip_position(output, SidebarPosition::Bottom),
            Some("%3".to_string())
        );
    }

    #[test]
    fn target_pane_for_strip_position_skips_malformed_lines() {
        let output = "bad-line\n0 nope %1\n5 10 %2";

        assert_eq!(
            target_pane_for_strip_position(output, SidebarPosition::Top),
            Some("%2".to_string())
        );
        assert_eq!(
            target_pane_for_strip_position("", SidebarPosition::Bottom),
            None
        );
    }

    #[test]
    fn split_window_flags_match_tmux_side_semantics() {
        assert_eq!(split_window_flags(SidebarPosition::Left), "-hfb");
        assert_eq!(split_window_flags(SidebarPosition::Right), "-hf");
        assert_eq!(split_window_flags(SidebarPosition::Top), "-vfb");
        assert_eq!(split_window_flags(SidebarPosition::Bottom), "-vf");
    }

    #[test]
    fn split_window_args_carry_the_layout_for_each_position() {
        for (position, flags, layout) in [
            (SidebarPosition::Left, "-hfb", "column"),
            (SidebarPosition::Right, "-hf", "column"),
            (SidebarPosition::Top, "-vfb", "strip"),
            (SidebarPosition::Bottom, "-vf", "strip"),
        ] {
            let args = split_window_args(
                split_window_flags(position),
                "12",
                "%9",
                "~",
                sidebar_layout_setting(position),
                "/bin/pmux-explorer",
            );
            assert_eq!(
                args,
                vec![
                    "split-window".to_string(),
                    flags.to_string(),
                    "-l".to_string(),
                    "12".to_string(),
                    "-t".to_string(),
                    "%9".to_string(),
                    "-c".to_string(),
                    "~".to_string(),
                    "-e".to_string(),
                    format!("PMUX_SIDEBAR_LAYOUT={layout}"),
                    "-P".to_string(),
                    "-F".to_string(),
                    "#{pane_id}".to_string(),
                    "/bin/pmux-explorer".to_string(),
                ]
            );
        }
    }

    #[test]
    fn sidebar_layout_setting_names_the_axis() {
        assert_eq!(sidebar_layout_setting(SidebarPosition::Left), "column");
        assert_eq!(sidebar_layout_setting(SidebarPosition::Right), "column");
        assert_eq!(sidebar_layout_setting(SidebarPosition::Top), "strip");
        assert_eq!(sidebar_layout_setting(SidebarPosition::Bottom), "strip");
    }

    #[test]
    fn sidebar_height_resolves_against_the_window_height() {
        assert_eq!(resolve_height_arg("30%", 40), "12", "30% of 40 rows");
        assert_eq!(
            resolve_height_arg("10", 40),
            "10",
            "a row count passes through"
        );
        assert_eq!(
            resolve_height_arg("", 40),
            "12",
            "an unset option is the default 30%"
        );
        assert_eq!(
            resolve_height_arg("  ", 40),
            "12",
            "an empty option is the default 30%"
        );
        assert_eq!(
            resolve_height_arg("abc%", 40),
            "12",
            "a malformed percent falls back to the default 30%"
        );
        assert_eq!(
            resolve_height_arg("0%", 40),
            "0%",
            "a zero percent passes through untouched, like the width parse"
        );
        assert_eq!(
            resolve_height_arg("30%", 0),
            "30%",
            "an unknown window height leaves the raw setting"
        );
        assert_eq!(
            resolve_height_arg("", 0),
            "30%",
            "an unset option with an unknown window height stays the raw percent"
        );
    }

    // ─── should_kill_window ───────────────────────────────────────────

    #[test]
    fn should_kill_window_kills_when_only_sidebar_and_other_windows_exist() {
        // Classic intended path: sidebar alone in a window, session has
        // other windows to fall back on. Attached-client count is
        // irrelevant because killing this window does not end the
        // session.
        assert!(should_kill_window(Some("sidebar"), Some(2), None));
        assert!(should_kill_window(Some("sidebar"), Some(2), Some(0)));
        assert!(should_kill_window(Some("sidebar"), Some(2), Some(5)));
    }

    #[test]
    fn should_kill_window_skips_when_non_sidebar_pane_remains() {
        // Another pane with `@pane_role` explicitly set to something
        // non-sidebar (e.g. a spawn-marked pane) keeps the window alive.
        assert!(!should_kill_window(Some("sidebar\npane"), Some(5), Some(1)));
        // `@pane_role` unset renders as an empty line — that pane is
        // a regular user pane, not a sidebar, so the window must stay.
        // The real tmux output for [sidebar pane, regular pane] is
        // "sidebar\n\n" (sidebar's role, then the regular pane's empty
        // role followed by the final record separator).
        assert!(!should_kill_window(Some("sidebar\n\n"), Some(5), Some(1)));
        assert!(!should_kill_window(Some("\nsidebar\n"), Some(5), Some(1)));
    }

    #[test]
    fn should_kill_window_skips_when_list_panes_failed() {
        // `list-panes` failure must never be treated as "window is empty" —
        // that used to let a busy-tmux race kill a live window.
        assert!(!should_kill_window(None, Some(5), Some(1)));
    }

    #[test]
    fn should_kill_window_skips_when_list_panes_empty() {
        // Whitespace-only output (e.g. window already gone) must not
        // trigger a kill either.
        assert!(!should_kill_window(Some(""), Some(5), Some(1)));
        assert!(!should_kill_window(Some("   \n"), Some(5), Some(1)));
    }

    #[test]
    fn should_kill_window_kills_last_window_when_single_client_attached() {
        // One client attached to a single-window session: destroying
        // the session only detaches the same client that just kept the
        // session alive, which matches tmux's standard `exit` behaviour
        // on the last pane — the user expects the sidebar to go with it.
        assert!(should_kill_window(Some("sidebar"), Some(1), Some(1)));
    }

    #[test]
    fn should_kill_window_kills_last_window_when_detached() {
        // No clients attached: killing the session harms no one, and
        // a stranded sidebar in a detached session is pointless anyway.
        assert!(should_kill_window(Some("sidebar"), Some(1), Some(0)));
    }

    #[test]
    fn should_kill_window_preserves_last_window_when_multiple_clients_attached() {
        // Core regression guard (0dc6e99): killing the last window of
        // a session drops every attached client. With multiple terminal
        // tabs sharing a single `main` session, that manifested as every
        // tab dying at once. Keep the sidebar stranded rather than nuke
        // the session.
        assert!(!should_kill_window(Some("sidebar"), Some(1), Some(2)));
        assert!(!should_kill_window(Some("sidebar"), Some(1), Some(7)));
    }

    #[test]
    fn should_kill_window_preserves_last_window_when_attached_query_failed() {
        // Without knowing how many clients are attached we cannot prove
        // the kill is safe. Better a lingering sidebar pane than a
        // mass-disconnect.
        assert!(!should_kill_window(Some("sidebar"), Some(1), None));
    }

    #[test]
    fn should_kill_window_skips_when_session_windows_query_failed() {
        // If we cannot prove the session has other windows, err on the
        // side of preservation. Better to leave a lingering sidebar
        // pane than to destroy a live workspace.
        assert!(!should_kill_window(Some("sidebar"), None, Some(1)));
        assert!(!should_kill_window(Some("sidebar"), Some(0), Some(1)));
    }

    #[test]
    fn the_conf_seeds_the_same_width_percent_the_binary_falls_back_to() {
        // One percentage, two homes: the conf seeds `@sidebar_width`
        // with `15%` and [`DEFAULT_WIDTH_PERCENT`] is the fallback when
        // the seeded value's number will not parse. A test pins them
        // together so a change that updates one copy and not the other
        // fails here instead of drifting in a user's session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_width "))
            .map(|(_, rest)| rest.trim().trim_matches('\'').to_owned());
        let expected = format!("{DEFAULT_WIDTH_PERCENT}%");
        assert_eq!(
            seeded.as_deref(),
            Some(expected.as_str()),
            "the conf's seeded width must match DEFAULT_WIDTH_PERCENT"
        );
    }

    #[test]
    fn the_conf_seeds_the_same_height_percent_the_binary_falls_back_to() {
        // One percentage, two homes: the conf seeds `@sidebar_height`
        // with `30%` and [`DEFAULT_HEIGHT_PERCENT`] is the fallback when
        // the option is unset, empty, or its number will not parse. A
        // test pins them together so a change that updates one copy and
        // not the other fails here instead of drifting in a user's
        // session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_height "))
            .map(|(_, rest)| rest.trim().trim_matches('\'').to_owned());
        let expected = format!("{DEFAULT_HEIGHT_PERCENT}%");
        assert_eq!(
            seeded.as_deref(),
            Some(expected.as_str()),
            "the conf's seeded height must match DEFAULT_HEIGHT_PERCENT"
        );
    }

    #[test]
    fn the_conf_seeds_the_same_position_the_binary_falls_back_to() {
        // One side, two homes: the conf seeds `@sidebar_position` for
        // tmux-side visibility and [`SidebarPosition::from_setting`]
        // falls back to `Left` when the option is unset or invalid. A
        // test pins them together so a change that updates one copy and
        // not the other fails here instead of drifting in a user's
        // session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_position "))
            .map(|(_, rest)| SidebarPosition::from_setting(rest.trim().trim_matches('\'')));
        assert_eq!(
            seeded,
            Some(SidebarPosition::from_setting("")),
            "the conf's seeded position must match the unset fallback"
        );
    }
}
