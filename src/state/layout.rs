use ratatui::layout::Rect;

use super::{AppState, BottomTab, RepoFilter, StatusCounts, StatusFilter, clamp_bottom_height};

/// Whether a sidebar instance is a full-height column or a full-width
/// strip. The `@sidebar_position` value that made the sidebar picks it
/// (`Left`/`Right` make a column, `Top`/`Bottom` a strip), and the
/// layout is fixed for the life of the pane: a later option change, a
/// resize, a side switch or a pop-out never crosses the kinds (the
/// `strip-layout` capability). Every code path that differs between the
/// two shapes branches on this value, never on the pane shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Column,
    Strip,
}

impl LayoutKind {
    /// The layout a `PMUX_SIDEBAR_LAYOUT` process-environment value
    /// selects: `strip` makes a strip, `column` a column, and a missing,
    /// empty or unknown value a column — a sidebar that started before
    /// the strip layout existed (or with a garbage value) keeps the
    /// column behavior. The value is read once at startup
    /// (`app/setup.rs`), never from a pane option: `split-window` starts
    /// the TUI before `toggle` can write `@pane_sidebar_layout`, so the
    /// environment is the only startup source that cannot lose the race.
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value {
            Some("strip") => LayoutKind::Strip,
            _ => LayoutKind::Column,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RowTarget {
    pub pane_id: String,
}

/// Click target for the `+` button rendered at the right edge of each
/// repo-group header in the agents panel. Clicking it opens the spawn
/// modal prefilled for that repo.
#[derive(Debug, Clone)]
pub struct RepoSpawnTarget {
    pub rect: ratatui::layout::Rect,
    pub repo_name: String,
    pub repo_root: String,
}

/// Click target for the red `×` rendered next to the branch of a
/// sidebar-spawned pane. Clicking it opens the close-pane confirmation
/// for that specific pane.
#[derive(Debug, Clone)]
pub struct SpawnRemoveTarget {
    pub rect: ratatui::layout::Rect,
    pub pane_id: String,
}

/// Click target for one tab label on the bottom panel's rule — or on
/// a strip's footer tab bar, the same horizontal band on the tab
/// section's bottom row: the columns the label text occupies as the
/// last frame drew them. The renderer truncates labels that do not
/// fit, so a label pushed off the band has a shrunken range — or none
/// — and a click can only ever land on a label that is drawn.
#[derive(Debug, Clone)]
pub struct BottomTabTarget {
    /// First column of the drawn label (0-based terminal column).
    pub start: u16,
    /// Column after the label's last drawn column.
    pub end: u16,
    /// The tab a click in the range selects.
    pub tab: BottomTab,
}

/// Screen-positioned hyperlink overlay for OSC 8 terminal hyperlinks.
#[derive(Debug, Clone)]
pub struct HyperlinkOverlay {
    pub x: u16,
    pub y: u16,
    pub text: String,
    pub url: String,
}

/// Ephemeral render output cached for click hit-testing.
///
/// Every field here is **rewritten on every frame** by the UI layer and
/// only read by event handlers (mouse/keyboard) before the next render.
/// Bundling them under `state.layout` makes the "frame-scoped vs
/// persistent state" boundary visible at a glance, since the rest of
/// `AppState` only holds data that survives across frames.
#[derive(Debug, Clone, Default)]
pub struct FrameLayout {
    /// Filtered pane list, in the order the UI rendered them. Index
    /// matches `GlobalState::selected_pane_row`.
    pub pane_row_targets: Vec<RowTarget>,
    /// Maps each rendered text line in the agents panel back to a row in
    /// `pane_row_targets`. `None` for header/blank lines that should not
    /// route clicks to a pane.
    pub line_to_row: Vec<Option<usize>>,
    /// X column of the repo filter button in the secondary header. `None`
    /// when the button is hidden. Used for click hit-testing.
    pub repo_button_col: Option<u16>,
    /// Width of the filter bar row (row 0) as last rendered. `0` before
    /// the first frame, which leaves the row's chips and control
    /// unclickable. The hit test recomputes the chip layout from this
    /// width and the live counts through the same [`filter_bar_layout`]
    /// the renderer uses, so the drawn chips and the clickable chips
    /// cannot drift apart — including when chips are dropped for space.
    pub filter_bar_width: u16,
    /// Effective height of the Activity/Git bottom panel as last
    /// rendered: the configured height while docked, the clamped height
    /// (at most a third of the panel) while the sidebar floats. `0`
    /// before the first frame, which leaves the bottom panel
    /// unclickable and routes every scroll to the agents panel. The
    /// mouse handlers read this back instead of the raw
    /// `bottom_panel_height`, so the hit-tested geometry is the drawn
    /// geometry by construction — the two can never drift apart.
    pub bottom_panel_height: u16,
    /// Click regions for the `[+]` spawn button rendered at the right
    /// edge of each repo-group header. One entry per visible repo group.
    pub repo_spawn_targets: Vec<RepoSpawnTarget>,
    /// Click regions for the red `×` remove marker rendered next to the
    /// branch of each sidebar-spawned pane. One entry per visible row.
    pub spawn_remove_targets: Vec<SpawnRemoveTarget>,
    /// OSC 8 hyperlink overlays the main loop writes after each frame so
    /// terminals can recognise PR numbers as clickable links.
    pub hyperlink_overlays: Vec<HyperlinkOverlay>,
    /// The tab labels' drawn column ranges on the bottom panel's rule,
    /// as the last frame computed them while truncating the run to the
    /// rule's width. Rewritten every frame; empty before the first
    /// frame and while the panel is hidden, when no label is
    /// clickable.
    pub bottom_tab_targets: Vec<BottomTabTarget>,
    /// The strip's panel geometry as the last frame laid it out. The
    /// field is `Some` whenever this sidebar is a strip
    /// ([`LayoutKind::Strip`]) and a frame has drawn — a hidden tab
    /// panel is `strip.tabs == None`, not a `None` field — and `None`
    /// before the first frame and on a column. The mouse handlers
    /// branch on it so a strip press routes by the panels the frame
    /// actually drew.
    pub strip: Option<StripLayout>,
}

pub(super) fn point_in_rect(row: u16, col: u16, rect: ratatui::layout::Rect) -> bool {
    rect.contains(ratatui::layout::Position { x: col, y: row })
}

/// One chip of the status filter: its category and the extent it
/// occupies on screen (icon plus count digits). The meaning of
/// `start` depends on the layout that produced the chip: on the
/// horizontal filter row it is the column of the chip's icon
/// ([`filter_bar_layout`]), on the strip's vertical filter bar it is
/// the row the chip occupies ([`filter_column_layout`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterChip {
    pub filter: StatusFilter,
    /// Column of the chip's icon, or its row on the strip's vertical
    /// filter bar.
    pub start: u16,
    /// Number of columns the chip occupies.
    pub width: u16,
}

/// Number of decimal digits `n` renders as; `0` renders as one digit.
fn count_digits(n: usize) -> u16 {
    n.checked_ilog10().map_or(1, |d| d + 1) as u16
}

/// Compute the layout of the status filter row for a row `width` columns
/// wide with the per-filter status counts (see [`StatusCounts`]).
///
/// The pop-out control owns the row's two rightmost columns and the
/// side-switch control the two left of it — each the glyph, then the
/// one-column spacer that keeps it a space from its neighbour — and
/// the chips fill from the left with a single-space separator after one
/// gutter column. A chip that would no longer fit before the leftmost
/// control is dropped whole — never partially drawn — together with
/// every chip after it. Returns the chips that fit and both control
/// columns; the side switch is `None` on a row narrower than 4 columns,
/// where both glyphs cannot fit side by side.
///
/// Both the renderer and the click hit test run through this function,
/// so a click can only ever select a chip that is actually drawn.
pub(crate) fn filter_bar_layout(
    counts: StatusCounts,
    width: u16,
) -> (Vec<FilterChip>, Option<u16>, Option<u16>) {
    if width == 0 {
        return (Vec::new(), None, None);
    }
    let popout_col = width.saturating_sub(2);
    let switch_col = width.checked_sub(4);
    // The chips must end before the leftmost control: the side switch
    // when it is drawn, the pop-out alone on a narrow row.
    let control_limit = switch_col.unwrap_or(popout_col);
    let mut chips = Vec::new();
    let mut start: u16 = 1; // leading gutter column
    for filter in StatusFilter::VARIANTS {
        // A chip is its icon, one space, then the count digits; the
        // space sits inside the chip so a click on it selects the chip.
        let chip_width = 2 + count_digits(counts.count(filter));
        if start + chip_width > control_limit {
            break;
        }
        chips.push(FilterChip {
            filter,
            start,
            width: chip_width,
        });
        start += chip_width + 1; // single-space separator
    }
    (chips, switch_col, Some(popout_col))
}

/// Columns the strip's vertical filter bar occupies at the left edge:
/// one chip per row, drawn as icon, space and count.
const STRIP_FILTER_COLS: u16 = 4;
/// Columns the strip's vertical tab divider occupies between the agent
/// panel and the tab panel: a single thin line. The tab glyphs live on
/// the footer row instead, so the divider is one column wide.
const STRIP_DIVIDER_COLS: u16 = 1;
/// Columns either strip panel keeps at least: the agent panel and the
/// tab panel (divider included) each never shrink below this.
const STRIP_MIN_PANEL_COLS: u16 = 20;

/// The panel geometry of a strip as the last frame laid it out: the
/// vertical filter bar at the left edge, the agent panel next to it,
/// and — while the tab panel shows — the thin divider line and the
/// tab panel to the right edge, with the tab bar as a footer row
/// across the tab section's bottom. Every `Rect` except `footer`
/// spans the full strip height and all are positioned in the strip
/// pane's own coordinates (the TUI's frame is the whole sidebar pane,
/// so `y` is 0 and the `x` columns are the frame's absolute columns
/// the mouse handlers test against).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripLayout {
    /// The 4-column vertical filter bar, `x 0..4`.
    pub filter: Rect,
    /// The agent panel, right of the filter bar.
    pub agents: Rect,
    /// The 1-column divider line, while the tab panel shows.
    pub divider: Option<Rect>,
    /// The tab panel, right of the divider, while it shows. Its last
    /// row is the footer tab bar, not content.
    pub tabs: Option<Rect>,
    /// The footer tab bar: the bottom row of the tab section, the
    /// divider column included, while the tab panel shows.
    pub footer: Option<Rect>,
}

