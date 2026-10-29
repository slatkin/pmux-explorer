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
    style::Style,
    widgets::Block,
};

use crate::state::{AppState, Focus, LayoutKind, clamp_bottom_height, split_rows, strip_layout};
use crate::tmux;
use ratatui::layout::Rect;

/// The bottom panel height used when `@sidebar_bottom_height` is unset,
/// empty, or not an integer from 0 to 100, as a percent of the sidebar
/// height.
pub const DEFAULT_BOTTOM_PERCENT: u8 = 30;

/// Rows reserved between the pane list and the bottom panel when the pet is
/// enabled. The pet and its desk/chair all render inside this band so they
/// never overdraw the pane list above or the bottom panel's border below.
pub const PET_SCENE_HEIGHT: u16 = 5;

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

/// The strip tab-panel width used when `@sidebar_tab_width` is unset,
/// empty, or not an integer from 0 to 100, as a percent of the strip
/// width (divider included).
pub const DEFAULT_TAB_PERCENT: u8 = 40;

/// Read `@sidebar_tab_width` as a percent of the strip width,
/// falling back to [`DEFAULT_TAB_PERCENT`]. The same parse rules as
/// [`bottom_percent_from_options`]: an integer from 0 to 100 is the
/// percent — an unset option, an empty value, or any other value gives
/// the default. A percent of 0 hides the strip's divider and tab panel.
pub fn tab_percent_from_options(opts: &HashMap<String, String>) -> u8 {
    opts.get(tmux::SIDEBAR_TAB_WIDTH)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u8>().ok())
        .filter(|p| *p <= 100)
        .unwrap_or(DEFAULT_TAB_PERCENT)
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
    match state.layout_kind {
        LayoutKind::Strip => draw_strip(frame, state, area),
        LayoutKind::Column => draw_column(frame, state, area),
    }
}

/// The column layout: the agent part on top, the bottom panel (rule +
/// tab content) under it, the pet band between them when the pet is
/// on. The sidebar's original shape; the strip's twin is [`draw_strip`].
fn draw_column(frame: &mut Frame, state: &mut AppState, area: Rect) {
    // While a split drag has moved, the frame draws the drag's rows —
    // the rule follows the mouse row and no percent value is adopted
    // during the drag; the release path, not a tick, writes the
    // percent. Before any move the shared percent's geometry stands.
    let bot_h = match &state.split_drag {
        Some(drag) if drag.moved => drag.size,
        _ => split_rows(state.global.bottom_percent, area.height),
    };
    let bot_h = if state.sidebar_pane.floating {
        // A floating panel keeps the agent list readable: the Activity/Git
        // bottom panel takes at most a third of the panel height. The
        // floating limit lands after the split minimums, so a short
        // floating panel can draw below 3 rows.
        clamp_bottom_height(bot_h, area.height)
    } else {
        bot_h
    };
    // Publish the height this frame actually drew so the mouse handlers
    // hit-test the drawn geometry (`FrameLayout::bottom_panel_height`),
    // never the configured percent. A hidden panel draws no rule, so
    // its tab labels are not clickable either.
    state.layout.bottom_panel_height = bot_h;
    if bot_h == 0 {
        state.layout.bottom_tab_targets.clear();
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if bot_h > 0 {
            vec![Constraint::Min(1), Constraint::Length(bot_h)]
        } else {
            vec![Constraint::Min(1)]
        })
        .split(area);

    // The two sections wear their fills at all times — the sidebar has
    // no focus/unfocus look: the keyboard-owning section gets the
    // active fill and the other gets the base fill: ActivityLog focus
    // owns the bottom panel, Filter/Panes focus own the agent list.
    // With the bottom panel hidden the top is the only place input can
    // land, so it takes the active fill regardless of focus. Ratatui
    // styles patch: later spans keep their own `bg` (selection rows,
    // popups), and cells nothing draws on keep the fill.
    let theme = &state.theme;
    let top_owns_keyboard = bot_h == 0 || state.focus_state.focus != Focus::ActivityLog;
    let (top_bg, bottom_bg) = if top_owns_keyboard {
        (theme.focus_active_bg, theme.focus_bg)
    } else {
        (theme.focus_bg, theme.focus_active_bg)
    };
    frame.render_widget(
        Block::default().style(Style::default().bg(top_bg)),
        chunks[0],
    );
    if let Some(bottom) = chunks.get(1) {
        frame.render_widget(
            Block::default().style(Style::default().bg(bottom_bg)),
            *bottom,
        );
    }

    // When the pet is on, its band is taken from the bottom of the agent
    // part, directly above the rule — only when that still leaves the
    // band plus at least 3 agent rows. Otherwise the band does not show.
    let (agent_area, pet_area) =
        if state.pet_enabled && bot_h > 0 && chunks[0].height >= PET_SCENE_HEIGHT + 3 {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(PET_SCENE_HEIGHT)])
                .split(chunks[0]);
            (split[0], Some(split[1]))
        } else {
            (chunks[0], None)
        };

    panes::draw_agents(frame, state, agent_area, true);

    if bot_h > 0 && chunks.len() > 1 {
        bottom::draw_bottom(frame, state, chunks[1]);
        if let Some(pet_area) = pet_area {
            let running_count = state.running_count();
            pet::draw_pet(frame, state, pet_area, running_count);
        }
    }
}

