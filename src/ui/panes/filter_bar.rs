use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::state::{AppState, FilterBarControl, RepoFilter, StatusFilter, filter_bar_layout};
use crate::tmux::PaneStatus;

use crate::ui::text::{display_width, truncate_to_width};

/// Glyphs of the pop-out control on the right edge of the top row: the
/// pop-out glyph while the sidebar is docked, the pop-in glyph while it
/// floats. Which one shows follows the pane's `floating` flag, not a
/// remembered mode.
const POPOUT_GLYPH: &str = "⤢";
const POPIN_GLYPH: &str = "⤡";

/// Glyphs of the width-lock control, one cell left of the pop-out
/// glyph: the ballot box carries its check while the lock is on, and is
/// empty while docked widths are free to drift. Plain single-width text
/// glyphs — the padlock family's emoji render double-width, which the
/// one-cell control region cannot hold (design D8).
const LOCK_GLYPH: &str = "☑";
const UNLOCK_GLYPH: &str = "☐";

/// Render the status filter bar, returning the line and the columns of
/// the two right-edge controls (lock, pop-out) for click hit-testing.
pub(super) fn render_filter_bar<'a>(
    state: &AppState,
    width: u16,
) -> (Line<'a>, Option<FilterBarControl>) {
    let theme = &state.theme;
    let icons = &state.icons;
    let counts = state.status_counts();

    let icon_for = |filter: StatusFilter| -> (String, ratatui::style::Color) {
        match filter {
            StatusFilter::All => (icons.all_icon().to_string(), theme.status_all),
            StatusFilter::Busy => (
                icons.status_icon(&PaneStatus::Running).to_string(),
                theme.status_running,
            ),
            StatusFilter::Waiting => (
                icons.status_icon(&PaneStatus::Waiting).to_string(),
                theme.status_color(&PaneStatus::Waiting, false),
            ),
            StatusFilter::Idle => (
                icons.status_icon(&PaneStatus::Idle).to_string(),
                theme.status_color(&PaneStatus::Idle, false),
            ),
            StatusFilter::Error => (
                icons.status_icon(&PaneStatus::Error).to_string(),
                theme.status_color(&PaneStatus::Error, false),
            ),
        }
    };

    // The chip layout (which chips fit before the control, and where
    // each starts) comes from the same pure function the click hit test
    // runs, so a drawn chip and a clickable chip are always the same
    // chip, whatever the counts are.
    let (chips, control) = filter_bar_layout(counts, width);
    let Some(control) = control else {
        return (Line::from(Vec::new()), None);
    };

    let mut spans: Vec<Span<'a>> = Vec::new();
    // The leading gutter column, skipped when the control region owns
    // the whole row (a one- or two-column row).
    let region_start = control.lock.unwrap_or(control.popout);
    if region_start > 0 {
        spans.push(Span::raw(" "));
    }

    // The busy chip always wears the running colour — running and
    // background panes are both busy, so its icon carries no selection
    // cue. The active chip is instead marked by its count, which is
    // drawn in the accent colour.
    for (i, chip) in chips.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let filter = chip.filter;
        let count = counts.count(filter);
        let (icon, icon_color) = icon_for(filter);

        let is_selected = state.global.status_filter == filter;
        let icon_style = if is_selected || filter == StatusFilter::Busy {
            Style::default().fg(icon_color)
        } else {
            Style::default().fg(theme.filter_inactive)
        };
        spans.push(Span::styled(icon, icon_style));

        let count_str = format!("{count}");
        let count_style = if is_selected {
            Style::default().fg(theme.accent)
        } else if count == 0 {
            Style::default().fg(theme.filter_inactive)
        } else {
            Style::default().fg(theme.text_active)
        };
        spans.push(Span::styled(count_str, count_style));
    }

    // The two controls sit one column in from the right border — the
    // lock glyph (its latest state visible at a glance: checked and
    // accent-coloured while locked, empty and muted while free), then
    // the pop-out glyph, then a trailing spacer where the border column
    // would otherwise sit — the way the secondary header right-aligns
    // its repo button.
    let lock_glyph = if state.global.width_lock {
        LOCK_GLYPH
    } else {
        UNLOCK_GLYPH
    };
    let lock_style = if state.global.width_lock {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.filter_inactive)
    };
    let control_glyph = if state.sidebar_pane.floating {
        POPIN_GLYPH
    } else {
        POPOUT_GLYPH
    };
    // Everything drawn ends at or before the control region: the last
    // drawn chip, or the leading gutter when every chip was dropped.
    let drawn_end = chips
        .last()
        .map_or(region_start.min(1), |chip| chip.start + chip.width);
    spans.push(Span::raw(" ".repeat((region_start - drawn_end) as usize)));
    if control.lock.is_some() {
        spans.push(Span::styled(lock_glyph, lock_style));
    }
    spans.push(Span::styled(
        control_glyph,
        Style::default().fg(theme.status_running),
    ));
    // The spacer after the pop-out glyph only fits on a row three
    // columns wide or more; a narrower row draws the glyphs alone.
    if control.popout + 1 < width {
        spans.push(Span::raw(" "));
    }

    (Line::from(spans), Some(control))
}

