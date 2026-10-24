pub mod bottom;
pub mod colors;
pub mod icons;
pub mod notices;
pub mod panes;
pub mod pet;
pub mod text;

use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::state::clamp_bottom_height;
use crate::{state::AppState, tmux};

pub const BOTTOM_PANEL_HEIGHT: u16 = 20;

/// The bottom panel height used when `@sidebar_bottom_height` is unset,
/// empty, or not an integer from 0 to 100, as a percent of the sidebar
/// height.
pub const DEFAULT_BOTTOM_PERCENT: u8 = 30;

/// Rows reserved between the pane list and the bottom panel when the pet is
/// enabled. The pet and its desk/chair all render inside this band so they
/// never overdraw the pane list above or the bottom panel's border below.
pub const PET_SCENE_HEIGHT: u16 = 5;

/// Read `@sidebar_bottom_height` from tmux global options, falling back to the default.
/// A value of 0 hides the bottom panel entirely.
pub fn bottom_panel_height_from_options(opts: &HashMap<String, String>) -> u16 {
    opts.get(tmux::SIDEBAR_BOTTOM_HEIGHT)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(BOTTOM_PANEL_HEIGHT)
}

pub fn bottom_panel_height_from_tmux() -> u16 {
    let opts = tmux::get_all_global_options();
    bottom_panel_height_from_options(&opts)
}

/// Read `@sidebar_bottom_height` as a percent of the sidebar height,
/// falling back to [`DEFAULT_BOTTOM_PERCENT`]. An integer from 0 to 100
/// is the percent — an unset option, an empty value, or any other value
/// gives the default. A percent of 0 hides the bottom panel entirely.
pub fn bottom_percent_from_options(opts: &HashMap<String, String>) -> u8 {
    opts.get(tmux::SIDEBAR_BOTTOM_HEIGHT)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u8>().ok())
        .filter(|p| *p <= 100)
        .unwrap_or(DEFAULT_BOTTOM_PERCENT)
}

/// Read `@sidebar_pet` from tmux global options, defaulting to `false` (off).
/// Accepts `on`/`off`, `true`/`false`, `1`/`0` (case-insensitive).
pub fn pet_enabled_from_options(opts: &HashMap<String, String>) -> bool {
    opts.get(tmux::SIDEBAR_PET)
        .map(|s| s.trim().to_ascii_lowercase())
        .map(|s| matches!(s.as_str(), "on" | "true" | "1" | "yes"))
        .unwrap_or(false)
}

pub fn pet_enabled_from_tmux() -> bool {
    let opts = crate::tmux::get_all_global_options();
    pet_enabled_from_options(&opts)
}

// ── public entry point ──────────────────────────────────────────────

pub fn draw(frame: &mut Frame, state: &mut AppState) {
    state.layout.hyperlink_overlays.clear();
    let area = frame.area();

    let bot_h = if state.sidebar_pane.floating {
        // A floating panel keeps the agent list readable: the Activity/Git
        // bottom panel takes at most a third of the panel height.
        clamp_bottom_height(state.bottom_panel_height, area.height)
    } else {
        state.bottom_panel_height
    };
    // Publish the height this frame actually drew so the mouse handlers
    // hit-test the drawn geometry (`FrameLayout::bottom_panel_height`),
    // never the raw configured height.
    state.layout.bottom_panel_height = bot_h;
    let divider_h = if bot_h > 0 && state.pet_enabled {
        PET_SCENE_HEIGHT
    } else {
        1
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if bot_h > 0 {
            vec![
                Constraint::Min(1),
                Constraint::Length(divider_h),
                Constraint::Length(bot_h),
            ]
        } else {
            vec![Constraint::Min(1)]
        })
        .split(area);

    panes::draw_agents(frame, state, chunks[0]);

    if bot_h > 0 && chunks.len() > 2 {
        bottom::draw_bottom(frame, state, chunks[2]);
        if state.pet_enabled {
            let running_count = state.running_count();
            pet::draw_pet(frame, state, chunks[1], running_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts_with(key: &str, value: &str) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(key.into(), value.into());
        m
    }

    #[test]
    fn bottom_height_defaults_when_option_missing() {
        let opts = HashMap::new();
        assert_eq!(bottom_panel_height_from_options(&opts), BOTTOM_PANEL_HEIGHT);
    }

    #[test]
    fn bottom_height_parses_valid_value() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "12");
        assert_eq!(bottom_panel_height_from_options(&opts), 12);
    }

    #[test]
    fn bottom_height_trims_whitespace() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "  8  ");
        assert_eq!(bottom_panel_height_from_options(&opts), 8);
    }

    #[test]
    fn bottom_height_zero_hides_panel() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "0");
        assert_eq!(bottom_panel_height_from_options(&opts), 0);
    }

    #[test]
    fn bottom_height_falls_back_on_invalid_value() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "abc");
        assert_eq!(bottom_panel_height_from_options(&opts), BOTTOM_PANEL_HEIGHT);
    }

    #[test]
    fn bottom_height_falls_back_on_empty_value() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "");
        assert_eq!(bottom_panel_height_from_options(&opts), BOTTOM_PANEL_HEIGHT);
    }

    #[test]
    fn bottom_percent_defaults_when_option_missing() {
        let opts = HashMap::new();
        assert_eq!(bottom_percent_from_options(&opts), DEFAULT_BOTTOM_PERCENT);
    }

    #[test]
    fn bottom_percent_parses_valid_values() {
        for (value, expected) in [("30", 30), ("0", 0), ("100", 100)] {
            let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, value);
            assert_eq!(
                bottom_percent_from_options(&opts),
                expected,
                "expected {value} to parse as {expected}"
            );
        }
    }

    #[test]
    fn bottom_percent_falls_back_above_100() {
        for value in ["101", "150", "300"] {
            let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, value);
            assert_eq!(
                bottom_percent_from_options(&opts),
                DEFAULT_BOTTOM_PERCENT,
                "expected {value} to fall back to the default"
            );
        }
    }

    #[test]
    fn bottom_percent_falls_back_on_invalid_value() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "abc");
        assert_eq!(bottom_percent_from_options(&opts), DEFAULT_BOTTOM_PERCENT);
    }

    #[test]
    fn bottom_percent_falls_back_on_empty_value() {
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "");
        assert_eq!(bottom_percent_from_options(&opts), DEFAULT_BOTTOM_PERCENT);
    }

    #[test]
    fn pet_defaults_off_when_option_missing() {
        let opts = HashMap::new();
        assert!(!pet_enabled_from_options(&opts));
    }

    #[test]
    fn pet_enabled_when_on() {
        for value in ["on", "ON", "true", "1", "yes"] {
            let opts = opts_with(tmux::SIDEBAR_PET, value);
            assert!(
                pet_enabled_from_options(&opts),
                "expected {value} to enable"
            );
        }
    }

    #[test]
    fn pet_disabled_when_off() {
        for value in ["off", "false", "0", "no", ""] {
            let opts = opts_with(tmux::SIDEBAR_PET, value);
            assert!(
                !pet_enabled_from_options(&opts),
                "expected {value} to disable"
            );
        }
    }
}
