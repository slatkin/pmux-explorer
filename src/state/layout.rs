use super::{AppState, RepoFilter, StatusCounts, StatusFilter};

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
}

pub(super) fn point_in_rect(row: u16, col: u16, rect: ratatui::layout::Rect) -> bool {
    rect.contains(ratatui::layout::Position { x: col, y: row })
}

/// One chip of the status filter row: its category and the columns it
/// occupies on screen (icon plus count digits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterChip {
    pub filter: StatusFilter,
    /// Column of the chip's icon.
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
/// The pop-out control owns the last column of the row, and the chips
/// fill from the left with a single-space separator after one gutter
/// column. A chip that would no longer fit before the control is
/// dropped whole — never partially drawn — together with every chip
/// after it. Returns the chips that fit and the control's column
/// (`None` for a zero-width row, where nothing is drawn).
///
/// Both the renderer and the click hit test run through this function,
/// so a click can only ever select a chip that is actually drawn.
pub(crate) fn filter_bar_layout(
    counts: StatusCounts,
    width: u16,
) -> (Vec<FilterChip>, Option<u16>) {
    if width == 0 {
        return (Vec::new(), None);
    }
    let control_col = width - 1;
    let mut chips = Vec::new();
    let mut start: u16 = 1; // leading gutter column
    for filter in StatusFilter::VARIANTS {
        let chip_width = 1 + count_digits(counts.count(filter));
        if start + chip_width > control_col {
            break;
        }
        chips.push(FilterChip {
            filter,
            start,
            width: chip_width,
        });
        start += chip_width + 1; // single-space separator
    }
    (chips, Some(control_col))
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

    /// Handle a left mouse click at (`row`, `col`) in a terminal
    /// `term_height` rows tall. Clicks on the bottom panel's tab row (the
    /// panel's first drawn row) go to the tab header, clicks above it to
    /// the agents panel. Returns `true` when the click landed on the tab
    /// row, so the caller can keep the background git poller in sync with
    /// a possible tab switch.
    ///
    /// The bottom-panel height comes from [`FrameLayout`], the height the
    /// last frame actually drew — while floating that is the clamped
    /// height — so a click can only ever land on a row that was drawn.
    pub fn handle_terminal_click(&mut self, row: u16, col: u16, term_height: u16) -> bool {
        let bottom_start = term_height.saturating_sub(self.layout.bottom_panel_height);
        if row < bottom_start {
            self.handle_mouse_click(row, col);
            false
        } else if row == bottom_start {
            self.handle_bottom_tab_click(col);
            true
        } else {
            false
        }
    }

    /// Handle mouse scroll event, routing to agents or bottom panel based on Y position.
    pub fn handle_mouse_scroll(
        &mut self,
        row: u16,
        term_height: u16,
        bottom_panel_height: u16,
        delta: isize,
    ) {
        let bottom_start = term_height.saturating_sub(bottom_panel_height);
        if row >= bottom_start {
            self.scroll_bottom(delta);
        } else {
            self.scrolls.panes.scroll(delta);
        }
    }

    /// Handle mouse click on the filter bar (row 0).
    /// Determines which filter was clicked based on x coordinate.
    /// Debounces rapid clicks to ignore phantom mouse events from tmux
    /// pane resize/layout changes.
    pub fn handle_filter_click(&mut self, col: u16) {
        const DEBOUNCE_MS: u128 = 150;
        let now = std::time::Instant::now();
        if now
            .duration_since(self.timers.last_filter_click)
            .as_millis()
            < DEBOUNCE_MS
        {
            return;
        }
        self.timers.last_filter_click = now;

        // The row's chip layout comes from the same pure function the
        // renderer used for the current frame, keyed on the width that
        // frame rendered with. The pop-out control is hit-tested before
        // the chip walk, so a click on it never changes the status
        // filter.
        let (chips, control_col) =
            filter_bar_layout(self.status_counts(), self.layout.filter_bar_width);
        if let Some(control_col) = control_col
            && col == control_col
        {
            self.toggle_popout();
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
        if self.is_notices_popup_open() {
            if let Some(area) = self.notices_popup_area()
                && point_in_rect(row, col, area)
            {
                if let Some(agent) = self.notices_copy_target_at(row, col).map(str::to_string) {
                    self.copy_notices_prompt(&agent);
                }
                return;
            }
            self.close_notices_popup();
            return;
        }
        if self.is_repo_popup_open() {
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
                return;
            }
            self.close_repo_popup();
            return;
        }
        if self.is_spawn_input_open() {
            if let Some(area) = self.spawn_input_popup_area()
                && point_in_rect(row, col, area)
            {
                return;
            }
            self.close_spawn_input();
            return;
        }
        if self.is_remove_confirm_open() {
            if let Some(area) = self.remove_confirm_popup_area()
                && point_in_rect(row, col, area)
            {
                return;
            }
            self.close_remove_confirm();
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
            return;
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
            return;
        }

        let line_index = (row as usize - 2) + self.scrolls.panes.offset;
        if let Some(Some(agent_row)) = self.layout.line_to_row.get(line_index) {
            self.global.selected_pane_row = *agent_row;
            self.global.queue_cursor_save();
            self.activate_selected_pane();
        }
    }
}
