//! Keep the sidebar pane from being squeezed below a usable width.
//!
//! Orchestrator agents spawn new panes in the sidebar's window, and tmux's
//! split layout can shrink the sidebar to a few columns. On every refresh
//! tick the guard compares the sidebar pane's width against
//! `@sidebar_min_width` and grows the pane back with `resize-pane -x` when
//! it falls below the minimum. It only ever grows: widths at or above the
//! minimum are left alone, so a larger manual size sticks.

use crate::tmux;

/// Minimum width applied when `@sidebar_min_width` is unset.
pub const DEFAULT_MIN_WIDTH: u32 = 20;

/// Parse `@sidebar_min_width`. Unset, empty, or non-numeric values return
/// `None` (the caller then applies [`DEFAULT_MIN_WIDTH`]); `0` is valid and
/// disables enforcement.
pub fn parse_min_width(raw: Option<&str>) -> Option<u32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u32>().ok())
}

/// Width to request for the sidebar pane, or `None` to leave it alone.
///
/// `min` is the parsed `@sidebar_min_width`: `None` (unset/invalid) means
/// [`DEFAULT_MIN_WIDTH`] applies, `0` disables the guard, and any larger
/// value is the floor. Returns `None` when the pane is already at or above
/// the floor.
pub fn resize_target(pane_width: u16, min: Option<u32>) -> Option<u32> {
    let min = min.unwrap_or(DEFAULT_MIN_WIDTH);
    if min == 0 || u32::from(pane_width) >= min {
        return None;
    }
    Some(min)
}

/// Grow the sidebar pane back to `@sidebar_min_width` when it is below the
/// minimum. Runs once per refresh tick while the sidebar's window is active;
/// a width of 0 (terminal size unknown) is ignored so a bad probe can never
/// trigger a resize.
pub fn enforce(pane_id: &str, pane_width: u16) {
    if pane_width == 0 {
        return;
    }
    let min = parse_min_width(tmux::get_option(tmux::SIDEBAR_MIN_WIDTH).as_deref());
    if let Some(width) = resize_target(pane_width, min) {
        let _ = tmux::run_tmux(&["resize-pane", "-t", pane_id, "-x", &width.to_string()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_target_applies_default_when_unset() {
        assert_eq!(resize_target(8, None), Some(DEFAULT_MIN_WIDTH));
        // At or above the floor the pane is left alone.
        assert_eq!(resize_target(DEFAULT_MIN_WIDTH as u16, None), None);
        assert_eq!(resize_target(60, None), None);
    }

    #[test]
    fn resize_target_zero_disables_guard() {
        assert_eq!(resize_target(5, Some(0)), None);
    }

    #[test]
    fn resize_target_explicit_min_grows_only() {
        assert_eq!(resize_target(8, Some(12)), Some(12));
        assert_eq!(resize_target(12, Some(12)), None);
        assert_eq!(resize_target(30, Some(12)), None);
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
}