/// The geometry of a strip `width` columns wide and `height` rows tall
/// with the tab panel taking `tab_percent` percent of the strip width,
/// divider included — the single split geometry the renderer, the hit
/// test and the divider drag all read, so the drawn panels and the
/// clickable panels cannot be in different columns.
///
/// The filter bar always occupies the left [`STRIP_FILTER_COLS`]
/// columns. The tab width is the rounded percent clamped so the agent
/// panel keeps at least [`STRIP_MIN_PANEL_COLS`] columns and the tab
/// panel with its divider keeps at least [`STRIP_MIN_PANEL_COLS`]. A
/// percent of 0, or a strip too narrow for the filter bar and both
/// minimums, hides the divider and the tab panel and gives the agent
/// panel the columns the filter bar leaves.
pub fn strip_layout(width: u16, height: u16, tab_percent: u8) -> StripLayout {
    let wanted = if tab_percent == 0 {
        0
    } else {
        ((u32::from(width) * u32::from(tab_percent) + 50) / 100) as u16
    };
    strip_layout_for_tab_cols(width, height, wanted)
}

/// [`strip_layout`] keyed on tab columns instead of a percent: a moved
/// divider drag lays the strip out with the drag's live tab columns, so
/// the drawn divider sits on the mouse column (`draw_strip`). The
/// clamping is the percent path's own — a count of 0 hides the divider
/// and the tab panel, and both panels keep their
/// [`STRIP_MIN_PANEL_COLS`] minimums.
pub fn strip_layout_for_tab_cols(width: u16, height: u16, tab_cols: u16) -> StripLayout {
    let filter_width = width.min(STRIP_FILTER_COLS);
    let filter = Rect::new(0, 0, filter_width, height);
    let agents_x = filter_width;
    let mut agents = Rect::new(agents_x, 0, width.saturating_sub(filter_width), height);
    let mut divider = None;
    let mut tabs = None;
    let mut footer = None;
    if tab_cols > 0 && width >= STRIP_FILTER_COLS + 2 * STRIP_MIN_PANEL_COLS {
        let available = width - STRIP_FILTER_COLS;
        let tab_cols = tab_cols.clamp(STRIP_MIN_PANEL_COLS, available - STRIP_MIN_PANEL_COLS);
        let agents_width = available - tab_cols;
        agents.width = agents_width;
        let divider_x = agents_x + agents_width;
        divider = Some(Rect::new(divider_x, 0, STRIP_DIVIDER_COLS, height));
        tabs = Some(Rect::new(
            divider_x + STRIP_DIVIDER_COLS,
            0,
            tab_cols - STRIP_DIVIDER_COLS,
            height,
        ));
        // The footer is the tab section's bottom row, divider column
        // included — a zero-height strip has no row to hold it.
        footer = height
            .checked_sub(1)
            .map(|last| Rect::new(divider_x, last, tab_cols, 1));
    }
    StripLayout {
        filter,
        agents,
        divider,
        tabs,
        footer,
    }
}