pub(super) fn render_secondary_header<'a>(
    state: &AppState,
    width: u16,
) -> (Line<'a>, Option<u16>, Option<u16>) {
    let theme = &state.theme;
    let repo_icon = "▾";

    let repo_has_filter = !matches!(state.global.repo_filter, RepoFilter::All);
    let repo_style = if state.is_repo_popup_open() || repo_has_filter {
        Style::default().fg(theme.text_active)
    } else {
        Style::default().fg(theme.text_muted)
    };

    let has_notices_info = crate::ui::notices::has_info(state);
    let notices_button_col = has_notices_info.then_some(0);
    let notices_width = crate::ui::notices::BUTTON_WIDTH;
    let max_repo_label_width = width.saturating_sub((notices_width + 3) as u16) as usize;
    let repo_label = match &state.global.repo_filter {
        RepoFilter::All => "—".to_string(),
        RepoFilter::Repo(name) => truncate_to_width(name, max_repo_label_width),
    };
    let repo_btn_width = display_width(&repo_label) + 2; // label + space + arrow

    let gap = (width as usize).saturating_sub(repo_btn_width + notices_width);
    let repo_button_col = Some((notices_width + gap) as u16);

    let mut spans: Vec<Span<'a>> = Vec::new();
    if has_notices_info {
        spans.push(crate::ui::notices::button_span(state));
        spans.push(Span::raw(" "));
    } else {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::raw(" ".repeat(gap)));
    spans.push(Span::styled(repo_label, repo_style));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(repo_icon, repo_style));

    (Line::from(spans), notices_button_col, repo_button_col)
}

