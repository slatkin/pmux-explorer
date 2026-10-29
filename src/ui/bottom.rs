mod activity;
mod git;
mod tasks;
mod thought;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::state::{AppState, BottomTab, BottomTabTarget, divider_tab_rows};

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

/// The rule row that tops the bottom panel: the tab run centered on
/// the row and space fill on both sides — a solid band [`draw_bottom`]
/// paints in `filter_bar_bg`. No left, right or bottom border
/// character exists around it. The run centers by padding
/// `floor((width - run) / 2)` columns on the left and the remainder on
/// the right, so the row stays exactly `width` long; a run wider than
/// `width` gets no pad and each title span truncates against the
/// remaining budget, so the row never overflows.
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
    let run_width: usize = title_spans
        .iter()
        .map(|title| display_width(&title.span.content))
        .sum();
    let leading = (width as usize).saturating_sub(run_width) / 2;
    let mut remaining = (width as usize).saturating_sub(leading);
    let mut spans = vec![Span::styled(" ".repeat(leading), rule_style)];
    let mut col = leading as u16; // the first span starts after the pad
    let mut targets = Vec::new();
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
    spans.push(Span::styled(" ".repeat(remaining), rule_style));
    (Line::from(spans), targets)
}

/// The column bottom panel: the rule row on top, the tab content under
/// it. The rule is the column's tab bar; a strip keeps its tabs on the
/// divider instead and calls [`draw_bottom_content`] with the tab
/// panel's `Rect` directly.
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

    draw_bottom_content(frame, state, content_rect);
}

/// The strip's vertical tab divider: the 2-column band in the filter
/// bar's background colour — the same band colour the column's rule
/// wears, rendered after the focus fill so it wins over it — with each
/// shown tab glyph on its own row. The run centers vertically and a
/// run taller than the strip is cut at the bottom ([`divider_tab_rows`],
/// the same layout the strip tab click test reads); the displayed tab
/// keeps the bolded accent the rule gives it. The glyph rows are
/// published as `FrameLayout::bottom_tab_targets` so a click
/// hit-tests the rows that were drawn.
pub fn draw_strip_divider(frame: &mut Frame, state: &mut AppState, area: Rect) {
    frame.render_widget(
        Block::default().style(Style::default().bg(state.theme.filter_bar_bg)),
        area,
    );
    let targets = divider_tab_rows(&state.shown_bottom_tabs(), area.height);
    state.layout.bottom_tab_targets = targets;
    let titles = build_tab_title(state);
    for target in &state.layout.bottom_tab_targets {
        let Some(title) = titles.iter().find(|title| title.tab == Some(target.tab)) else {
            continue;
        };
        let row = Rect::new(area.x, area.y + target.start, area.width, 1);
        frame.render_widget(Paragraph::new(Line::from(title.span.clone())), row);
    }
}

/// The tab content the displayed tab selects, drawn over the whole
/// given `Rect` with no rule row of its own. Both layouts call it: the
/// column with the rows under its rule, a strip with the right-hand
/// tab panel.
pub fn draw_bottom_content(frame: &mut Frame, state: &mut AppState, area: Rect) {
    match state.displayed_bottom_tab() {
        BottomTab::Activity => activity::draw_activity_content(frame, state, area),
        BottomTab::Tasks => tasks::draw_tasks_content(frame, state, area),
        BottomTab::GitStatus => git::draw_git_content(frame, state, area),
        BottomTab::Thought => thought::draw_thought_content(frame, state, area),
    }
}