/// The inverse of [`strip_layout`] for a divider-drag release: tab
/// columns (divider included) are this percent of a strip `width`
/// columns wide, rounded to the nearest percent. The result saturates
/// at 100, the largest percent `@sidebar_tab_width` accepts. A drag
/// that leaves the divider on column `c` holds `width - c` tab
/// columns, so re-laying the saved percent out puts the divider back
/// on the column the user dragged it to.
pub fn cols_for_percent(cols: u16, width: u16) -> u8 {
    if width == 0 {
        return 0;
    }
    ((u32::from(cols) * 100 + u32::from(width) / 2) / u32::from(width)).min(100) as u8
}

/// The layout of the strip's vertical filter bar: one chip per row,
/// in the [`StatusFilter::VARIANTS`] order from the first row, and
/// the control row (the side switch at column 0, the pop-out at
/// column 2) on the last row. A chip that does not fit above the
/// control row is dropped whole, together with every chip after it.
///
/// The counts shape only the chip widths — a chip is its icon, one
/// space, then the count digits, and a count of 100 or more is drawn
/// as `99`, so a chip never needs more than the bar's
/// [`STRIP_FILTER_COLS`] columns (the count is a hint here, the agent
/// list shows the truth). Both the renderer and the click hit test
/// run through this function, so a click can only ever select a chip
/// that is actually drawn.
///
/// A strip of fewer than 2 rows cannot hold a chip and the control
/// row: one row draws only the control, no rows draw neither.
pub fn filter_column_layout(counts: StatusCounts, height: u16) -> (Vec<FilterChip>, Option<u16>) {
    let Some(control_row) = height.checked_sub(1) else {
        return (Vec::new(), None);
    };
    let chips = StatusFilter::VARIANTS
        .iter()
        .enumerate()
        .take_while(|(row, _)| u16::try_from(*row).is_ok_and(|row| row < control_row))
        .map(|(row, filter)| {
            let digits = count_digits(counts.count(*filter).min(99));
            FilterChip {
                filter: *filter,
                start: row as u16,
                width: 2 + digits,
            }
        })
        .collect();
    (chips, Some(control_row))
}

/// Rows the bottom panel draws for a `percent` of a sidebar `height`
/// rows tall, rule included — the single split geometry the renderer,
/// the hit test and the split drag all read, so the drawn rule and the
/// clickable rule cannot be in different rows.
///
/// A percent of 0, or a sidebar shorter than 6 rows, hides the panel.
/// Otherwise the rounded percent is clamped so the panel keeps at least
/// 3 rows (the rule and two content rows) and the agent list keeps at
/// least 3.
pub(crate) fn split_rows(percent: u8, height: u16) -> u16 {
    if percent == 0 || height < 6 {
        return 0;
    }
    let rows = (u32::from(height) * u32::from(percent) + 50) / 100;
    (rows as u16).clamp(3, height - 3)
}

/// The inverse of [`split_rows`] for a split-drag release: a bottom
/// panel of `rows` rows is this percent of a sidebar `height` rows tall,
/// rounded to the nearest percent. The result saturates at 100, the
/// largest percent `@sidebar_bottom_height` accepts.
pub(crate) fn percent_for_rows(rows: u16, height: u16) -> u8 {
    if height == 0 {
        return 0;
    }
    ((u32::from(rows) * 100 + u32::from(height) / 2) / u32::from(height)).min(100) as u8
}

/// A press on the column's rule or the strip's divider that may still
/// become a drag or a tab click. A release with a move saves the
/// percent `size` round-trips to (decision 4); a release without one
/// is a click and selects the tab the press names.
#[derive(Debug, Clone, Copy)]
pub struct SplitDrag {
    /// The layout the press belongs to: a column drag resizes the
    /// bottom panel by rows, a strip drag the two strip panels by tab
    /// columns — and each axis's release saves its own shared option
    /// (`@sidebar_bottom_height` for a column, `@sidebar_tab_width` for
    /// a strip).
    pub axis: LayoutKind,
    /// Column of the press, kept for the unmoved release's tab click:
    /// both tab bars are horizontal (a column's rule, a strip's
    /// footer), so the release reads the press column against the
    /// drawn glyph columns.
    pub press_col: u16,
    /// Row of the press, kept for the unmoved release of a strip
    /// press: only a press on the footer row resolves to a tab click —
    /// a press on the thin divider line above it resolves to nothing.
    pub press_row: u16,
    /// Whether a drag event reached a row other than the rule row (a
    /// column) or a column other than the divider's (a strip).
    pub moved: bool,
    /// The panel size the drag shows, already clamped to the split
    /// minimums — the bottom-panel rows (rule included) for a column,
    /// the tab columns (divider included) for a strip — the renderer
    /// draws exactly this much for as long as the drag moves.
    pub size: u16,
}

/// Clamp a split drag's row count to the limits decision 4 takes over
/// from decision 2 plus the floating third-panel limit. `0` mirrors
/// [`split_rows`] on a sidebar too short for a split: no rule survives.
fn clamp_drag_rows(rows: u16, term_height: u16, floating: bool) -> u16 {
    if term_height < 6 {
        return 0;
    }
    let rows = rows.clamp(3, term_height - 3);
    if floating {
        clamp_bottom_height(rows, term_height)
    } else {
        rows
    }
}

