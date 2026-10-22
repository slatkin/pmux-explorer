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
use crate::ui::text::truncate_to_width;

pub(super) fn draw_tasks_content(frame: &mut Frame, state: &mut AppState, inner: Rect) {
    let theme = &state.theme;

    if state.tasks.items.is_empty() {
        super::render_centered(frame, inner, "No tasks recorded", theme.text_muted);
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    let inner_w = inner.width as usize;

    // Leave one blank row above the first item for breathing room,
    // inside the scroll content like the Activity tab's spacer.
    lines.push(Line::from(""));

    for item in &state.tasks.items {
        // Subjects hold free text that may contain newlines; the list
        // is one row per item, so only the first line is shown.
        let flat = item.subject.split('\n').next().unwrap_or_default();
        // The icon and its separating space leave the rest for the subject.
        let subject = truncate_to_width(flat, inner_w.saturating_sub(2));
        let style = Style::default().fg(status_color(item.status, theme));
        let icon = status_icon(item.status);

        lines.push(Line::from(Span::styled(format!("{icon} {subject}"), style)));
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
    frame.render_widget(paragraph, inner);
}

/// Glyph colors for a task status: queued work stays bright, the
/// item in flight takes the task-progress color the agent list's
/// compact row already uses, and completed items dim out.
fn status_color(
    status: TaskItemStatus,
    theme: &crate::ui::colors::ColorTheme,
) -> ratatui::style::Color {
    match status {
        TaskItemStatus::Pending => theme.text_active,
        TaskItemStatus::InProgress => theme.task_progress,
        TaskItemStatus::Completed => theme.text_muted,
    }
}

fn status_icon(status: TaskItemStatus) -> &'static str {
    match status {
        TaskItemStatus::Pending => "◻",
        TaskItemStatus::InProgress => "◼",
        TaskItemStatus::Completed => "✔",
    }
}