/// The tab run drawn on the rule: one glyph span per shown tab, a
/// space-delimiter-space separator span between glyphs, styled
/// active/inactive against the displayed tab. The spaces matter: the
/// Nerd Font glyphs can render two cells wide, and a bare `│` would
/// sit under the glyph's second cell. The shown tabs come from [`AppState::shown_bottom_tabs`]
/// and the glyph strings from [`BottomTab::label`]; each glyph span
/// carries its tab so `top_line_border` can hand the drawn glyph
/// columns to the click hitboxes — the two describe one layout.
pub(crate) fn build_tab_title(state: &AppState) -> Vec<TitleSpan> {
    let theme = &state.theme;
    // Both label styles sit on the filter bar's band, which is lighter
    // than the terminal background: the inactive labels wear the bright
    // text colour (text_muted washes out on the band), and the active
    // tab keeps the accent but bolded, so it still reads on the band.
    let active = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let inactive = Style::default().fg(theme.text_active);
    let displayed = state.displayed_bottom_tab();
    let is = |tab: &BottomTab| {
        if displayed == *tab { active } else { inactive }
    };
    // The delimiters wear the tab separator colour (#9aa5ce) — a
    // quiet grey that stays distinct from both the bold accent of the
    // active glyph and the bright text of the inactive ones.
    let sep_style = Style::default().fg(theme.tab_separator);

    let mut spans: Vec<TitleSpan> = Vec::new();
    for (i, tab) in state.shown_bottom_tabs().into_iter().enumerate() {
        if i > 0 {
            // " │ " — a free column on each side of the delimiter so a
            // two-cell glyph never bleeds onto it.
            spans.push(TitleSpan {
                span: Span::styled(" \u{2502} ", sep_style),
                tab: None,
            });
        }
        spans.push(TitleSpan {
            // One glyph per tab (see `BottomTab::label`); the run is
            // centered on the rule by `top_line_border`.
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

/// Publish the strip divider's tab hitboxes the way `draw_strip_divider`
/// does: run [`divider_tab_rows`] over the state's shown tabs at
/// `height` rows and store the drawn rows. Tests use it instead of a
/// rendered frame so the divider's unmoved-release click hit-tests the
/// drawn rows, not a re-implemented walk.
#[cfg(test)]
pub(crate) fn draw_divider_tab_rows(state: &mut AppState, height: u16) {
    state.layout.bottom_tab_targets = divider_tab_rows(&state.shown_bottom_tabs(), height);
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

        // A pi pane shows the Thought glyph second; the displayed tab
        // (Thought here) is styled with the bolded accent, the rest in
        // the bright text colour.
        let mut pi = state_with_focused_agent(AgentType::Pi);
        pi.global.bottom_tab = BottomTab::Thought;
        let (rule_line, _) = top_line_border(&build_tab_title(&pi), rule, 40);
        insta::assert_snapshot!(rule_text(rule_line), @"              │ 󰟶 │  │ 󰊢");
        let title = build_tab_title(&pi);
        let thought = title
            .iter()
            .find(|t| t.span.content == "\u{f07f6}")
            .unwrap();
        assert_eq!(thought.span.style.fg, Some(pi.theme.accent));
        assert!(
            thought
                .span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        );
        let tools = title.iter().find(|t| t.span.content == "\u{f0ad}").unwrap();
        assert_eq!(tools.span.style.fg, Some(pi.theme.text_active));

        // A non-pi pane hides the Thought glyph; the displayed tab is
        // the Activity fallback. The run centers at every width.
        let claude = state_with_focused_agent(AgentType::Claude);
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 30);
        insta::assert_snapshot!(rule_text(rule_line), @"           │  │ 󰊢");

        // Every width keeps the rule exactly the width: the four-glyph
        // pi run needs 13 columns, and at 6 the overflowing run loses
        // its pad, truncates the tail (Tasks and Git draw nothing)
        // and stays 6 wide.
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 22);
        insta::assert_snapshot!(rule_text(rule_line), @"       │  │ 󰊢");
        let (rule_line, _) = top_line_border(&build_tab_title(&pi), rule, 20);
        insta::assert_snapshot!(rule_text(rule_line), @"    │ 󰟶 │  │ 󰊢");
        let (rule_line, _) = top_line_border(&build_tab_title(&claude), rule, 18);
        insta::assert_snapshot!(rule_text(rule_line), @"     │  │ 󰊢");
        let (rule_line, _) = top_line_border(&build_tab_title(&pi), rule, 6);
        insta::assert_snapshot!(rule_text(rule_line), @" │ 󰟶…");
    }

    #[test]
    fn the_drawn_label_columns_are_the_click_targets() {
        let rule = Style::default().fg(Color::Indexed(240));
        let pi = state_with_focused_agent(AgentType::Pi);

        // At a wide width the glyphs sit centered on their full
        // columns; the run's floor-half pad puts the first glyph at
        // column 13 of 40 and column 3 of 20, and each " │ "
        // separator costs three. Each glyph is one column, so a click
        // can only ever land on drawn text.
        let (_, wide) = top_line_border(&build_tab_title(&pi), rule, 40);
        let ranges: Vec<(u16, u16, BottomTab)> =
            wide.iter().map(|t| (t.start, t.end, t.tab)).collect();
        assert_eq!(
            ranges,
            vec![
                (13, 14, BottomTab::Activity),
                (17, 18, BottomTab::Thought),
                (21, 22, BottomTab::Tasks),
                (25, 26, BottomTab::GitStatus),
            ]
        );

        let (_, narrow) = top_line_border(&build_tab_title(&pi), rule, 20);
        let ranges: Vec<(u16, u16, BottomTab)> =
            narrow.iter().map(|t| (t.start, t.end, t.tab)).collect();
        assert_eq!(
            ranges,
            vec![
                (3, 4, BottomTab::Activity),
                (7, 8, BottomTab::Thought),
                (11, 12, BottomTab::Tasks),
                (15, 16, BottomTab::GitStatus),
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