/// Clamp a strip drag's tab-column count to the same limits
/// [`strip_layout`] lays the percent out with: the tab panel with its
/// divider keeps [`STRIP_MIN_PANEL_COLS`] columns and the agent panel
/// its own. `available` is the width the filter bar leaves — the drawn
/// strip's full width minus its filter columns. A strip too narrow for
/// both minimums draws no divider, so no strip drag can be live on it;
/// the guard only keeps the clamp away from a `min > max` panic there.
fn clamp_drag_cols(cols: u16, available: u16) -> u16 {
    if available < 2 * STRIP_MIN_PANEL_COLS {
        return cols;
    }
    cols.clamp(STRIP_MIN_PANEL_COLS, available - STRIP_MIN_PANEL_COLS)
}

/// The full width the drawn [`StripLayout`] spans: the four rects tile
/// the frame without gaps or overlap, so their widths sum to the
/// strip's width. Both the drag move and the drag release convert a
/// mouse column against it.
fn drawn_strip_width(strip: &StripLayout) -> u16 {
    strip.filter.width
        + strip.agents.width
        + strip.divider.map_or(0, |divider| divider.width)
        + strip.tabs.map_or(0, |tabs| tabs.width)
}

impl AppState {
    pub fn rebuild_row_targets(&mut self) {
        // Reset stale repo filter if the repo no longer exists, and
        // persist the reset back to tmux so fresh sidebar instances do
        // not reload the dead repo name on startup.
        if let RepoFilter::Repo(ref name) = self.global.repo_filter
            && !self.repo_groups.iter().any(|g| g.name == *name)
        {
            self.global.repo_filter = RepoFilter::All;
            self.global.save_repo_filter();
        }

        self.layout.pane_row_targets.clear();
        for group in &self.repo_groups {
            if !self.global.repo_filter.matches_group(&group.name) {
                continue;
            }
            for (pane, _) in &group.panes {
                if self.global.status_filter.matches(&pane.status) {
                    self.layout.pane_row_targets.push(RowTarget {
                        pane_id: pane.pane_id.clone(),
                    });
                }
            }
        }
        if self.global.selected_pane_row >= self.layout.pane_row_targets.len()
            && !self.layout.pane_row_targets.is_empty()
        {
            self.global.selected_pane_row = self.layout.pane_row_targets.len() - 1;
        }
    }

    /// Handle a left mouse press at (`row`, `col`) in a terminal
    /// `term_height` rows tall. The routing branches on the sidebar's
    /// layout kind, and the hit test reads only the geometry the last
    /// frame drew ([`FrameLayout`]), so a press can only ever land on a
    /// panel that was drawn:
    ///
    /// - A column routes by row as before: a press on the rule (the
    ///   bottom panel's first drawn row) starts a split drag — any
    ///   press column can drag, and only an unmoved release selects a
    ///   tab (decision 4) — and a press above the rule goes to the
    ///   agents panel.
    /// - A strip routes by the drawn [`StripLayout`]: a press on the
    ///   divider starts a strip drag, a press in the filter column goes
    ///   to [`Self::handle_filter_column_click`], and a press in the
    ///   agent panel to [`Self::handle_strip_agent_click`].
    ///
    /// Returns `true` when the press started a split drag (the rule row
    /// of a column, the divider of a strip).
    ///
    /// A press first ends a live drag whose release never reached this
    /// pane (a Down on another pane consumed it): the stale drag ends
    /// as if released at its last size so the instance is not left
    /// blocked from percent adoption, and only the moved branch of that
    /// stale end runs — the new press, not the lost release, is the
    /// click that may resolve on a label (decision 4).
    pub fn handle_terminal_click(&mut self, row: u16, col: u16, term_height: u16) -> bool {
        // A stale drag whose release was lost: a moved one saves its
        // rows (the moved branch only); an unmoved one is only a held
        // press and ends with no save (nothing was resized) and no tab
        // click, because the user did not release on that label
        // (decision 4).
        if let Some(drag) = self.split_drag.take()
            && drag.moved
        {
            self.save_split_drag(drag, term_height);
        }
        match self.layout_kind {
            LayoutKind::Column => self.handle_column_click(row, col, term_height),
            LayoutKind::Strip => self.handle_strip_press(row, col),
        }
    }

    /// The column branch of [`Self::handle_terminal_click`]: the row
    /// routing a column has always had. `true` only on the rule row,
    /// where the press starts a split drag.
    fn handle_column_click(&mut self, row: u16, col: u16, term_height: u16) -> bool {
        let bottom_start = term_height.saturating_sub(self.layout.bottom_panel_height);
        if row < bottom_start {
            self.handle_mouse_click(row, col);
            false
        } else if row == bottom_start {
            // A press anywhere on the rule starts a drag. On the drawn
            // rows, so the release's unmoved click and the drawn rule
            // agree even when the panel floats.
            self.split_drag = Some(SplitDrag {
                axis: LayoutKind::Column,
                press_col: col,
                press_row: row,
                moved: false,
                size: self.layout.bottom_panel_height,
            });
            true
        } else {
            false
        }
    }

    /// The strip branch of [`Self::handle_terminal_click`]: route by
    /// the drawn [`StripLayout`]. A press on the thin divider line or
    /// on the footer tab bar starts a strip drag (its `size` is the
    /// drawn tab columns, divider included) — the same rule the
    /// column's rule row keeps, where the bar is both the drag handle
    /// and the tab strip; the filter column and the agent panel go to
    /// their handlers. `true` only where the press starts the drag.
    fn handle_strip_press(&mut self, row: u16, col: u16) -> bool {
        let Some(strip) = self.layout.strip else {
            return false;
        };
        if strip.divider.is_some_and(|d| point_in_rect(row, col, d))
            || strip.footer.is_some_and(|f| point_in_rect(row, col, f))
        {
            self.split_drag = Some(SplitDrag {
                axis: LayoutKind::Strip,
                press_col: col,
                press_row: row,
                moved: false,
                size: strip.divider.map_or(0, |d| d.width)
                    + strip.tabs.map_or(0, |tabs| tabs.width),
            });
            return true;
        }
        if point_in_rect(row, col, strip.filter) {
            self.handle_filter_column_click(row, col);
            return false;
        }
        if point_in_rect(row, col, strip.agents) {
            self.handle_strip_agent_click(row, col, strip.agents);
        }
        false
    }