/// The strip layout: two panels side by side — the vertical filter bar
/// and the agent panel on the left (the agent section), the tab
/// divider and the tab panel on the right (the tab section). No pet:
/// a strip shows none (the `strip-layout` capability). The drawn
/// geometry goes into `FrameLayout::strip` so the mouse handlers
/// hit-test the panels the frame actually drew.
fn draw_strip(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let strip = strip_layout(area.width, area.height, state.global.tab_percent);
    state.layout.strip = Some(strip);
    // The strip has no bottom panel and no horizontal filter row:
    // clear the column geometry a previous frame may have left, so a
    // column-shaped hit test can never fire on strip-drawn state.
    state.layout.bottom_panel_height = 0;
    state.layout.filter_bar_width = 0;
    if strip.tabs.is_none() {
        state.layout.bottom_tab_targets.clear();
    }

    // The two section fills at all times — the keyboard-owning section
    // gets the active fill, the other the base one: Filter/Panes focus
    // owns the agent section on the left, ActivityLog focus the tab
    // section on the right. Ratatui styles patch: later spans keep
    // their own `bg` (selection rows, popups), and cells nothing draws
    // on keep the fill.
    let theme = &state.theme;
    let (agent_bg, tab_bg) = if state.focus_state.focus != Focus::ActivityLog {
        (theme.focus_active_bg, theme.focus_bg)
    } else {
        (theme.focus_bg, theme.focus_active_bg)
    };
    frame.render_widget(
        Block::default().style(Style::default().bg(agent_bg)),
        strip.filter,
    );
    frame.render_widget(
        Block::default().style(Style::default().bg(agent_bg)),
        strip.agents,
    );
    if let Some(divider) = strip.divider {
        frame.render_widget(Block::default().style(Style::default().bg(tab_bg)), divider);
    }
    if let Some(tabs) = strip.tabs {
        frame.render_widget(Block::default().style(Style::default().bg(tab_bg)), tabs);
    }

    panes::draw_strip_filter_bar(frame, state, strip.filter);
    panes::draw_agents(frame, state, strip.agents, false);
    if let (Some(divider), Some(tabs)) = (strip.divider, strip.tabs) {
        bottom::draw_strip_divider(frame, state, divider);
        bottom::draw_bottom_content(frame, state, tabs);
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
    fn bottom_percent_defaults_when_option_missing() {
        let opts = HashMap::new();
        assert_eq!(bottom_percent_from_options(&opts), DEFAULT_BOTTOM_PERCENT);
    }

    #[test]
    fn the_conf_seeds_the_same_bottom_percent_the_binary_falls_back_to() {
        // One number, two homes: the conf seeds `@sidebar_bottom_height`
        // for tmux-side visibility and [`DEFAULT_BOTTOM_PERCENT`] is the
        // fallback when the option is unset or invalid. A test pins
        // them together so a change that updates one copy and not the
        // other fails here instead of drifting in a user's session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_bottom_height "))
            .and_then(|(_, rest)| rest.trim().trim_matches('\'').parse::<u8>().ok());
        assert_eq!(
            seeded,
            Some(DEFAULT_BOTTOM_PERCENT),
            "the conf's seeded bottom height must match DEFAULT_BOTTOM_PERCENT"
        );
    }

    #[test]
    fn the_conf_seeds_the_same_tab_percent_the_binary_falls_back_to() {
        // One number, two homes: the conf seeds `@sidebar_tab_width`
        // for tmux-side visibility and [`DEFAULT_TAB_PERCENT`] is the
        // fallback when the option is unset or invalid. A test pins
        // them together so a change that updates one copy and not the
        // other fails here instead of drifting in a user's session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_tab_width "))
            .and_then(|(_, rest)| rest.trim().trim_matches('\'').parse::<u8>().ok());
        assert_eq!(
            seeded,
            Some(DEFAULT_TAB_PERCENT),
            "the conf's seeded tab width must match DEFAULT_TAB_PERCENT"
        );
    }

    #[test]
    fn the_conf_seeds_the_same_pet_default_the_binary_falls_back_to() {
        // One switch, two homes: the conf seeds `@sidebar_pet` for
        // tmux-side visibility and the binary falls back to `off`
        // (false) when the option is unset or invalid. A test pins
        // them together so a change that updates one copy and not the
        // other fails here instead of drifting in a user's session.
        let conf = include_str!("../../pmux-explorer.conf");
        let seeded = conf
            .lines()
            .find_map(|line| line.split_once("set -g @sidebar_pet "))
            .map(|(_, rest)| {
                pet_enabled_from_options(&opts_with(
                    tmux::SIDEBAR_PET,
                    rest.trim().trim_matches('\''),
                ))
            });
        assert_eq!(
            seeded,
            Some(pet_enabled_from_options(&HashMap::new())),
            "the conf's seeded pet value must match the unset fallback"
        );
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
        // Whitespace around the number is tolerated (the trim the old
        // line-count parse kept): a padded value parses to its number,
        // not to the default.
        let opts = opts_with(tmux::SIDEBAR_BOTTOM_HEIGHT, "  8  ");
        assert_eq!(
            bottom_percent_from_options(&opts),
            8,
            "whitespace-padded \"8\" must parse as 8"
        );
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
    fn tab_percent_parses_valid_values_and_defaults_otherwise() {
        for (value, expected) in [("40", 40), ("0", 0), ("100", 100)] {
            let opts = opts_with(tmux::SIDEBAR_TAB_WIDTH, value);
            assert_eq!(
                tab_percent_from_options(&opts),
                expected,
                "expected {value} to parse as {expected}"
            );
        }
        // Whitespace around the number is tolerated, like the bottom
        // percent's parse.
        let opts = opts_with(tmux::SIDEBAR_TAB_WIDTH, "  55  ");
        assert_eq!(tab_percent_from_options(&opts), 55);
        // An unset, empty, non-numeric or out-of-range value gives the
        // default.
        for value in [None, Some(""), Some("abc"), Some("101")] {
            let opts = value
                .map(|v| opts_with(tmux::SIDEBAR_TAB_WIDTH, v))
                .unwrap_or_default();
            assert_eq!(
                tab_percent_from_options(&opts),
                DEFAULT_TAB_PERCENT,
                "expected {value:?} to fall back to the default"
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
