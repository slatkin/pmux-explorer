//! The Thought tab: the focused pi pane's last thought.
//!
//! The text comes from [`state.thought`](crate::state::ThoughtState),
//! which `refresh_thought_feed` fills from the pane's thinking feed on
//! the 1 s refresh while this tab is displayed. The tab draws one muted
//! `HH:mm:ss` line — the local time of the record that began the
//! thought — above the wrapped thought, and follows the end of a
//! growing thought until the user scrolls up (design D6).

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::state::AppState;
use crate::ui::text::wrap_text;

pub(super) fn draw_thought_content(frame: &mut Frame, state: &mut AppState, inner: Rect) {
    let theme = &state.theme;

    if state.thought.last.is_empty() {
        super::render_centered(frame, inner, "No thinking yet", theme.text_muted);
        return;
    }

    // The time line lives inside the scroll content, so a tall thought
    // shows its last rows whole — the spec pins the last wrapped rows of
    // the thought, not a pinned stamp above them.
    let mut lines: Vec<Line<'_>> = Vec::new();
    if let Some(ts) = &state.thought.last.start_ts {
        // An unparsable `ts` draws no time line: the stamp is a
        // nicety, and a wrong-looking stamp would be worse than none.
        if let Some(hhmmss) = crate::time::local_hhmmss_from_ts(ts) {
            lines.push(Line::from(Span::styled(
                hhmmss,
                Style::default().fg(theme.text_muted),
            )));
        }
    }

    for row in wrapped_thought(&state.thought.last.text, inner.width as usize) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(theme.text_active),
        )));
    }

    // A new `start` block resets the scroll to follow the end; the
    // seen `ts` is compared on every draw, so the reset needs no
    // extra tick plumbing.
    if state.thought.last.start_ts != state.thought.seen_start_ts {
        state
            .thought
            .seen_start_ts
            .clone_from(&state.thought.last.start_ts);
        state.thought.follow = true;
    }

    state.scrolls.thought.total_lines = lines.len();
    state.scrolls.thought.visible_height = inner.height as usize;
    // While the follow stands the offset moves to the last page; a
    // suspended follow only clamps, so a shrinking thought cannot
    // leave a stale offset pointing past the end.
    if state.thought.follow {
        state.scrolls.thought.offset = state
            .scrolls
            .thought
            .total_lines
            .saturating_sub(state.scrolls.thought.visible_height);
    } else {
        state.scrolls.thought.scroll(0);
    }

    let scroll_offset = state.scrolls.thought.offset as u16;
    let paragraph = Paragraph::new(lines).scroll((scroll_offset, 0));
    frame.render_widget(paragraph, inner);
}

/// The thought as wrapped row strings: the text holds free text with
/// line breaks that must stay, so every source line wraps on its own
/// and a blank line still takes a row — the same shape the Tasks tab's
/// subjects wrap by.
fn wrapped_thought(text: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for segment in text.split('\n') {
        let wrapped = wrap_text(segment, width, usize::MAX);
        if wrapped.is_empty() {
            rows.push(String::new());
        } else {
            rows.extend(wrapped);
        }
    }
    rows
}