#[cfg(test)]
use crate::group::PaneGitInfo;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn snapshot_secondary_header_omits_version_banner_when_notice_present() {
        // Version notices light up the `ⓘ` indicator in the header but
        // must not leak the "new release vX.Y.Z" banner into the row —
        // the banner lives in the popup, not the header. A snapshot here
        // catches any regression that would put banner text back on the
        // row, including subtle width or spacing drift.
        let mut state = crate::state::AppState::new(String::new());
        state.version_notice = Some(crate::version::UpdateNotice {
            local_version: "0.2.6".into(),
            latest_version: "0.2.7".into(),
        });

        let text = line_text(&render_secondary_header(&state, 30).0);
        insta::assert_snapshot!(text, @"ⓘ                          — ▾");
    }

    #[test]
    fn render_secondary_header_keeps_repo_position_with_or_without_notices_info() {
        let mut with_info = AppState::new(String::new());
        with_info.version_notice = Some(crate::version::UpdateNotice {
            local_version: "0.2.6".into(),
            latest_version: "0.2.7".into(),
        });
        with_info.notices.missing_hook_groups = vec![crate::state::NoticesMissingHookGroup {
            agent: "claude".into(),
            hooks: vec!["SessionStart".into()],
        }];

        let without_info = AppState::new(String::new());

        let (_, _, with_repo_col) = render_secondary_header(&with_info, 30);
        let (_, _, without_repo_col) = render_secondary_header(&without_info, 30);

        assert_eq!(with_repo_col, without_repo_col);
        assert_eq!(with_repo_col, Some(27));
    }

    #[test]
    fn snapshot_secondary_header_without_notices_info() {
        let state = AppState::new(String::new());
        let text = line_text(&render_secondary_header(&state, 30).0);
        insta::assert_snapshot!(text, @"                           — ▾");
    }

    #[test]
    fn snapshot_secondary_header_with_version_only() {
        let mut state = AppState::new(String::new());
        state.version_notice = Some(crate::version::UpdateNotice {
            local_version: "0.2.6".into(),
            latest_version: "0.2.7".into(),
        });
        let text = line_text(&render_secondary_header(&state, 30).0);
        insta::assert_snapshot!(text, @"ⓘ                          — ▾");
    }

    #[test]
    fn snapshot_secondary_header_with_hooks_only() {
        let mut state = AppState::new(String::new());
        state.notices.missing_hook_groups = vec![crate::state::NoticesMissingHookGroup {
            agent: "claude".into(),
            hooks: vec!["SessionStart".into()],
        }];
        let text = line_text(&render_secondary_header(&state, 30).0);
        insta::assert_snapshot!(text, @"ⓘ                          — ▾");
    }

    #[test]
    fn snapshot_secondary_header_with_version_and_hooks() {
        let mut state = AppState::new(String::new());
        state.version_notice = Some(crate::version::UpdateNotice {
            local_version: "0.2.6".into(),
            latest_version: "0.2.7".into(),
        });
        state.notices.missing_hook_groups = vec![crate::state::NoticesMissingHookGroup {
            agent: "claude".into(),
            hooks: vec!["SessionStart".into()],
        }];
        let text = line_text(&render_secondary_header(&state, 30).0);
        insta::assert_snapshot!(text, @"ⓘ                          — ▾");
    }

    // ─── render_filter_bar tests ──────────────────────────────

    fn make_state_with_groups(groups: Vec<crate::group::RepoGroup>) -> AppState {
        let mut state = AppState::new("%99".into());
        state.repo_groups = groups;
        state.rebuild_row_targets();
        state
    }

    /// Render the filter bar at a fixed test width and collect its text.
    fn filter_bar_text_at(state: &AppState, width: u16) -> String {
        let (line, _) = render_filter_bar(state, width);
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn filter_bar_text(state: &AppState) -> String {
        filter_bar_text_at(state, 20)
    }

    #[test]
    fn render_filter_bar_is_status_only() {
        let state = make_state_with_groups(vec![]);
        let text = filter_bar_text(&state);
        assert!(
            !text.contains("▾"),
            "status filter bar should not contain repo button"
        );
    }

    #[test]
    fn render_filter_bar_uses_selected_and_inactive_icon_colors() {
        let pane1 = crate::tmux::PaneInfo {
            pane_id: "%2".into(),
            pane_active: true,
            status: PaneStatus::Running,
            attention: false,
            agent: crate::tmux::AgentType::Claude,
            path: String::new(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: crate::tmux::PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: crate::tmux::WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        };
        let pane2 = crate::tmux::PaneInfo {
            pane_id: "%3".into(),
            pane_active: false,
            status: PaneStatus::Idle,
            attention: false,
            agent: crate::tmux::AgentType::Codex,
            path: String::new(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: crate::tmux::PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: crate::tmux::WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        };
        let mut state = make_state_with_groups(vec![crate::group::RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (pane1, PaneGitInfo::default()),
                (pane2, PaneGitInfo::default()),
            ],
        }]);
        state.global.status_filter = StatusFilter::Busy;
        let theme = &state.theme;

        let (line, _) = render_filter_bar(&state, 20);
        let cells: Vec<_> = line
            .spans
            .iter()
            .filter(|span| !span.content.as_ref().trim().is_empty())
            .collect();

        assert_eq!(cells.len(), 12);

        assert_eq!(cells[0].content.as_ref(), "≡");
        assert_eq!(cells[0].style.fg, Some(theme.filter_inactive));
        assert!(!cells[0].style.add_modifier.contains(Modifier::UNDERLINED));

        assert_eq!(cells[1].content.as_ref(), "2");
        assert_eq!(cells[1].style.fg, Some(theme.text_active));

        // The busy chip icon wears the running colour even while busy is
        // the active category, and its count carries the accent colour.
        assert_eq!(cells[2].content.as_ref(), "●");
        assert_eq!(cells[2].style.fg, Some(theme.status_running));
        assert!(!cells[2].style.add_modifier.contains(Modifier::UNDERLINED));

        assert_eq!(cells[3].content.as_ref(), "1");
        assert_eq!(cells[3].style.fg, Some(theme.accent));

        assert_eq!(cells[4].content.as_ref(), "◐");
        assert_eq!(cells[4].style.fg, Some(theme.filter_inactive));

        assert_eq!(cells[5].content.as_ref(), "0");
        assert_eq!(cells[5].style.fg, Some(theme.filter_inactive));

        assert_eq!(cells[6].content.as_ref(), "○");
        assert_eq!(cells[6].style.fg, Some(theme.filter_inactive));

        assert_eq!(cells[7].content.as_ref(), "1");
        assert_eq!(cells[7].style.fg, Some(theme.text_active));

        assert_eq!(cells[8].content.as_ref(), "✕");
        assert_eq!(cells[8].style.fg, Some(theme.filter_inactive));

        assert_eq!(cells[9].content.as_ref(), "0");
        assert_eq!(cells[9].style.fg, Some(theme.filter_inactive));
    }

    #[test]
    fn render_filter_bar_busy_icon_keeps_running_colour_when_inactive() {
        let mut pane = test_filter_pane("%2", PaneStatus::Idle);
        pane.agent = crate::tmux::AgentType::Codex;
        let mut state = make_state_with_groups(vec![crate::group::RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }]);
        state.global.status_filter = StatusFilter::Idle;
        let theme = &state.theme;

        let (line, _) = render_filter_bar(&state, 20);
        let cells: Vec<_> = line
            .spans
            .iter()
            .filter(|span| !span.content.as_ref().trim().is_empty())
            .collect();

        // Busy is not active: its icon still wears the running colour,
        // its count drops back to the ordinary text colour, and the
        // active chip's count wears the accent colour.
        assert_eq!(cells[2].content.as_ref(), "●");
        assert_eq!(cells[2].style.fg, Some(theme.status_running));
        assert_eq!(cells[3].content.as_ref(), "0");
        assert_eq!(cells[3].style.fg, Some(theme.filter_inactive));

        assert_eq!(cells[6].content.as_ref(), "○");
        assert_eq!(cells[6].style.fg, Some(theme.status_idle));
        assert_eq!(cells[7].content.as_ref(), "1");
        assert_eq!(cells[7].style.fg, Some(theme.accent));
    }

    #[test]
    fn render_filter_bar_busy_count_sums_running_and_background() {
        let mut state = make_state_with_groups(vec![crate::group::RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (
                    test_filter_pane("%1", PaneStatus::Running),
                    PaneGitInfo::default(),
                ),
                (
                    test_filter_pane("%2", PaneStatus::Background),
                    PaneGitInfo::default(),
                ),
            ],
        }]);
        state.global.status_filter = StatusFilter::Busy;

        let text = filter_bar_text(&state);
        insta::assert_snapshot!(text, @" ≡2 ●2 ◐0 ○0 ✕0  ☐⤢ ");
    }

    // ─── the pop-out control ─────────────────────────────────

    #[test]
    fn snapshot_filter_bar_docked_shows_the_pop_out_glyph() {
        // The zero-count docked row at the 20 column minimum: the
        // snapshot pins the whole row, the renderer returns the control
        // columns and a full-width row at every width, the pop-out
        // glyph wears the running colour, and the unlocked lock glyph
        // stays muted. Direct equality checks sit alongside the
        // snapshot because layout columns and colours are not visible
        // characters.
        let state = make_state_with_groups(vec![]);
        assert!(!state.sidebar_pane.floating);
        for width in [20u16, 30, 40] {
            let (line, control) = render_filter_bar(&state, width);
            let text = line_text(&line);
            let Some(control) = control else {
                panic!("the renderer must return the control columns at width {width}")
            };
            assert_eq!(
                control.popout,
                width - 2,
                "pop-out column one space off the right edge at width {width}"
            );
            assert_eq!(
                control.lock,
                Some(width - 3),
                "lock column one cell left of the pop-out glyph at width {width}"
            );
            assert_eq!(
                display_width(&text),
                width as usize,
                "the row must be as wide as the area at width {width}"
            );
        }
        let (line, _) = render_filter_bar(&state, 20);
        insta::assert_snapshot!(line_text(&line), @" ≡0 ●0 ◐0 ○0 ✕0  ☐⤢ ");
        // The trailing spacer spans the row's last column, so the
        // pop-out glyph is the second-to-last span and the lock glyph
        // the one before it.
        let glyph = &line.spans[line.spans.len() - 2];
        assert_eq!(glyph.content.as_ref(), "⤢");
        assert_eq!(glyph.style.fg, Some(state.theme.status_running));
        let lock = &line.spans[line.spans.len() - 3];
        assert_eq!(lock.content.as_ref(), "☐");
        assert_eq!(lock.style.fg, Some(state.theme.filter_inactive));
    }

    #[test]
    fn snapshot_filter_bar_locked_shows_the_checked_lock_glyph() {
        // Locked: the glyph switches to the checked box and borrows the
        // accent colour the active chip count wears, so the state reads
        // at a glance against the muted unlocked glyph.
        let mut state = make_state_with_groups(vec![]);
        state.global.width_lock = true;
        insta::assert_snapshot!(filter_bar_text(&state), @" ≡0 ●0 ◐0 ○0 ✕0  ☑⤢ ");
        let (line, _) = render_filter_bar(&state, 20);
        let lock = &line.spans[line.spans.len() - 3];
        assert_eq!(lock.content.as_ref(), "☑");
        assert_eq!(lock.style.fg, Some(state.theme.accent));
    }

    #[test]
    fn snapshot_filter_bar_floating_shows_the_pop_in_glyph() {
        // The glyph follows the pane's floating flag, so a sidebar that
        // was floated from the tmux command line shows the pop-in glyph.
        let mut state = make_state_with_groups(vec![]);
        state.sidebar_pane.floating = true;
        let text = filter_bar_text(&state);
        insta::assert_snapshot!(text, @" ≡0 ●0 ◐0 ○0 ✕0  ☐⤡ ");
    }

    // ─── the row with multi-digit counts (the 20 column minimum) ───

    /// Ten panes of each of the four non-all statuses, so every chip
    /// count renders with two digits and the chip run alone fills the
    /// 20 column minimum.
    fn state_with_two_digit_counts() -> AppState {
        let mut panes = Vec::new();
        for (status, n) in [
            (PaneStatus::Running, 10),
            (PaneStatus::Waiting, 10),
            (PaneStatus::Idle, 10),
            (PaneStatus::Error, 10),
        ] {
            for i in 0..n {
                panes.push((
                    test_filter_pane(&format!("%{i}x{status:?}"), status.clone()),
                    PaneGitInfo::default(),
                ));
            }
        }
        make_state_with_groups(vec![crate::group::RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes,
        }])
    }

    #[test]
    fn rendered_icons_sit_exactly_on_the_layout_chip_columns() {
        // The renderer and the hit test share one layout function; this
        // pins the render half: every drawn icon sits on the column the
        // layout names for its chip, at every width including the
        // truncating one.
        let state = state_with_two_digit_counts();
        let counts = state.status_counts();
        for width in [20u16, 24, 40] {
            let (line, _) = render_filter_bar(&state, width);
            let text = line_text(&line);
            let (chips, _) = filter_bar_layout(counts, width);
            let chars: Vec<char> = text.chars().collect();
            for chip in &chips {
                assert_eq!(
                    chars[chip.start as usize],
                    icon_of(&state, chip.filter).chars().next().unwrap(),
                    "icon column mismatch at width {width}: {text}"
                );
            }
            // A dropped chip must leave no drawn trace either.
            if chips.len() < 5 {
                assert_eq!(
                    chars[popout_col(width) as usize],
                    '⤢',
                    "the pop-out control stays one space off the right edge at width {width}"
                );
                assert_eq!(
                    chars[popout_col(width) as usize - 1],
                    '☐',
                    "the lock glyph sits one cell left of the pop-out glyph at width {width}"
                );
            }
        }
    }

    /// The icon glyph the renderer draws for a chip of `filter`, keyed
    /// by filter rather than by position.
    fn icon_of(state: &AppState, filter: StatusFilter) -> String {
        match filter {
            StatusFilter::All => state.icons.all_icon().to_string(),
            StatusFilter::Busy => state.icons.status_icon(&PaneStatus::Running).to_string(),
            StatusFilter::Waiting => state.icons.status_icon(&PaneStatus::Waiting).to_string(),
            StatusFilter::Idle => state.icons.status_icon(&PaneStatus::Idle).to_string(),
            StatusFilter::Error => state.icons.status_icon(&PaneStatus::Error).to_string(),
        }
    }

    fn popout_col(width: u16) -> u16 {
        width - 2
    }

    fn test_filter_pane(pane_id: &str, status: PaneStatus) -> crate::tmux::PaneInfo {
        crate::tmux::PaneInfo {
            pane_id: pane_id.into(),
            pane_active: false,
            status,
            attention: false,
            agent: crate::tmux::AgentType::Claude,
            path: String::new(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: crate::tmux::PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: crate::tmux::WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        }
    }

    #[test]
    fn render_secondary_header_repo_button_col_returned() {
        let state = make_state_with_groups(vec![]);
        let (_, _, col) = render_secondary_header(&state, 28);
        assert_eq!(col, Some(25), "repo button should be right-aligned");
    }

    #[test]
    fn snapshot_secondary_header_shows_notices_indicator_when_missing_hooks_exist() {
        // Visual regression check: the indicator MUST sit at column 0
        // and the repo filter MUST stay pinned to the right edge when
        // missing hooks are present. A snapshot catches any drift in
        // spacing, glyph, or column alignment that a `starts_with` /
        // `contains` probe would silently miss.
        let mut state = make_state_with_groups(vec![]);
        state.notices.missing_hook_groups = vec![crate::state::NoticesMissingHookGroup {
            agent: "claude".into(),
            hooks: vec!["SessionStart".into(), "Stop".into()],
        }];

        let (line, notices_col, repo_col) = render_secondary_header(&state, 28);
        let text = line_text(&line);
        insta::assert_snapshot!(text, @"ⓘ                        — ▾");
        // Click-target columns are layout state, not visible characters,
        // so they stay as direct equality checks alongside the snapshot.
        assert_eq!(notices_col, Some(0));
        assert_eq!(repo_col, Some(25));
    }

    #[test]
    fn render_secondary_header_shows_repo_name_when_filtered() {
        let mut state = make_state_with_groups(vec![crate::group::RepoGroup {
            name: "my-app".into(),
            has_focus: true,
            panes: vec![],
        }]);
        state.global.repo_filter = RepoFilter::Repo("my-app".into());
        let text = line_text(&render_secondary_header(&state, 40).0);
        assert!(
            text.contains("my-app"),
            "secondary header should show filtered repo name, got: {text}"
        );
        assert!(
            text.find("my-app").unwrap() < text.find("▾").unwrap(),
            "repo name should come before the arrow"
        );
        let (line, _, _) = render_secondary_header(&state, 40);
        let repo_span = line
            .spans
            .iter()
            .find(|span| span.content.contains("my-app"))
            .unwrap();
        assert!(
            !repo_span.style.add_modifier.contains(Modifier::BOLD),
            "filtered repo label should not be bold"
        );
    }

    #[test]
    fn render_secondary_header_truncates_long_repo_name() {
        let mut state = make_state_with_groups(vec![crate::group::RepoGroup {
            name: "very-long-repository-name-that-exceeds-width".into(),
            has_focus: true,
            panes: vec![],
        }]);
        state.global.repo_filter =
            RepoFilter::Repo("very-long-repository-name-that-exceeds-width".into());
        let text = line_text(&render_secondary_header(&state, 28).0);
        assert!(
            text.contains('…'),
            "repo name should be truncated with an ellipsis"
        );
        assert!(text.contains("▾"));
        assert!(
            !text.contains("very-long-repository-name-that-exceeds-width"),
            "repo name should not fit in full at this width"
        );
        assert!(
            text.find('…').unwrap() < text.find("▾").unwrap(),
            "repo name should come before the arrow"
        );
    }

    #[test]
    fn render_secondary_header_popup_open_styling() {
        let mut state = make_state_with_groups(vec![]);
        state.popup = crate::state::PopupState::Repo {
            selected: 0,
            area: None,
        };
        let (line, _, _) = render_secondary_header(&state, 28);
        let last_span = line.spans.last().unwrap();
        assert!(
            !last_span.style.add_modifier.contains(Modifier::UNDERLINED),
            "repo button should not be underlined when popup is open"
        );
        assert!(
            !last_span.style.add_modifier.contains(Modifier::BOLD),
            "repo button should not be bold when popup is open"
        );
    }
}
