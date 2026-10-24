mod activity;
mod git;
mod tasks;

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

    match state.global.bottom_tab {
        BottomTab::Activity => activity::draw_activity_content(frame, state, content_rect),
        BottomTab::Tasks => tasks::draw_tasks_content(frame, state, content_rect),
        BottomTab::GitStatus => git::draw_git_content(frame, state, content_rect),
    }
}

fn build_tab_title(state: &AppState) -> Line<'static> {
    let theme = &state.theme;
    let active = Style::default().fg(theme.accent);
    let inactive = Style::default().fg(theme.text_muted);
    let is = |tab: &BottomTab| {
        if state.global.bottom_tab == *tab {
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
    fn top_line_border_fits_the_width_and_truncates_the_tab_run() {
        let state = AppState::new("%99".into());
        let title = build_tab_title(&state);
        let rule = Style::default().fg(Color::Indexed(240));
        let text = |line: Line<'static>| -> String {
            line.spans.iter().map(|s| s.content.as_ref()).collect()
        };

        // 30 and 22 leave the full 18-cell tab run plus fill dashes;
        // 20 — the supported minimum — and 18 truncate the tail of the
        // run (Git goes to "G…", then to nothing) but the rule stays
        // exactly the width. The `─ ` prefix is 2 columns, so the label
        // columns never move.
        assert_eq!(
            text(top_line_border(&title.spans, rule, 30)),
            "─ Activity│Tasks│Git ─────────"
        );
        assert_eq!(
            text(top_line_border(&title.spans, rule, 22)),
            "─ Activity│Tasks│Git ─"
        );
        assert_eq!(
            text(top_line_border(&title.spans, rule, 20)),
            "─ Activity│Tasks│G\u{2026} "
        );
        assert_eq!(
            text(top_line_border(&title.spans, rule, 18)),
            "─ Activity│Tasks│ "
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
