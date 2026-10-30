use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::ui::colors::ColorTheme;
use crate::ui::text::pad_to;

/// Left-edge marker character used for the currently selected pane.
pub(super) const SELECTION_MARKER: &str = "┃";

pub(super) struct RowCtx<'a> {
    /// 1-column left marker: `┃` when the pane is selected, otherwise a space.
    pub(super) marker_char: &'static str,
    /// Style for the left marker (fg + optional bg already applied).
    pub(super) marker_style: Style,
    /// Extra blank columns after the marker and its trailing space, which
    /// push a nested subagent's content right by that many columns. The
    /// inner width already has this indent subtracted.
    pub(super) indent: usize,
    /// Usable inner width for content after the marker, its trailing space
    /// and the indent.
    pub(super) inner_width: usize,
    pub(super) theme: &'a ColorTheme,
    pub(super) bg: Option<Color>,
    pub(super) active: bool,
    /// The branch label lifted onto the status row (strip only): the
    /// strip's agent list has no row to spare for a dedicated branch
    /// line, so the branch name rides after the pane name instead.
    /// `None` on every other row and in the column layout.
    pub(super) inline_branch: Option<String>,
    /// Line cap for the prompt/response preview: 3 in a column, 1 in a
    /// strip — a top/bottom strip cannot spend three rows on the
    /// last-conversation marker, so it truncates to a single
    /// ellipsis-closed line.
    pub(super) prompt_max_lines: usize,
}

impl RowCtx<'_> {
    #[inline]
    pub(super) fn apply_bg(&self, style: Style) -> Style {
        match self.bg {
            Some(c) => style.bg(c),
            None => style,
        }
    }

    pub(super) fn row_line(
        &self,
        content_spans: Vec<Span<'static>>,
        content_width: usize,
    ) -> Line<'static> {
        let padding = pad_to(content_width, self.inner_width);
        let bg_default = self.apply_bg(Style::default());
        let mut spans = Vec::with_capacity(content_spans.len() + 4);
        spans.push(Span::styled(self.marker_char, self.marker_style));
        spans.push(Span::styled(" ", bg_default));
        if self.indent > 0 {
            spans.push(Span::styled(" ".repeat(self.indent), bg_default));
        }
        spans.extend(content_spans);
        spans.push(Span::styled(padding, bg_default));
        Line::from(spans)
    }

    pub(super) fn row_line_split(
        &self,
        left_spans: Vec<Span<'static>>,
        left_width: usize,
        right_spans: Vec<Span<'static>>,
        right_width: usize,
    ) -> Line<'static> {
        let padding = self.inner_width.saturating_sub(left_width + right_width);
        let bg_default = self.apply_bg(Style::default());
        let mut spans = Vec::with_capacity(left_spans.len() + right_spans.len() + 4);
        spans.push(Span::styled(self.marker_char, self.marker_style));
        spans.push(Span::styled(" ", bg_default));
        if self.indent > 0 {
            spans.push(Span::styled(" ".repeat(self.indent), bg_default));
        }
        spans.extend(left_spans);
        spans.push(Span::styled(" ".repeat(padding), bg_default));
        spans.extend(right_spans);
        Line::from(spans)
    }
}