    /// Handle a press in the strip's vertical filter bar: the control
    /// row acts (the side switch at column 0, the pop-out at column 2)
    /// and a chip row selects its category. The chip layout comes from
    /// the same pure function the renderer used for the current frame
    /// ([`filter_column_layout`], keyed on the drawn bar's height), so
    /// a click can only ever select a chip that is actually drawn.
    ///
    /// The same two gates the column's filter row applies stand here:
    /// a click that lands while this sidebar was not the active pane is
    /// the click that focuses the sidebar, not a chip selection, and
    /// rapid clicks are debounced against phantom mouse events from
    /// tmux pane resize/layout changes.
    pub fn handle_filter_column_click(&mut self, row: u16, col: u16) {
        if !self.filter_click_allowed() {
            return;
        }
        let Some(strip) = self.layout.strip else {
            return;
        };
        let (chips, control_row) = filter_column_layout(self.status_counts(), strip.filter.height);
        if control_row.is_some_and(|control_row| row == control_row) {
            // The drawn control row: the side switch first (a no-op
            // while the sidebar floats; the dimmed glyph says so before
            // the click), then the pop-out one column in.
            if col == 0 {
                self.switch_sidebar_side();
            } else if col == 2 {
                self.toggle_popout();
            }
            return;
        }
        // On the vertical bar a chip's `start` is its row.
        for chip in &chips {
            if chip.start == row {
                self.global.status_filter = chip.filter;
                self.global.save_filter();
                self.rebuild_row_targets();
                return;
            }
        }
    }

    /// The strip twin of [`Self::handle_mouse_click`] for a press in
    /// the drawn agent panel. The panel has no filter row, so the row
    /// offsets differ: `agents.y` is the secondary header, every row
    /// below it a list row. The popups and the `+`/`×` click targets are
    /// tested first, in absolute frame coordinates, exactly as the
    /// column's handler does.
    fn handle_strip_agent_click(&mut self, row: u16, col: u16, agents: Rect) {
        if self.handle_notices_popup_click(row, col) {
            return;
        }
        if self.handle_repo_popup_click(row, col) {
            return;
        }
        if self.handle_spawn_input_click(row, col) {
            return;
        }
        if self.handle_remove_confirm_click(row, col) {
            return;
        }
        if self.handle_agent_row_click(row, col) {
            return;
        }
        if row == agents.y {
            self.handle_secondary_header_click(col - agents.x);
            return;
        }
        // A row below the header is a list row: the first one sits
        // directly under the header (no filter row in a strip), offset
        // by the scroll the last frame drew with.
        let line_index = (row - agents.y - 1) as usize + self.scrolls.panes.offset;
        if let Some(Some(agent_row)) = self.layout.line_to_row.get(line_index) {
            self.global.selected_pane_row = *agent_row;
            self.global.queue_cursor_save();
            self.activate_selected_pane();
        }
    }

    /// Track a held-button move (`Drag(Left)`) at `row`/`col` during a
    /// split drag, on the axis the drag's press started. A column drag
    /// follows the mouse row: a move to another row marks the drag moved
    /// and resizes the drawn split to the mouse row, clamped to the
    /// split minimums (and the floating limit while the sidebar
    /// floats); over the rule row itself the drag stays the unmoved
    /// press a release treats as a click. A strip drag follows the
    /// mouse column the same way: a move off the drawn divider marks
    /// the drag moved and resizes the drawn split to the tab columns
    /// the mouse column leaves, clamped to the two panel minimums, and
    /// the renderer draws exactly this much for as long as the drag
    /// moves; over the drawn divider itself the drag stays the unmoved
    /// press. No drag: nothing to move.
    pub fn handle_split_drag_move(&mut self, row: u16, col: u16, term_height: u16) {
        let Some(drag) = &mut self.split_drag else {
            return;
        };
        match drag.axis {
            LayoutKind::Column => {
                // Against the row the rule is drawn at: while unmoved
                // `size` still holds the drawn rows at the press, while
                // moved the renderer draws `size` — in both states the
                // rule sits at `term_height - size`.
                if row == term_height.saturating_sub(drag.size) {
                    return;
                }
                drag.moved = true;
                drag.size = clamp_drag_rows(
                    term_height.saturating_sub(row),
                    term_height,
                    self.sidebar_pane.floating,
                );
            }
            LayoutKind::Strip => {
                // Against the divider the last frame drew: while unmoved
                // that is the press's own divider, while moved the
                // renderer lays the strip out from `size` — in both
                // states the divider sits on the columns the mouse is
                // still over.
                let Some(strip) = self.layout.strip else {
                    return;
                };
                if strip
                    .divider
                    .is_some_and(|divider| point_in_rect(row, col, divider))
                {
                    return;
                }
                drag.moved = true;
                let available = drawn_strip_width(&strip) - strip.filter.width;
                drag.size =
                    clamp_drag_cols(drawn_strip_width(&strip).saturating_sub(col), available);
            }
        }
    }

    /// Finish a split drag on a button release. A drag that moved saves
    /// its size as the shared percent of its axis (~a tmux write per
    /// drag, not per move); an unmoved press resolves as a click and
    /// selects the tab the press names — by column on a column's rule,
    /// and likewise by column on a strip's footer, where a press on
    /// the thin divider line instead resolves to nothing. No drag:
    /// the release belongs to nothing.
    pub fn end_split_drag(&mut self, term_height: u16) {
        let Some(drag) = self.split_drag.take() else {
            return;
        };
        if drag.moved {
            self.save_split_drag(drag, term_height);
        } else {
            match drag.axis {
                LayoutKind::Column => self.handle_bottom_tab_click(drag.press_col),
                LayoutKind::Strip => {
                    // Only a footer press resolves to a tab click: the
                    // glyphs live on the footer row now, so a held
                    // press released on the thin divider line above
                    // the footer changes nothing.
                    if self
                        .layout
                        .strip
                        .and_then(|strip| strip.footer)
                        .is_some_and(|footer| point_in_rect(drag.press_row, drag.press_col, footer))
                    {
                        self.handle_bottom_tab_click(drag.press_col);
                    }
                }
            }
        }
    }

