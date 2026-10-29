//! Keep the sidebar pane within a usable width band.
//!
//! Orchestrator agents spawn new panes in the sidebar's window, and tmux's
//! split layout can shrink the sidebar to a few columns; conversely, tmux's
//! resize behaviour can hand the sidebar more columns than it asked for,
//! which shows up when it docks on the right. On every refresh tick the
//! guard compares the sidebar pane's width against `@sidebar_min_width` and
//! `@sidebar_max_width` and corrects the pane with `resize-pane -x`: it
//! grows the pane back when it falls below the minimum and shrinks it when
//! it grows past the maximum. Within the band the pane is left alone, so a
//! manual size inside the band sticks.

use crate::state::LayoutKind;
use crate::tmux;

/// Minimum width applied when `@sidebar_min_width` is unset.
pub const DEFAULT_MIN_WIDTH: u32 = 35;

/// Parse `@sidebar_min_width` / `@sidebar_max_width`. Unset, empty, or
/// non-numeric values return `None` (the caller then applies
/// [`DEFAULT_MIN_WIDTH`] to the minimum and leaves the maximum off);
/// `0` is valid and disables that bound.
pub fn parse_min_width(raw: Option<&str>) -> Option<u32> {
    parse_width(raw)
}

/// Parse `@sidebar_max_width`, with the same rules as
/// [`parse_min_width`] — unset or invalid means no maximum, `0` disables.
pub fn parse_max_width(raw: Option<&str>) -> Option<u32> {
    parse_width(raw)
}

fn parse_width(raw: Option<&str>) -> Option<u32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u32>().ok())
}

/// Width to request for the sidebar pane, or `None` to leave it alone.
///
/// A strip is never resized here: it is as wide as the window, so a
/// `@sidebar_max_width` would shrink it into a narrow strip in a corner
/// (design decision 10 of `add-strip-layout`). [`LayoutKind::Strip`]
/// always returns `None`; a column gets the band rule below.
///
/// `min` is the parsed `@sidebar_min_width`: `None` (unset/invalid) means
/// [`DEFAULT_MIN_WIDTH`] applies, `0` disables the floor. `max` is the
/// parsed `@sidebar_max_width`: `None` or `0` disables the ceiling, and a
/// maximum at or below the effective minimum is ignored — the two bounds
/// would fight every tick, so the floor wins.
///
/// Returns `None` when the pane already sits inside the band.
pub fn guard_target(
    layout: LayoutKind,
    pane_width: u16,
    min: Option<u32>,
    max: Option<u32>,
) -> Option<u32> {
    match layout {
        LayoutKind::Strip => None,
        LayoutKind::Column => resize_target(pane_width, min, max),
    }
}

/// Width to request for the sidebar pane, or `None` to leave it alone.
///
/// `min` is the parsed `@sidebar_min_width`: `None` (unset/invalid) means
/// [`DEFAULT_MIN_WIDTH`] applies, `0` disables the floor. `max` is the
/// parsed `@sidebar_max_width`: `None` or `0` disables the ceiling, and a
/// maximum at or below the effective minimum is ignored — the two bounds
/// would fight every tick, so the floor wins.
///
/// Returns `None` when the pane already sits inside the band.
pub fn resize_target(pane_width: u16, min: Option<u32>, max: Option<u32>) -> Option<u32> {
    let min = min.unwrap_or(DEFAULT_MIN_WIDTH);
    if min != 0 && u32::from(pane_width) < min {
        return Some(min);
    }
    if let Some(max) = max.filter(|max| *max > 0 && *max > min)
        && u32::from(pane_width) > max
    {
        return Some(max);
    }
    None
}

