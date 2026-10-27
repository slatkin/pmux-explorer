mod activity;
mod git;
mod tasks;
mod thought;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::{AppState, BottomTab, Focus};

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

/// The rule row that tops the bottom panel: a `─ ` prefix, the tab run,
/// one separating space, and `─` fill to the right edge. No left, right
/// or bottom border character exists around it. The prefix plus that
/// space need 3 display columns; when `width` is smaller (the floor is
/// the same 20 columns the filter bar treats as its minimum), each title
/// span truncates against a shared displayed-width budget so the rule
/// stays exactly `width` long.
fn top_line_border(title_spans: &[Span<'static>], rule_style: Style, width: u16) -> Line<'static> {
    let fixed = 3usize; // "─ " + the space before the fill
    let mut remaining = (width as usize).saturating_sub(fixed);
    let mut spans = vec![Span::styled("─ ", rule_style)];
    for span in title_spans {
        let text = truncate_to_width(&span.content, remaining);
        remaining = remaining.saturating_sub(display_width(&text));
        spans.push(Span::styled(text, span.style));
    }
    spans.push(Span::styled(
        format!(" {}", "─".repeat(remaining)),
        rule_style,
    ));
    Line::from(spans)
}

pub fn draw_bottom(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;
    let rule_color = if state.focus_state.focus == Focus::ActivityLog {
        theme.accent
    } else {
        theme.border_inactive
    };

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

    let top_line = top_line_border(
        &build_tab_title(state).spans,
        Style::default().fg(rule_color),
        area.width,
    );
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
/// and the label strings from [`BottomTab::label`] — the same sources the
/// click hitboxes are computed from, so the two cannot drift.
fn build_tab_title(state: &AppState) -> Line<'static> {
    let theme = &state.theme;
    let active = Style::default().fg(theme.accent);
    let inactive = Style::default().fg(theme.text_muted);
    let displayed = state.displayed_bottom_tab();
    let is = |tab: &BottomTab| {
        if displayed == *tab { active } else { inactive }
    };
    let sep_style = Style::default().fg(theme.border_inactive);

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, tab) in state.shown_bottom_tabs().into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("\u{2502}", sep_style));
        }
        spans.push(Span::styled(tab.label(), is(&tab)));
    }
    Line::from(spans)
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
        insta::assert_snapshot!(
            rule_text(top_line_border(&build_tab_title(&pi).spans, rule, 40)),
            @"─ Tools│Thought│Tasks│Git ──────────────"
        );
        let title = build_tab_title(&pi);
        let thought = title.spans.iter().find(|s| s.content == "Thought").unwrap();
        assert_eq!(thought.style.fg, Some(pi.theme.accent));
        let tools = title.spans.iter().find(|s| s.content == "Tools").unwrap();
        assert_eq!(tools.style.fg, Some(pi.theme.text_muted));

        // A non-pi pane hides the Thought label; the displayed tab is
        // the Activity fallback.
        let claude = state_with_focused_agent(AgentType::Claude);
        insta::assert_snapshot!(
            rule_text(top_line_border(&build_tab_title(&claude).spans, rule, 30)),
            @"─ Tools│Tasks│Git ────────────"
        );

        // 22 and 18 leave the full 15-cell tab run plus fill dashes;
        // 20 — the supported minimum — truncates the tail of the run
        // (Tasks closes with an ellipsis, Git is gone entirely) but
        // the rule stays exactly the width. The `─ ` prefix is 2
        // columns, so the label columns never move.
        insta::assert_snapshot!(
            rule_text(top_line_border(&build_tab_title(&claude).spans, rule, 22)),
            @"─ Tools│Tasks│Git ────"
        );
        insta::assert_snapshot!(
            rule_text(top_line_border(&build_tab_title(&pi).spans, rule, 20)),
            @"─ Tools│Thought│Ta… "
        );
        insta::assert_snapshot!(
            rule_text(top_line_border(&build_tab_title(&claude).spans, rule, 18)),
            @"─ Tools│Tasks│Git "
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