    /// The moved branch of a split-drag ending: adopt the drag's size
    /// as the shared percent of its axis and hand it to the tmux write
    /// — `@sidebar_bottom_height` for a column (the size is rows),
    /// `@sidebar_tab_width` for a strip (the size is tab columns, and
    /// the percent is of the drawn strip's width). Both endings share
    /// it verbatim — the real release (`end_split_drag`) and the
    /// stale-drag end on a new press — and each caller gates on
    /// `drag.moved`: it runs only for a drag that moved.
    fn save_split_drag(&mut self, drag: SplitDrag, term_height: u16) {
        match drag.axis {
            LayoutKind::Column => {
                self.global.bottom_percent = percent_for_rows(drag.size, term_height);
                self.global.save_bottom_percent();
            }
            LayoutKind::Strip => {
                // The strip width the last frame drew: the drag's tab
                // columns are a percent of it, divider included.
                let Some(strip) = self.layout.strip else {
                    return;
                };
                let width = drawn_strip_width(&strip);
                self.global.tab_percent = cols_for_percent(drag.size, width);
                self.global.save_tab_percent();
            }
        }
    }

    /// Handle mouse scroll event. A column routes by row as before: at
    /// or under the bottom panel's first drawn row the wheel scrolls
    /// the displayed tab, above it the agent list. A strip routes by
    /// column over the drawn [`StripLayout`]: the wheel over the agent
    /// panel scrolls the agent list and over the tab panel the
    /// displayed tab; the filter bar and the divider have nothing to
    /// scroll.
    pub fn handle_mouse_scroll(
        &mut self,
        row: u16,
        col: u16,
        term_height: u16,
        bottom_panel_height: u16,
        delta: isize,
    ) {
        match self.layout_kind {
            LayoutKind::Strip => {
                let Some(strip) = self.layout.strip else {
                    return;
                };
                if strip.tabs.is_some_and(|tabs| point_in_rect(row, col, tabs)) {
                    self.scroll_bottom(delta);
                } else if point_in_rect(row, col, strip.agents) {
                    self.scrolls.panes.scroll(delta);
                }
            }
            LayoutKind::Column => {
                let bottom_start = term_height.saturating_sub(bottom_panel_height);
                if row >= bottom_start {
                    self.scroll_bottom(delta);
                } else {
                    self.scrolls.panes.scroll(delta);
                }
            }
        }
    }

    /// The two gates both filter-click handlers share (the column's
    /// horizontal row and the strip's vertical bar): the click must
    /// come from an already-active sidebar and pass the phantom-event
    /// debounce. `true` when the click may act; the debouncer is
    /// restamped as a side effect.
    fn filter_click_allowed(&mut self) -> bool {
        if !self.sidebar_pane.pane_active {
            return false;
        }
        const DEBOUNCE_MS: u128 = 150;
        let now = std::time::Instant::now();
        if now
            .duration_since(self.timers.last_filter_click)
            .as_millis()
            < DEBOUNCE_MS
        {
            return false;
        }
        self.timers.last_filter_click = now;
        true
    }

    /// Handle mouse click on the filter bar (row 0).
    /// Determines which filter was clicked based on x coordinate.
    /// Debounces rapid clicks to ignore phantom mouse events from tmux
    /// pane resize/layout changes.
    ///
    /// A click that lands on the filter bar while this sidebar was not
    /// the active pane is the click that focuses the sidebar, not a chip
    /// selection: tmux focuses the pane it delivers the click to, so the
    /// event arrives even though the user only meant to reach the pane.
    /// Such a click is ignored; the status filter is a server-wide
    /// setting, so honouring it here would change every window.
    pub fn handle_filter_click(&mut self, col: u16) {
        if !self.filter_click_allowed() {
            return;
        }

        // The chip walk of the column's filter row: the row's chip
        // layout comes from the same pure function the
        // renderer used for the current frame, keyed on the width that
        // frame rendered with. Both controls are hit-tested before the
        // chip walk, so a click on either never changes the status
        // filter.
        let (chips, switch_col, popout_col) =
            filter_bar_layout(self.status_counts(), self.layout.filter_bar_width);
        if let Some(popout_col) = popout_col
            && col == popout_col
        {
            self.toggle_popout();
            return;
        }
        if let Some(switch_col) = switch_col
            && col == switch_col
        {
            // A no-op while the sidebar floats; the dimmed glyph says
            // so before the click.
            self.switch_sidebar_side();
            return;
        }
        for chip in &chips {
            if col >= chip.start && col < chip.start + chip.width {
                self.global.status_filter = chip.filter;
                self.global.save_filter();
                self.rebuild_row_targets();
                return;
            }
        }
    }

    /// Handle mouse click on the secondary header row (row 1).
    /// The repo filter button lives on the far right of this row.
    pub fn handle_secondary_header_click(&mut self, col: u16) {
        if self
            .notices
            .button_col
            .is_some_and(|notices_col| col == notices_col)
        {
            self.toggle_notices_popup();
            return;
        }
        if self
            .layout
            .repo_button_col
            .is_some_and(|repo_button_col| col >= repo_button_col)
        {
            self.toggle_repo_popup();
        }
    }

    /// Handle mouse click in agents panel. Maps screen row to agent row
    /// via line_to_row (adjusted for scroll offset) and activates that pane.
    /// Row 0 is the fixed filter bar, row 1+ maps to the scrollable agent list.
    pub fn handle_mouse_click(&mut self, row: u16, col: u16) {
        if self.handle_notices_popup_click(row, col) {
            return;
        }
        if self.handle_repo_popup_click(row, col) {
            return;
        }
        if self.handle_spawn_input_click(row, col) {
            return;
        }
        if self.handle_remove_confirm_click(row, col) {
            return;
        }
        if row == 0 {
            self.handle_filter_click(col);
            return;
        }
        if row == 1 {
            self.handle_secondary_header_click(col);
            return;
        }
        if self.handle_agent_row_click(row, col) {
            return;
        }
        let line_index = (row as usize - 2) + self.scrolls.panes.offset;
        if let Some(Some(agent_row)) = self.layout.line_to_row.get(line_index) {
            self.global.selected_pane_row = *agent_row;
            self.global.queue_cursor_save();
            self.activate_selected_pane();
        }
    }

    /// The notices-popup branch of [`Self::handle_mouse_click`]: a click
    /// inside the popup copies the clicked agent name, a click outside
    /// closes the popup. `false` when the popup is not open.
    fn handle_notices_popup_click(&mut self, row: u16, col: u16) -> bool {
        if !self.is_notices_popup_open() {
            return false;
        }
        if let Some(area) = self.notices_popup_area()
            && point_in_rect(row, col, area)
        {
            if let Some(agent) = self.notices_copy_target_at(row, col).map(str::to_string) {
                self.copy_notices_prompt(&agent);
            }
            return true;
        }
        self.close_notices_popup();
        true
    }

