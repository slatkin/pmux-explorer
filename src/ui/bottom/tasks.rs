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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BottomTab;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::{Color, Modifier};

    /// Render the tasks panel at the given size and return the resulting
    /// `AppState` plus the buffer-backed terminal.
    fn draw(state: &mut AppState, width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, width, height);
                draw_tasks_content(frame, state, area);
            })
            .unwrap();
        terminal
    }

    /// Snapshot the tasks panel as plain text. Trailing whitespace on
    /// each row and trailing empty rows are trimmed so inline snapshots
    /// stay readable.
    fn render(state: &mut AppState, width: u16, height: u16) -> String {
        let terminal = draw(state, width, height);
        let buf = terminal.backend().buffer().clone();
        let mut rows: Vec<String> = Vec::new();
        for y in 0..buf.area.height {
            let mut line = String::new();
            for x in 0..buf.area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            rows.push(line.trim_end().to_string());
        }
        while rows.last().is_some_and(|l| l.is_empty()) {
            rows.pop();
        }
        rows.join("\n")
    }

    fn item(id: &str, subject: &str, status: TaskItemStatus) -> crate::task_store::TaskItem {
        crate::task_store::TaskItem {
            id: id.into(),
            subject: subject.into(),
            status,
        }
    }

    #[test]
    fn tasks_list_renders_icon_and_subject_per_item() {
        let mut state = AppState::new("%99".into());
        state.tasks.items = vec![
            item("1", "Write the store", TaskItemStatus::Completed),
            item("2", "Render the tab", TaskItemStatus::InProgress),
            item("3", "Prove with pi", TaskItemStatus::Pending),
        ];

        insta::assert_snapshot!(render(&mut state, 30, 8), @"

        ✔ Write the store
        ◼ Render the tab
        ◻ Prove with pi
        ");
    }

    #[test]
    fn tasks_tab_shows_an_empty_state_without_items() {
        let mut state = AppState::new("%99".into());

        insta::assert_snapshot!(render(&mut state, 30, 6), @"
            No tasks recorded
        ");
    }

    #[test]
    fn tasks_subject_truncates_at_narrow_width() {
        let mut state = AppState::new("%99".into());
        state.tasks.items = vec![item(
            "1",
            "a-very-long-task-subject-that-runs-out-of-columns",
            TaskItemStatus::Pending,
        )];

        insta::assert_snapshot!(render(&mut state, 24, 4), @"
        ◻ a-very-long-task-subj…
        ");
    }

    #[test]
    fn completed_items_dim_and_pending_stay_bright() {
        let mut state = AppState::new("%99".into());
        state.tasks.items = vec![
            item("1", "done", TaskItemStatus::Completed),
            item("2", "wip", TaskItemStatus::InProgress),
        ];

        let terminal = draw(&mut state, 30, 6);
        let buf = terminal.backend().buffer().clone();
        // Row 0 is the spacer, row 1 the first item: icon at x=0.
        let first = &buf[(0, 1)];
        assert_eq!(first.symbol(), "✔");
        assert_eq!(
            first.fg,
            Color::Indexed(252),
            "completed dims to text_muted"
        );
        let second = &buf[(0, 2)];
        assert_eq!(second.symbol(), "◼");
        assert_eq!(
            second.fg,
            Color::Indexed(223),
            "in-progress takes the task-progress color"
        );
        let _ = Modifier::empty();
    }

    #[test]
    fn multi_line_subject_shows_first_line_only() {
        // The store round-trips free text with newlines (pinned by the
        // task-store test); the one-row-per-item list shows its first line.
        let mut state = AppState::new("%99".into());
        state.tasks.items = vec![item(
            "1",
            "first line\nsecond line",
            TaskItemStatus::Pending,
        )];

        insta::assert_snapshot!(render(&mut state, 30, 4), @"
        ◻ first line
        ");
    }

    #[test]
    fn tasks_scroll_clamps_when_the_list_shrinks() {
        // The same clamp-on-shrink the Activity tab renders with: an
        // offset past the new total collapses back instead of showing
        // a blank paragraph.
        let mut state = AppState::new("%99".into());
        state.tasks.items = (0..10)
            .map(|i| {
                item(
                    &i.to_string(),
                    &format!("task {i}"),
                    TaskItemStatus::Pending,
                )
            })
            .collect();

        let _ = render(&mut state, 30, 6);
        // 10 items + spacer = 11 lines, viewport 6 → max offset 5.
        state.tasks.scroll.offset = 5;
        assert_eq!(state.tasks.scroll.total_lines, 11);

        // The list drops to one item: re-rendering pulls the offset back.
        state.tasks.items = vec![item("1", "only", TaskItemStatus::Pending)];
        let _ = render(&mut state, 30, 6);
        assert!(state.tasks.scroll.offset <= 1);
        assert_eq!(state.bottom_tab, BottomTab::Activity);
    }
}
