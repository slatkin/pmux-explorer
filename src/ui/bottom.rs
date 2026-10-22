mod activity;
mod git;
mod tasks;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

/// Top border row of the bottom panel: the bracket, the tab run, the
/// fill dashes, and the closing bracket. The full title run plus the
/// two bracket pieces needs `title + 4` display columns; when `width`
/// is smaller (the floor is the same 20 columns the filter bar treats
/// as its minimum), each title span truncates against a shared
/// displayed-width budget so the closing `╮` always renders and the
/// row stays exactly `width` long.
fn top_line_border(
    title_spans: &[Span<'static>],
    bracket_style: Style,
    width: u16,
) -> Line<'static> {
    let bracket = 4usize; // "╭ " + " ╮"
    let mut remaining = (width as usize).saturating_sub(bracket);
    let mut spans = vec![Span::styled("╭ ", bracket_style)];
    for span in title_spans {
        let text = truncate_to_width(&span.content, remaining);
        remaining = remaining.saturating_sub(display_width(&text));
        spans.push(Span::styled(text, span.style));
    }
    spans.push(Span::styled(
        format!(" {}╮", "─".repeat(remaining)),
        bracket_style,
    ));
    Line::from(spans)
}

pub fn draw_bottom(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let theme = &state.theme;
    let border_color = if state.focus_state.focus == Focus::ActivityLog {
        theme.accent
    } else {
        theme.border_inactive
    };

    let tab_title = build_tab_title(state);
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let top_line = top_line_border(
        &tab_title.spans,
        Style::default().fg(border_color),
        area.width,
    );
    let top_rect = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(Paragraph::new(top_line), top_rect);

    let bottom_line = Line::from(Span::styled(
        format!("╰{}╯", "─".repeat((area.width as usize).saturating_sub(2))),
        Style::default().fg(border_color),
    ));
    let bottom_rect = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );
    frame.render_widget(Paragraph::new(bottom_line), bottom_rect);

    match state.bottom_tab {
        BottomTab::Activity => activity::draw_activity_content(frame, state, inner),
        BottomTab::Tasks => tasks::draw_tasks_content(frame, state, inner),
        BottomTab::GitStatus => git::draw_git_content(frame, state, inner),
    }
}

fn build_tab_title(state: &AppState) -> Line<'static> {
    let theme = &state.theme;
    let active = Style::default().fg(theme.accent);
    let inactive = Style::default().fg(theme.text_muted);
    let is = |tab: &BottomTab| {
        if state.bottom_tab == *tab {
            active
        } else {
            inactive
        }
    };
    let sep_style = Style::default().fg(theme.border_inactive);

    Line::from(vec![
        Span::styled("Activity", is(&BottomTab::Activity)),
        Span::styled("\u{2502}", sep_style),
        Span::styled("Tasks", is(&BottomTab::Tasks)),
        Span::styled("\u{2502}", sep_style),
        Span::styled("Git", is(&BottomTab::GitStatus)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::text::truncate_to_width;

    #[test]
    fn top_line_border_keeps_both_corners_when_the_title_truncates() {
        let state = AppState::new("%99".into());
        let title = build_tab_title(&state);
        let brackets = Style::default().fg(Color::Indexed(240));
        let text = |line: Line<'static>| -> String {
            line.spans.iter().map(|s| s.content.as_ref()).collect()
        };

        // 30 and 22 leave room for the full 18-cell tab run (22 has no
        // columns left for fill dashes); 20 — the supported minimum — and
        // 18 truncate the tail of the run but still close the corner.
        assert_eq!(
            text(top_line_border(&title.spans, brackets, 30)),
            "╭ Activity│Tasks│Git ────────╮"
        );
        assert_eq!(
            text(top_line_border(&title.spans, brackets, 22)),
            "╭ Activity│Tasks│Git ╮"
        );
        assert_eq!(
            text(top_line_border(&title.spans, brackets, 20)),
            "╭ Activity│Tasks│\u{2026} ╮"
        );
        assert_eq!(
            text(top_line_border(&title.spans, brackets, 18)),
            "╭ Activity│Tasks ╮"
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