    /// The repo-popup branch of [`Self::handle_mouse_click`]: a click
    /// inside the popup selects and confirms the clicked repo, a click
    /// outside closes the popup. `false` when the popup is not open.
    fn handle_repo_popup_click(&mut self, row: u16, col: u16) -> bool {
        if !self.is_repo_popup_open() {
            return false;
        }
        if let Some(area) = self.repo_popup_area()
            && point_in_rect(row, col, area)
        {
            // Skip clicks on the popup chrome (top border / title row).
            // Without this guard `saturating_sub(1)` collapses a click on
            // the title row into `item_index == 0`, switching the filter
            // to the first repo the moment the user reaches for the
            // popup.
            if row > area.y {
                let item_index = (row - area.y - 1) as usize;
                if item_index < self.repo_names().len() {
                    self.set_repo_popup_selected(item_index);
                    self.confirm_repo_popup();
                }
            }
            return true;
        }
        self.close_repo_popup();
        true
    }

    /// The spawn-modal branch of [`Self::handle_mouse_click`]: a click
    /// inside the modal is swallowed by it, a click outside dismisses
    /// it. `false` when the modal is not open.
    fn handle_spawn_input_click(&mut self, row: u16, col: u16) -> bool {
        if !self.is_spawn_input_open() {
            return false;
        }
        if let Some(area) = self.spawn_input_popup_area()
            && point_in_rect(row, col, area)
        {
            return true;
        }
        self.close_spawn_input();
        true
    }

    /// The remove-confirmation branch of [`Self::handle_mouse_click`]:
    /// a click inside the dialog is swallowed by it, a click outside
    /// dismisses it. `false` when the dialog is not open.
    fn handle_remove_confirm_click(&mut self, row: u16, col: u16) -> bool {
        if !self.is_remove_confirm_open() {
            return false;
        }
        if let Some(area) = self.remove_confirm_popup_area()
            && point_in_rect(row, col, area)
        {
            return true;
        }
        self.close_remove_confirm();
        true
    }

