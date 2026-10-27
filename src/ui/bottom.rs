mod activity;
mod git;
mod tasks;
mod thought;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::state::{AppState, BottomTab, BottomTabTarget};

use super::text::{display_width, truncate_to_width};

fn render_centered(frame: &mut Frame, area: Rect, text: &str, color: Color) {
    // Vertically center: pad with empty lines above
    let top_pad = area.height.saturating_sub(1) / 2;
    let mut lines: Vec<Line<'_>> = Vec::new();
    for _ in 0..top_pad {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(text, Style::default().fg(color))));
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// One span of the tab run drawn on the bottom rule: the styled text
/// and, for a label span, the tab it selects (a separator carries
/// `None`).
pub(crate) struct TitleSpan {
    span: Span<'static>,
    tab: Option<BottomTab>,
}

/// The rule row that tops the bottom panel: a two-space prefix, the tab
/// run, one separating space, and space fill to the right edge — a
/// solid band [`draw_bottom`] paints in `filter_bar_bg`. No left, right
/// or bottom border character exists around it. The prefix plus that
/// space need 3 display columns; when `width` is smaller (the floor is
/// the same 20 columns the filter bar treats as its minimum), each title
/// span truncates against a shared displayed-width budget so the rule
/// stays exactly `width` long.
///
/// The walk also produces each label's drawn column range — the click
/// hitboxes are this layout read back, so a label the rule truncates
/// away has a shrunken range or none and can never be clicked where it
/// is not drawn.
pub(crate) fn top_line_border(
    title_spans: &[TitleSpan],
    rule_style: Style,
    width: u16,
) -> (Line<'static>, Vec<BottomTabTarget>) {
    let fixed = 3usize; // the two-space prefix + the space before the fill
    let mut remaining = (width as usize).saturating_sub(fixed);
    let mut spans = vec![Span::styled("  ", rule_style)];
    let mut targets = Vec::new();
    let mut col = 2u16; // the first span starts right after the prefix
    for title in title_spans {
        let text = truncate_to_width(&title.span.content, remaining);
        let cols = display_width(&text);
        remaining = remaining.saturating_sub(cols);
        if let Some(tab) = title.tab {
            targets.push(BottomTabTarget {
                start: col,
                end: col + cols as u16,
                tab,
            });
        }
        col += cols as u16;
        spans.push(Span::styled(text, title.span.style));
    }
    spans.push(Span::styled(" ".repeat(remaining + 1), rule_style));
    (Line::from(spans), targets)
}

pub fn draw_bottom(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;

    // No block: the rule is the panel's first row and the tab renderers
    // take the rows under it with the full sidebar width and no padding,
    // so content starts at column 0 as the agent rows do.
    let rule_rect = Rect::new(area.x, area.y, area.width, 1);
    let content_rect = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    // The tab bar is a solid band in the filter bar's colour, painted
    // at all times — it renders after the focus fill, so it wins over
    // it; the tab spans keep the colour underneath them.
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.filter_bar_bg)),
        rule_rect,
    );

    let (top_line, targets) =
        top_line_border(&build_tab_title(state), Style::default(), area.width);
    // Publish the label columns this frame actually drew so the click
    // handler hit-tests the drawn rule, never a recomputed one.
    state.layout.bottom_tab_targets = targets;
    frame.render_widget(Paragraph::new(top_line), rule_rect);

    match state.displayed_bottom_tab() {
        BottomTab::Activity => activity::draw_activity_content(frame, state, content_rect),
        BottomTab::Tasks => tasks::draw_tasks_content(frame, state, content_rect),
        BottomTab::GitStatus => git::draw_git_content(frame, state, content_rect),
        BottomTab::Thought => thought::draw_thought_content(frame, state, content_rect),
    }
}

/// The tab run drawn on the rule: one label span per shown tab, a
/// separator span between labels, styled active/inactive against the
/// displayed tab. The shown tabs come from [`AppState::shown_bottom_tabs`]
/// and the label strings from [`BottomTab::label`]; each label span
/// carries its tab so `top_line_border` can hand the drawn label
/// columns to the click hitboxes — the two describe one layout.
pub(crate) fn build_tab_title(state: &AppState) -> Vec<TitleSpan> {
    let theme = &state.theme;
    let active = Style::default().fg(theme.accent);
    let inactive = Style::default().fg(theme.text_muted);
    let displayed = state.displayed_bottom_tab();
    let is = |tab: &BottomTab| {
        if displayed == *tab { active } else { inactive }
    };
    let sep_style = Style::default().fg(theme.border_inactive);

    let mut spans: Vec<TitleSpan> = Vec::new();
    for (i, tab) in state.shown_bottom_tabs().into_iter().enumerate() {
        if i > 0 {
            spans.push(TitleSpan {
                span: Span::styled("\u{2502}", sep_style),
                tab: None,
            });
        }
        spans.push(TitleSpan {
            span: Span::styled(tab.label(), is(&tab)),
            tab: Some(tab),
        });
    }
    spans
}