/// Correct the sidebar pane's width once per refresh tick while the
/// sidebar's window is active: grow it back to `@sidebar_min_width` when
/// squeezed below the minimum, shrink it to `@sidebar_max_width` when it
/// grew past the maximum. A width of 0 (terminal size unknown) is ignored
/// so a bad probe can never trigger a resize. A strip is never touched —
/// see [`guard_target`].
pub fn enforce(pane_id: &str, pane_width: u16, layout: LayoutKind) {
    if pane_width == 0 {
        return;
    }
    let min = parse_min_width(tmux::get_option(tmux::SIDEBAR_MIN_WIDTH).as_deref());
    let max = parse_max_width(tmux::get_option(tmux::SIDEBAR_MAX_WIDTH).as_deref());
    if let Some(width) = guard_target(layout, pane_width, min, max) {
        let _ = tmux::run_tmux(&["resize-pane", "-t", pane_id, "-x", &width.to_string()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_target_leaves_a_strip_alone_at_any_width() {
        // A strip is as wide as the window: the band rule would shrink it
        // into a narrow strip in a corner, so no width is ever requested,
        // not even for a pane far outside the band.
        assert_eq!(guard_target(LayoutKind::Strip, 8, None, None), None);
        assert_eq!(guard_target(LayoutKind::Strip, 8, Some(35), Some(60)), None);
        assert_eq!(guard_target(LayoutKind::Strip, 200, None, Some(60)), None);
    }

    #[test]
    fn guard_target_gives_a_column_the_band_rule() {
        assert_eq!(
            guard_target(LayoutKind::Column, 8, None, None),
            resize_target(8, None, None)
        );
        assert_eq!(guard_target(LayoutKind::Column, 60, None, None), None);
    }

    #[test]
    fn resize_target_applies_default_when_unset() {
        assert_eq!(resize_target(8, None, None), Some(DEFAULT_MIN_WIDTH));
        // At or above the floor the pane is left alone.
        assert_eq!(resize_target(DEFAULT_MIN_WIDTH as u16, None, None), None);
        assert_eq!(resize_target(60, None, None), None);
    }

    #[test]
    fn resize_target_zero_min_disables_the_floor() {
        assert_eq!(resize_target(5, Some(0), None), None);
    }

    #[test]
    fn resize_target_explicit_min_grows_only() {
        assert_eq!(resize_target(8, Some(12), None), Some(12));
        assert_eq!(resize_target(12, Some(12), None), None);
        assert_eq!(resize_target(30, Some(12), None), None);
    }

    #[test]
    fn resize_target_shrinks_a_pane_past_the_max() {
        assert_eq!(resize_target(60, None, Some(40)), Some(40));
        // At or below the ceiling — and above the default floor — the
        // pane is left alone.
        assert_eq!(resize_target(40, None, Some(40)), None);
        assert_eq!(resize_target(38, None, Some(40)), None);
    }

    #[test]
    fn resize_target_zero_or_unset_max_disables_the_ceiling() {
        assert_eq!(resize_target(60, None, None), None);
        assert_eq!(resize_target(60, None, Some(0)), None);
    }

    #[test]
    fn resize_target_ignores_a_max_at_or_below_the_min() {
        // The bounds would fight every tick — grow to the min, shrink to
        // the max — so the floor wins and the ceiling is ignored.
        assert_eq!(resize_target(60, Some(20), Some(20)), None);
        assert_eq!(resize_target(60, Some(20), Some(10)), None);
        // The same band still grows a squeezed pane.
        assert_eq!(resize_target(8, Some(20), Some(10)), Some(20));
    }

    #[test]
    fn resize_target_enforces_both_bounds_in_one_band() {
        assert_eq!(resize_target(8, Some(20), Some(40)), Some(20));
        assert_eq!(resize_target(50, Some(20), Some(40)), Some(40));
        assert_eq!(resize_target(30, Some(20), Some(40)), None);
    }

    #[test]
    fn parse_min_width_falls_back_on_unset_or_invalid() {
        assert_eq!(parse_min_width(None), None);
        assert_eq!(parse_min_width(Some("")), None);
        assert_eq!(parse_min_width(Some("  ")), None);
        assert_eq!(parse_min_width(Some("abc")), None);
        // Whitespace and explicit values parse cleanly; 0 stays valid.
        assert_eq!(parse_min_width(Some(" 12 ")), Some(12));
        assert_eq!(parse_min_width(Some("0")), Some(0));
    }

    #[test]
    fn parse_max_width_follows_the_same_rules() {
        assert_eq!(parse_max_width(None), None);
        assert_eq!(parse_max_width(Some("abc")), None);
        assert_eq!(parse_max_width(Some(" 60 ")), Some(60));
        assert_eq!(parse_max_width(Some("0")), Some(0));
    }

    #[test]
    fn the_conf_seeds_the_same_default_the_binary_falls_back_to() {
        // One number, two homes: the conf seeds `@sidebar_min_width` for
        // tmux-side visibility (its seed-everything-unset policy) and
        // [`DEFAULT_MIN_WIDTH`] is the fallback when it is unset. The
        // copies can drift silently — a changed constant that leaves the
        // seed behind runs 20 against 35 forever — so a test pins them
        // together and drift fails here instead.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .filter_map(|line| line.split_once("set -g @sidebar_min_width "))
            .filter_map(|(_, rest)| rest.trim().trim_matches('\'').parse::<u32>().ok())
            .next();
        assert_eq!(seeded, Some(DEFAULT_MIN_WIDTH));
    }
}