    /// The agent-list branch of [`Self::handle_mouse_click`]: the `+`
    /// spawn button and the red `×` remove marker win before the
    /// body-row fallback so a click on a button never also shifts the
    /// pane selection. `false` when no control was hit.
    fn handle_agent_row_click(&mut self, row: u16, col: u16) -> bool {
        // Check the `+` spawn buttons before the pane-row fallback so a
        // click on the button doesn't also shift the pane selection.
        if let Some((repo_name, repo_root, anchor_y)) = self
            .layout
            .repo_spawn_targets
            .iter()
            .find(|t| point_in_rect(row, col, t.rect))
            .map(|t| (t.repo_name.clone(), t.repo_root.clone(), t.rect.y))
        {
            self.open_spawn_input_for_repo(repo_name, repo_root, Some(anchor_y));
            return true;
        }

        // Check the red `×` remove markers next to spawn-created branches.
        if let Some(pane_id) = self
            .layout
            .spawn_remove_targets
            .iter()
            .find(|t| point_in_rect(row, col, t.rect))
            .map(|t| t.pane_id.clone())
        {
            self.open_remove_confirm_for_pane(pane_id);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{PaneInfo, PaneStatus};

    // The numbers are the spec's own scenarios, not free-standing
    // arithmetic: each one pins a documented split case.

    #[test]
    fn split_rows_draws_the_rounded_percent() {
        assert_eq!(split_rows(30, 40), 12, "30% of 40 rows is 12");
        assert_eq!(split_rows(30, 60), 18, "30% of 60 rows is 18");
    }

    #[test]
    fn split_rows_clamps_to_the_minimum_heights() {
        assert_eq!(
            split_rows(1, 40),
            3,
            "a small percent keeps the panel at its 3-row minimum"
        );
        assert_eq!(
            split_rows(100, 40),
            37,
            "a large percent keeps the agent list at its 3-row minimum"
        );
    }

    #[test]
    fn split_rows_hides_the_panel() {
        assert_eq!(split_rows(0, 40), 0, "a percent of 0 hides the panel");
        assert_eq!(
            split_rows(30, 5),
            0,
            "a sidebar shorter than 6 rows has no bottom panel"
        );
    }

    #[test]
    fn percent_for_rows_rounds_to_the_nearest_percent() {
        assert_eq!(percent_for_rows(20, 40), 50, "20 rows of 40 is 50%");
    }

    #[test]
    fn percent_round_trip_returns_the_same_rows() {
        assert_eq!(
            split_rows(percent_for_rows(37, 40), 40),
            37,
            "37 rows of 40 must give 37 rows again"
        );
    }

    #[test]
    fn the_environment_value_names_the_layout() {
        assert_eq!(
            LayoutKind::from_env_value(Some("strip")),
            LayoutKind::Strip,
            "strip makes a strip"
        );
        assert_eq!(
            LayoutKind::from_env_value(Some("column")),
            LayoutKind::Column,
            "column makes a column"
        );
    }

    #[test]
    fn an_unknown_or_missing_environment_value_makes_a_column() {
        // A sidebar that started before the strip layout existed has no
        // value at all, and a garbage value names no layout: both keep
        // the column behavior.
        for value in [None, Some(""), Some("Strip"), Some("vertical")] {
            assert_eq!(
                LayoutKind::from_env_value(value),
                LayoutKind::Column,
                "the value {value:?} must fall back to a column"
            );
        }
    }

    // ─── strip_layout ─────────────────────────────────────────

    #[test]
    fn strip_layout_draws_the_rounded_tab_percent() {
        // The spec's parts scenario: 120 columns, 40 percent — the
        // filter bar, then the agent panel, then the divider line on
        // column 72, then the tab panel to the right edge — with the
        // footer tab bar on the tab section's bottom row.
        let layout = strip_layout(120, 12, 40);
        assert_eq!(layout.filter, Rect::new(0, 0, 4, 12));
        assert_eq!(layout.agents, Rect::new(4, 0, 68, 12));
        assert_eq!(layout.divider, Some(Rect::new(72, 0, 1, 12)));
        assert_eq!(layout.tabs, Some(Rect::new(73, 0, 47, 12)));
        assert_eq!(layout.footer, Some(Rect::new(72, 11, 48, 1)));
    }

    #[test]
    fn strip_layout_clamps_to_the_minimum_panel_widths() {
        // One percent rounds to a single tab column, clamped up to the
        // 20-column minimum the tab panel keeps with its divider.
        let small = strip_layout(120, 12, 1);
        assert_eq!(small.agents, Rect::new(4, 0, 96, 12));
        assert_eq!(small.divider, Some(Rect::new(100, 0, 1, 12)));
        assert_eq!(small.tabs, Some(Rect::new(101, 0, 19, 12)));
        // A hundred percent is clamped down so the agent panel keeps
        // its own 20-column minimum.
        let large = strip_layout(120, 12, 100);
        assert_eq!(large.agents, Rect::new(4, 0, 20, 12));
        assert_eq!(large.divider, Some(Rect::new(24, 0, 1, 12)));
        assert_eq!(large.tabs, Some(Rect::new(25, 0, 95, 12)));
    }

    #[test]
    fn strip_layout_hides_the_tab_panel() {
        // 40 columns cannot hold the filter bar and both 20-column
        // minimums: no divider, no tab panel, the agent panel takes
        // the columns the filter bar leaves.
        let narrow = strip_layout(40, 12, 40);
        assert_eq!(narrow.filter, Rect::new(0, 0, 4, 12));
        assert_eq!(narrow.agents, Rect::new(4, 0, 36, 12));
        assert_eq!(narrow.divider, None);
        assert_eq!(narrow.tabs, None);
        // A tab width of 0 hides the divider and the tab panel at any
        // width.
        let zero = strip_layout(120, 12, 0);
        assert_eq!(zero.divider, None);
        assert_eq!(zero.tabs, None);
        assert_eq!(zero.agents, Rect::new(4, 0, 116, 12));
    }

    #[test]
    fn the_drag_round_trip_returns_the_same_divider() {
        // The spec's drag scenario: a 120-column strip dragged so the
        // divider lands on column 60. The release converts the tab
        // columns the drag holds to 50 percent, and re-laying the strip
        // out from that percent puts the divider back on column 60.
        let cols = 120u16 - 60;
        assert_eq!(cols_for_percent(cols, 120), 50);
        let relaid = strip_layout(120, 12, cols_for_percent(cols, 120));
        assert_eq!(relaid.divider.map(|rect| rect.x), Some(60));
        assert_eq!(relaid.agents, Rect::new(4, 0, 56, 12));
    }

    // ─── filter_column_layout ──────────────────────────────

    /// A pane of the given status, the minimal shape the counts read.
    fn pane_of_status(status: PaneStatus) -> PaneInfo {
        PaneInfo {
            pane_id: String::new(),
            pane_active: false,
            status,
            attention: false,
            agent: crate::tmux::AgentType::Claude,
            path: String::new(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: crate::tmux::PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: crate::tmux::WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        }
    }

    /// The status counts of `statuses` worth of panes.
    fn counts_of(statuses: &[PaneStatus]) -> StatusCounts {
        let mut state = AppState::new(String::new());
        state.repo_groups = vec![crate::group::RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: statuses
                .iter()
                .enumerate()
                .map(|(i, status)| {
                    (
                        PaneInfo {
                            pane_id: format!("%{i}"),
                            ..pane_of_status(status.clone())
                        },
                        crate::group::PaneGitInfo::default(),
                    )
                })
                .collect(),
        }];
        state.status_counts()
    }

    #[test]
    fn filter_column_layout_draws_one_chip_per_row_from_the_first() {
        // The spec's all-chips-fit scenario: 12 rows put the five
        // chips on rows 0-4 in the variant order and the controls on
        // the last row.
        let counts = counts_of(&[PaneStatus::Running, PaneStatus::Waiting, PaneStatus::Idle]);
        let (chips, control_row) = filter_column_layout(counts, 12);
        assert_eq!(control_row, Some(11));
        let rows: Vec<(StatusFilter, u16)> =
            chips.iter().map(|chip| (chip.filter, chip.start)).collect();
        assert_eq!(
            rows,
            vec![
                (StatusFilter::All, 0),
                (StatusFilter::Busy, 1),
                (StatusFilter::Waiting, 2),
                (StatusFilter::Idle, 3),
                (StatusFilter::Error, 4),
            ]
        );
        // A single-digit count shapes a 3-column chip: icon, space,
        // one digit. The all chip counts 3 panes, the busy chip 1.
        assert_eq!(chips[0].width, 3);
        assert_eq!(chips[1].width, 3);
    }

    #[test]
    fn filter_column_layout_drops_whole_chips_above_the_control_row() {
        // The spec's short-strip scenario: 4 rows leave three rows
        // above the control row, so the idle and error chips are not
        // drawn at all — never partially.
        let counts = counts_of(&[]);
        let (chips, control_row) = filter_column_layout(counts, 4);
        assert_eq!(control_row, Some(3));
        let filters: Vec<StatusFilter> = chips.iter().map(|chip| chip.filter).collect();
        assert_eq!(
            filters,
            vec![StatusFilter::All, StatusFilter::Busy, StatusFilter::Waiting,]
        );
    }

    #[test]
    fn filter_column_layout_on_a_one_row_strip() {
        // A one-row strip cannot hold a chip above the control row:
        // only the control row itself, on row 0.
        let counts = counts_of(&[]);
        let (chips, control_row) = filter_column_layout(counts, 1);
        assert!(chips.is_empty());
        assert_eq!(control_row, Some(0));
    }

    #[test]
    fn filter_column_layout_on_a_zero_row_strip() {
        let counts = counts_of(&[]);
        let (chips, control_row) = filter_column_layout(counts, 0);
        assert!(chips.is_empty());
        assert_eq!(control_row, None);
    }

    #[test]
    fn a_count_of_100_or_more_keeps_the_chip_inside_its_row() {
        // 120 running panes: the all and busy chips count 120, drawn
        // as 99 — two digits, so the chip fills its 4-column row and
        // never overflows into the agent panel.
        let counts = counts_of(&vec![PaneStatus::Running; 120]);
        let (chips, _) = filter_column_layout(counts, 12);
        assert_eq!(chips[0].width, 4);
        assert_eq!(chips[1].width, 4);
        for chip in &chips {
            assert!(chip.width <= 4, "a chip must fit the 4-column bar");
        }
    }
}
