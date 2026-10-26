//! The Tasks tab: the focused pane's stored todo list.
//!
//! Items and the pane description come from [`state.tasks`](crate::state::TasksState),
//! which `refresh_pane_tasks` fills from the pane's store file. The
//! agent writes items and statuses through the `task-set` /
//! `task-update` / `task-describe` subcommands; the sidebar's one
//! write is the close sweep, which empties a list that has sat
//! all-completed and untouched for 90 seconds.

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

    // The description header is pinned above the scrolling list in
    // the standard text color, with one blank row between it and the
    // first item.
    let mut header_lines = description_header(&state.tasks.description, padded.width, theme);
    let has_header = !header_lines.is_empty();
    let header_h = if has_header {
        // One row of padding between the description and the list.
        (header_lines.len() + 1).min(padded.height as usize) as u16
    } else {
        0
    };
    if has_header {
        header_lines.push(Line::from(""));
        frame.render_widget(
            Paragraph::new(header_lines),
            Rect {
                height: header_h,
                ..padded
            },
        );
    }
    let items_area = Rect {
        y: padded.y + header_h,
        height: padded.height - header_h,
        ..padded
    };

    if state.tasks.items.is_empty() {
        super::render_centered(
            frame,
            if has_header { items_area } else { padded },
            "No tasks recorded",
            theme.text_muted,
        );
        return;
    }

    // Leave one blank row above the first item for breathing room,
    // inside the scroll content like the Activity tab's spacer. A
    // pinned description already supplies that gap, so it skips the
    // spacer.
    let mut lines: Vec<Line<'_>> = Vec::new();
    if !has_header {
        lines.push(Line::from(""));
    }

    // The icon plus its separating space leave the rest for the subject.
    let subject_w = (padded.width as usize).saturating_sub(2);

    for item in &state.tasks.items {
        let style = Style::default().fg(status_color(item.status, theme));
        for (i, text) in wrap_subject(&item.subject, subject_w).iter().enumerate() {
            // Continuation rows indent under the subject, not the icon.
            let prefix = if i == 0 {
                format!("{} ", status_icon(item.status))
            } else {
                "  ".to_string()
            };
            lines.push(Line::from(Span::styled(format!("{prefix}{text}"), style)));
        }
    }

    state.tasks.scroll.total_lines = lines.len();
    state.tasks.scroll.visible_height = items_area.height as usize;
    // Clamp `offset` to the new total/visible. When the list shrinks
    // (items trimmed, focus change) the stale offset would otherwise
    // produce an empty or over-scrolled paragraph — same clamp-on-shrink
    // behavior as the Activity tab.
    state.tasks.scroll.scroll(0);

    let scroll_offset = state.tasks.scroll.offset as u16;
    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    frame.render_widget(paragraph, items_area);
}

/// The pinned description header: each source line of the description
/// wraps to `width` in the standard text color. An empty description
/// returns no lines — the caller renders the list without a header.
fn description_header<'a>(
    description: &str,
    width: u16,
    theme: &crate::ui::colors::ColorTheme,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();
    if description.is_empty() {
        return lines;
    }
    for segment in description.split('\n') {
        let wrapped = wrap_text(segment, width as usize, usize::MAX);
        if wrapped.is_empty() {
            // A blank source line still takes a row.
            lines.push(Line::from(""));
        } else {
            lines.extend(
                wrapped
                    .into_iter()
                    .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.text_active)))),
            );
        }
    }
    lines
}

/// One item's subject as wrapped row strings: subjects hold free text
/// that may contain newlines, and every source line wraps on its own
/// so the full text is always shown.
fn wrap_subject(subject: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for segment in subject.split('\n') {
        let wrapped = wrap_text(segment, width, usize::MAX);
        if wrapped.is_empty() {
            // A blank source line still takes a row.
            rows.push(String::new());
        } else {
            rows.extend(wrapped);
        }
    }
    rows
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
