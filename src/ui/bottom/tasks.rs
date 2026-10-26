//! The Tasks tab: the focused pane's stored todo list.
//!
//! Items come from [`state.tasks.items`](crate::state::TasksState),
//! which `refresh_pane_tasks` fills from the pane's store file. The
//! tab is read-only — the writing pane is always the agent, through
//! the `task-set` / `task-update` subcommands.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::AppState;
use crate::task_store::TaskItemStatus;
use crate::ui::text::wrap_text;

pub(super) fn draw_tasks_content(frame: &mut Frame, state: &mut AppState, inner: Rect) {
    let theme = &state.theme;

    // One column of margin on both sides so rows don't sit right on
    // the panel edges.
    let padded = Rect {
        x: inner.x.saturating_add(1),
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };

    if state.tasks.items.is_empty() {
        super::render_centered(frame, padded, "No tasks recorded", theme.text_muted);
        return;
    }

    // Leave one blank row above the first item for breathing room,
    // inside the scroll content like the Activity tab's spacer.
    let mut lines: Vec<Line<'_>> = vec![Line::from("")];

    // The icon plus its separating space leave the rest for the subject.
    let subject_w = (padded.width as usize).saturating_sub(2);

    for item in &state.tasks.items {
        let style = Style::default().fg(status_color(item.status, theme));
        let icon = status_icon(item.status);

        // Subjects hold free text that may contain newlines; every source
        // line wraps on its own so the full text is always shown.
        let mut item_lines: Vec<String> = Vec::new();
        for segment in item.subject.split('\n') {
            let wrapped = wrap_text(segment, subject_w, usize::MAX);
            if wrapped.is_empty() {
                // A blank source line still takes a row.
                item_lines.push(String::new());
            } else {
                item_lines.extend(wrapped);
            }
        }

        for (i, text) in item_lines.iter().enumerate() {
            // Continuation rows indent under the subject, not the icon.
            let prefix = if i == 0 {
                format!("{icon} ")
            } else {
                "  ".to_string()
            };
            lines.push(Line::from(Span::styled(format!("{prefix}{text}"), style)));
        }
    }

    state.tasks.scroll.total_lines = lines.len();
    state.tasks.scroll.visible_height = inner.height as usize;
    // Clamp `offset` to the new total/visible. When the list shrinks
    // (items trimmed, focus change) the stale offset would otherwise
    // produce an empty or over-scrolled paragraph — same clamp-on-shrink
    // behavior as the Activity tab.
    state.tasks.scroll.scroll(0);

    let scroll_offset = state.tasks.scroll.offset as u16;
    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    frame.render_widget(paragraph, padded);
}

/// Row color for a task status: pending takes the dedicated
/// `task_pending` option, the item in flight keeps `task_progress`
/// (the color the agent list's compact row already uses), and
/// completed items take `task_completed` green.
fn status_color(
    status: TaskItemStatus,
    theme: &crate::ui::colors::ColorTheme,
) -> ratatui::style::Color {
    match status {
        TaskItemStatus::Pending => theme.task_pending,
        TaskItemStatus::InProgress => theme.task_progress,
        TaskItemStatus::Completed => theme.task_completed,
    }
}

fn status_icon(status: TaskItemStatus) -> &'static str {
    match status {
        TaskItemStatus::Pending => "◻",
        TaskItemStatus::InProgress => "◼",
        TaskItemStatus::Completed => "✔",
    }
}