/// Publish the rule's tab hitboxes the way `draw_bottom` does: run the
/// rule's own layout over the state's shown tabs at `width` columns and
/// store the drawn ranges. Tests use it instead of a rendered frame so
/// clicks hit-test the drawn columns, not a re-implemented walk.
#[cfg(test)]
pub(crate) fn draw_tab_targets(state: &mut AppState, width: u16) {
    let (_, targets) = top_line_border(&build_tab_title(state), Style::default(), width);
    state.layout.bottom_tab_targets = targets;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{PaneGitInfo, RepoGroup};
    use crate::tmux::{AgentType, PaneInfo, PaneStatus, PermissionMode, WorktreeMetadata};

    /// A state whose focused pane runs the given agent, the minimal
    /// setup `build_tab_title` reads: the shown tabs come from the
    /// focused pane's agent, the accent from the displayed tab.
    fn state_with_focused_agent(agent: AgentType) -> AppState {
        let pane = PaneInfo {
            pane_id: "%1".into(),
            pane_active: true,
            status: PaneStatus::Running,
            attention: false,
            agent,
            path: "/tmp".into(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        };
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        state.focus_state.focused_pane_id = Some("%1".into());
        state
    }

    fn rule_text(line: Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn top_line_border_fits_the_width_and_truncates_the_tab_run() {
        let rule = Style::default().fg(Color::Indexed(240));

        // A pi pane shows the Thought label second; the displayed tab
        // (Thought here) is styled with the accent, the rest muted.
        let mut pi = state_with_focused_agent(AgentType::Pi);
        pi.global.bottom_tab = BottomTab::Thought;
        let (rule_line, _) = top_line_border(&build_tab_title(&pi), rule, 40);
        insta::assert_snapshot!(rule_text(rule_line), @"  Tools│Thought│Tasks│Git");
        let title = build_tab_title(&pi);
        let thought = title.iter().find(|t| t.span.content == "Thought").unwrap();
        assert_eq!(thought.span.style.fg, Some(pi.theme.accent));
        let tools = title.iter().find(|t| t.span.content == "Tools").unwrap();
        assert_eq!(tools.span.style.fg, Some(pi.theme.text_muted));

        // A non-pi pane hides the Thought label; the displayed tab is
        // the Activity fallback.
        let claude = state_with_focused_agent(AgentType::Claude);
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 30);
        insta::assert_snapshot!(rule_text(rule_line), @"  Tools│Tasks│Git");

        // 22 and 18 leave the full 15-cell tab run plus fill dashes;
        // 20 — the supported minimum — truncates the tail of the run
        // (Tasks closes with an ellipsis, Git is gone entirely) but
        // the rule stays exactly the width. The `─ ` prefix is 2
        // columns, so the label columns never move.
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 22);
        insta::assert_snapshot!(rule_text(rule_line), @"  Tools│Tasks│Git");
        let (rule_line, _) = top_line_border(&build_tab_title(&pi), rule, 20);
        insta::assert_snapshot!(rule_text(rule_line), @"  Tools│Thought│Ta…");
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 18);
        insta::assert_snapshot!(rule_text(rule_line), @"  Tools│Tasks│Git");
    }

    #[test]
    fn the_drawn_label_columns_are_the_click_targets() {
        let rule = Style::default().fg(Color::Indexed(240));
        let pi = state_with_focused_agent(AgentType::Pi);

        // At a wide width the labels sit at their full columns; at 20
        // the truncated run gives Tasks only the columns it draws and
        // Git none, so a click can only ever land on drawn text.
        let (_, wide) = top_line_border(&build_tab_title(&pi), rule, 40);
        let ranges: Vec<(u16, u16, BottomTab)> =
            wide.iter().map(|t| (t.start, t.end, t.tab)).collect();
        assert_eq!(
            ranges,
            vec![
                (2, 7, BottomTab::Activity),
                (8, 15, BottomTab::Thought),
                (16, 21, BottomTab::Tasks),
                (22, 25, BottomTab::GitStatus),
            ]
        );

        let (_, narrow) = top_line_border(&build_tab_title(&pi), rule, 20);
        let ranges: Vec<(u16, u16, BottomTab)> =
            narrow.iter().map(|t| (t.start, t.end, t.tab)).collect();
        assert_eq!(
            ranges,
            vec![
                (2, 7, BottomTab::Activity),
                (8, 15, BottomTab::Thought),
                (16, 19, BottomTab::Tasks),
                (19, 19, BottomTab::GitStatus),
            ]
        );
    }

    #[test]
    fn truncate_to_width_short() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_to_width_exact() {
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    #[test]
    fn truncate_to_width_truncated() {
        let result = truncate_to_width("hello world", 8);
        assert!(result.ends_with('…'));
        assert!(result.len() <= 10); // 7 chars + ellipsis in bytes
    }
}
