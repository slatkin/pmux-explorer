use std::collections::HashMap;
use std::time::Instant;

use crate::tmux;
use crate::ui;

use super::BottomTab;
use super::LayoutKind;
use super::filter::{RepoFilter, StatusFilter};

/// One pending-write latch for a value shared with tmux.
///
/// The shared options are read back from tmux on every refresh tick
/// (`GlobalState::apply_all`), so a value a write has not reached yet
/// must not be adopted over the user's choice. A latch therefore
/// remembers the last value tmux successfully accepted (`last_saved`)
/// and whether the last write was refused (`save_pending`). While the
/// latch is pending, tmux holds the stale value this instance is
/// trying to replace, so adoption skips the field until the write
/// lands; adoption also skips a shared value equal to `last_saved`,
/// so a local change waiting for its write stays put.
///
/// The status filter, the bottom tab, the bottom percent, the strip
/// tab percent, the two split tabs, the two split widths, the width
/// lock and the shared docked width each hold one `SaveLatch`
/// instance, so the save/retry/adopt mechanics below is written once
/// and the ten shared fields share it. The type and its fields are
/// `pub(crate)` because the width latch's pending flag also gates the
/// width guard's adopt side (`app::width_guard`, design D4) and
/// sibling-module tests drive the latch contract.
pub(crate) struct SaveLatch<T> {
    /// The global option this latch's `set -g` writes go to.
    option: &'static str,
    /// Last value successfully written to tmux.
    pub(crate) last_saved: T,
    /// Whether a write tmux rejected is still being retried.
    pub(crate) save_pending: bool,
}

impl<T: Copy + PartialEq> SaveLatch<T> {
    /// A fresh latch: guarded at `last_saved`, nothing pending.
    fn new(option: &'static str, last_saved: T) -> Self {
        Self {
            option,
            last_saved,
            save_pending: false,
        }
    }

    /// Write `arg` — `value` serialized for the option — to tmux. Only
    /// a write tmux accepts advances `last_saved` and clears the
    /// latch, so a failed write does not cause sync to overwrite the
    /// user's choice with the stale value it tried to replace; a
    /// rejected write sets the latch so the next refresh tick retries
    /// it.
    fn save(&mut self, value: T, arg: &str) {
        if tmux::run_tmux(&["set", "-g", self.option, arg]).is_some() {
            self.last_saved = value;
            self.save_pending = false;
        } else {
            self.save_pending = true;
        }
    }

    /// Retry a write tmux rejected with the field's current value, so
    /// an instance cannot drift from the shared value and later have
    /// its stale choice adopted by (or pushed onto) every other
    /// sidebar. A retry tmux rejects again keeps the latch set for the
    /// next tick. Runs once per refresh tick, before the tick reads
    /// the shared options, so a successful retry is what that tick
    /// reads back.
    fn retry(&mut self, value: T, arg: &str) {
        if self.save_pending {
            self.save(value, arg);
        }
    }

    /// Adopt the shared value, or leave everything untouched (`None`).
    /// The gate runs once here: a write still pending blocks adoption,
    /// the caller's field gate holds it back (`allowed` — for the
    /// percent a live split drag, for the label fields nothing), and a
    /// parse that misses (`None`) adopts nothing. A parsed value equal
    /// to `last_saved` is the one this instance wrote and must not
    /// overwrite a local change whose write is on the wire. The
    /// adopted value returns to the caller, which writes it into the
    /// live field.
    ///
    /// The `parse` closure carries the field's parse semantics: the
    /// label fields look a single option up and return `None` when it
    /// is absent (a missing key adopts nothing), while the percent
    /// parses the whole map with a default so an absent, empty or
    /// invalid option resolves to `DEFAULT_BOTTOM_PERCENT` and is
    /// adopted.
    fn adopt(
        &mut self,
        opts: &HashMap<String, String>,
        allowed: bool,
        parse: impl FnOnce(&HashMap<String, String>) -> Option<T>,
    ) -> Option<T> {
        if !self.save_pending
            && allowed
            && let Some(tmux_value) = parse(opts)
            && tmux_value != self.last_saved
        {
            self.last_saved = tmux_value;
            return Some(tmux_value);
        }
        None
    }
}

/// Read the shared cursor option as the pane it names.
///
/// The cursor travels as a tmux pane id (`%3`). A value that is not one
/// — a bare row index written by a version that shared the cursor as a
/// position, or garbage from a hand-edited option — names no pane and
/// adopts nothing, so the cursor resolves from the tmux-focused pane
/// instead of landing on whatever row that number happens to be.
pub(crate) fn parse_shared_cursor(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let digits = raw.strip_prefix('%')?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| raw.to_string())
}

/// State shared across all sidebar instances via tmux global variables.
/// Read once at startup and re-applied from the shared options on every
/// refresh tick.
pub struct GlobalState {
    pub status_filter: StatusFilter,
    pub selected_pane_row: usize,
    pub repo_filter: RepoFilter,
    /// The bottom panel tab, shared like the filters: written with
    /// `set -g @sidebar_bottom_tab` and adopted from that option.
    pub bottom_tab: BottomTab,
    /// The bottom panel height as a percent of the sidebar height,
    /// shared like the filters: written with
    /// `set -g @sidebar_bottom_height <n>` and adopted from that
    /// option.
    pub bottom_percent: u8,
    /// The strip tab-panel width (divider included) as a percent of
    /// the strip width, shared like the filters: written with
    /// `set -g @sidebar_tab_width <n>` and adopted from that option.
    /// A column never writes or reads it; a strip drag release is its
    /// only writer.
    pub tab_percent: u8,
    /// A split strip's pane A tab, shared like the bottom tab: written
    /// with `set -g @sidebar_split_tab_a` and adopted from that
    /// option. Only pane A's group (`activity` / `git`) is selectable.
    pub split_tab_a: BottomTab,
    /// A split strip's pane B tab, shared like the bottom tab: written
    /// with `set -g @sidebar_split_tab_b` and adopted from that
    /// option. Only pane B's group (`tasks` / `thought`) is selectable.
    pub split_tab_b: BottomTab,
    /// The split strip's agent panel width as a percent of the strip
    /// width, shared like the strip tab percent: written with
    /// `set -g @sidebar_split_agents_width <n>` and adopted from that
    /// option. A split divider 1 drag's release is its only writer.
    pub split_agents_percent: u8,
    /// Pane B's width with divider 2 as a percent of the strip width,
    /// shared like the strip tab percent: written with
    /// `set -g @sidebar_split_tab_b_width <n>` and adopted from that
    /// option. A split divider 2 drag's release is its only writer.
    pub split_tab_b_percent: u8,
    /// On/off switch for the shared docked width; mirrored to and
    /// adopted from the `@sidebar_width_lock` tmux option.
    pub width_lock: bool,
    /// The docked width every sidebar converges on while the lock is
    /// on (`None` until one exists); mirrored to and adopted from the
    /// `@sidebar_docked_width` tmux option. `shared_` because
    /// `AppState.docked_width` already names the pop-out capture.
    pub shared_docked_width: Option<u16>,
    /// Parsed `@sidebar_min_width` as carried on the per-tick reading,
    /// `None` when the option is unset or not numeric. Adopted as-is
    /// every tick with no guard or latch: no sidebar instance ever
    /// writes the option, so there is no local value to protect, and
    /// the width-lock floor falls back to `DEFAULT_MIN_WIDTH` when
    /// unset.
    pub min_width: Option<u32>,
    /// Parsed `@sidebar_max_width` as carried on the same reading —
    /// the band's ceiling, adopted unconditionally like `min_width`;
    /// `None` when the option is unset or not numeric.
    pub max_width: Option<u32>,
    /// Parsed `@sidebar_tab_split_width` as carried on the per-tick
    /// reading: the strip width at or above which a strip shows the
    /// split layout, `0` turning the split layout off. Adopted
    /// unconditionally like `min_width`/`max_width` — no sidebar
    /// instance ever writes the option (it is configuration, not a
    /// shared view setting, so it keeps no save latch — design D9).
    pub tab_split_width: u16,
    /// The status filter's save latch (see `SaveLatch` for the whole
    /// latch rationale; the bottom-tab and bottom-percent latches
    /// below are further instances of the same mechanism).
    filter_latch: SaveLatch<StatusFilter>,
    /// The bottom tab's save latch, written to and adopted from
    /// `@sidebar_bottom_tab`.
    tab_latch: SaveLatch<BottomTab>,
    /// The bottom percent's save latch, written to and adopted from
    /// `@sidebar_bottom_height`.
    percent_latch: SaveLatch<u8>,
    /// The strip tab percent's save latch, written to and adopted from
    /// `@sidebar_tab_width`.
    tab_percent_latch: SaveLatch<u8>,
    /// Pane A's split tab latch, written to and adopted from
    /// `@sidebar_split_tab_a`.
    split_tab_a_latch: SaveLatch<BottomTab>,
    /// Pane B's split tab latch, written to and adopted from
    /// `@sidebar_split_tab_b`.
    split_tab_b_latch: SaveLatch<BottomTab>,
    /// The split agent percent's latch, written to and adopted from
    /// `@sidebar_split_agents_width`.
    split_agents_percent_latch: SaveLatch<u8>,
    /// Pane B's split percent latch, written to and adopted from
    /// `@sidebar_split_tab_b_width`.
    split_tab_b_percent_latch: SaveLatch<u8>,
    /// The lock latch, written to and adopted from
    /// `@sidebar_width_lock`.
    pub(crate) lock_latch: SaveLatch<bool>,
    /// The width latch, written to and adopted from
    /// `@sidebar_docked_width`. While its write is pending the width
    /// guard must also skip its adopt side: the value tmux still
    /// holds is the stale shared width this instance is replacing
    /// (design D4), and adopting it back would resize the pane over
    /// the user's hand.
    pub(crate) width_latch: SaveLatch<Option<u16>>,
    /// The pane the cursor names — the identity behind
    /// `selected_pane_row`. The row is only a cache into the filtered
    /// list and moves when a filter hides a pane, when a repo group
    /// reorders, or when a pane above it closes; this value does not,
    /// so the cursor keeps naming the pane the user picked.
    pub cursor_pane_id: Option<String>,
    /// Last cursor value successfully written to tmux.
    last_saved_cursor: Option<String>,
    /// Last repo filter value successfully written to tmux.
    last_saved_repo_filter: RepoFilter,
    /// When the selected cursor was last changed and still needs persisting.
    pending_cursor_save_since: Option<Instant>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            status_filter: StatusFilter::All,
            selected_pane_row: 0,
            repo_filter: RepoFilter::All,
            bottom_tab: BottomTab::Activity,
            bottom_percent: ui::DEFAULT_BOTTOM_PERCENT,
            tab_percent: ui::DEFAULT_TAB_PERCENT,
            split_tab_a: BottomTab::Activity,
            split_tab_b: BottomTab::Tasks,
            split_agents_percent: ui::DEFAULT_SPLIT_PERCENT,
            split_tab_b_percent: ui::DEFAULT_SPLIT_PERCENT,
            width_lock: false,
            shared_docked_width: None,
            min_width: None,
            max_width: None,
            tab_split_width: ui::DEFAULT_TAB_SPLIT_WIDTH,
            filter_latch: SaveLatch::new(tmux::SIDEBAR_FILTER, StatusFilter::All),
            tab_latch: SaveLatch::new(tmux::SIDEBAR_BOTTOM_TAB, BottomTab::Activity),
            percent_latch: SaveLatch::new(tmux::SIDEBAR_BOTTOM_HEIGHT, ui::DEFAULT_BOTTOM_PERCENT),
            tab_percent_latch: SaveLatch::new(tmux::SIDEBAR_TAB_WIDTH, ui::DEFAULT_TAB_PERCENT),
            split_tab_a_latch: SaveLatch::new(tmux::SIDEBAR_SPLIT_TAB_A, BottomTab::Activity),
            split_tab_b_latch: SaveLatch::new(tmux::SIDEBAR_SPLIT_TAB_B, BottomTab::Tasks),
            split_agents_percent_latch: SaveLatch::new(
                tmux::SIDEBAR_SPLIT_AGENTS_WIDTH,
                ui::DEFAULT_SPLIT_PERCENT,
            ),
            split_tab_b_percent_latch: SaveLatch::new(
                tmux::SIDEBAR_SPLIT_TAB_B_WIDTH,
                ui::DEFAULT_SPLIT_PERCENT,
            ),
            lock_latch: SaveLatch::new(tmux::SIDEBAR_WIDTH_LOCK, false),
            width_latch: SaveLatch::new(tmux::SIDEBAR_DOCKED_WIDTH, None),
            cursor_pane_id: None,
            last_saved_cursor: None,
            last_saved_repo_filter: RepoFilter::All,
            pending_cursor_save_since: None,
        }
    }

    /// Save the status filter to tmux global variable via the filter
    /// latch; see `SaveLatch` for why a rejected write is retried on
    /// the next tick instead of letting the instance drift from the
    /// shared value.
    pub fn save_filter(&mut self) {
        self.filter_latch
            .save(self.status_filter, self.status_filter.as_str());
    }

    /// Save the selected bottom tab to tmux global variable via the
    /// tab latch.
    pub fn save_bottom_tab(&mut self) {
        self.tab_latch
            .save(self.bottom_tab, self.bottom_tab.as_str());
    }

    /// Assign the selected bottom tab and persist it to the shared
    /// tmux option. The only writers of the shared tab (the manual
    /// cycle key, the tab-title click) funnel through here so the
    /// whole server converges within one refresh tick.
    pub fn set_bottom_tab(&mut self, tab: BottomTab) {
        self.bottom_tab = tab;
        self.save_bottom_tab();
    }

    /// Save the bottom panel percent to tmux global variable via the
    /// percent latch.
    pub fn save_bottom_percent(&mut self) {
        self.percent_latch
            .save(self.bottom_percent, &self.bottom_percent.to_string());
    }

    /// Whether the bottom-percent write is waiting for tmux to accept
    /// it. Read-only test window into the private latch: the split-drag
    /// release tests must assert that a release hands the percent to
    /// `save_bottom_percent`, and the latch is exactly what a refused
    /// write leaves behind.
    #[cfg(test)]
    pub(crate) fn percent_save_pending(&self) -> bool {
        self.percent_latch.save_pending
    }

    /// Save the strip tab percent to tmux global variable via its
    /// latch. The strip divider drag's release is the only writer.
    pub fn save_tab_percent(&mut self) {
        self.tab_percent_latch
            .save(self.tab_percent, &self.tab_percent.to_string());
    }

    /// Whether the strip tab-percent write is waiting for tmux to
    /// accept it. Read-only test window into the private latch, the
    /// tab-percent twin of [`Self::percent_save_pending`].
    #[cfg(test)]
    pub(crate) fn tab_percent_save_pending(&self) -> bool {
        self.tab_percent_latch.save_pending
    }

    /// Save pane A's split tab to tmux global variable via its latch.
    /// A split pane A tab selection's only writer.
    pub fn save_split_tab_a(&mut self) {
        self.split_tab_a_latch
            .save(self.split_tab_a, self.split_tab_a.as_str());
    }

    /// Assign pane A's split tab and persist it to the shared tmux
    /// option. The split pane A tab selection's writers (the per-pane
    /// cycle, the per-pane click) funnel through here so the whole
    /// server converges within one refresh tick.
    pub fn set_split_tab_a(&mut self, tab: BottomTab) {
        self.split_tab_a = tab;
        self.save_split_tab_a();
    }

    /// Save pane B's split tab to tmux global variable via its latch.
    /// A split pane B tab selection's only writer.
    pub fn save_split_tab_b(&mut self) {
        self.split_tab_b_latch
            .save(self.split_tab_b, self.split_tab_b.as_str());
    }

    /// Assign pane B's split tab and persist it to the shared tmux
    /// option. The split pane B tab selection's writers (the per-pane
    /// cycle, the per-pane click) funnel through here so the whole
    /// server converges within one refresh tick.
    pub fn set_split_tab_b(&mut self, tab: BottomTab) {
        self.split_tab_b = tab;
        self.save_split_tab_b();
    }

    /// Save the split agent percent to tmux global variable via its
    /// latch. A split divider 1 drag's release is its only writer.
    pub fn save_split_agents_percent(&mut self) {
        self.split_agents_percent_latch.save(
            self.split_agents_percent,
            &self.split_agents_percent.to_string(),
        );
    }

    /// Save pane B's split percent to tmux global variable via its
    /// latch. A split divider 2 drag's release is its only writer.
    pub fn save_split_tab_b_percent(&mut self) {
        self.split_tab_b_percent_latch.save(
            self.split_tab_b_percent,
            &self.split_tab_b_percent.to_string(),
        );
    }

    /// Save the shared docked width to tmux global variable — the
    /// width latch's write; see `SaveLatch` for the accept/reject
    /// contract. Callers (the lock's enabling seed, the width guard's
    /// publish side) assign the desired width to `shared_docked_width`
    /// first, then call this.
    pub fn save_shared_docked_width(&mut self) {
        let Some(width) = self.shared_docked_width else {
            // Production always assigns Some first, so this branch is
            // only reachable from tests: with nothing to publish there
            // is nothing on the wire to reconcile, and a pending flag
            // would block adoption forever. Off the latch path
            // deliberately: clearing the flag alone must not advance
            // `last_saved` to `None` when a width was saved before.
            self.width_latch.save_pending = false;
            return;
        };
        self.width_latch.save(Some(width), &width.to_string());
    }

    /// Save the width lock to tmux global variable — the lock latch's
    /// write; see `SaveLatch` for the accept/reject contract. Toggling
    /// code assigns the intent to `width_lock` first, then calls this.
    pub fn save_width_lock(&mut self) {
        let label = if self.width_lock { "on" } else { "off" };
        self.lock_latch.save(self.width_lock, label);
    }

    /// Save the cursor's pane to tmux global variable. Returns `true`
    /// when tmux accepted the write so callers can decide whether to
    /// clear a queued save or keep retrying; a cursor that names no pane
    /// (an empty agent list) has nothing to write and counts as saved,
    /// so a queued save is not retried forever.
    pub fn save_cursor(&mut self) -> bool {
        let Some(pane_id) = self.cursor_pane_id.clone() else {
            return true;
        };
        if tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_CURSOR, &pane_id]).is_some() {
            self.last_saved_cursor = Some(pane_id);
            true
        } else {
            false
        }
    }

    /// Mark the cursor as dirty so the main loop can persist it once the
    /// user pauses navigation.
    pub fn queue_cursor_save(&mut self) {
        self.pending_cursor_save_since = Some(Instant::now());
    }

    /// Persist a queued cursor update after it has been idle for at least the
    /// requested debounce duration. Returns true when the queue was consumed.
    pub fn flush_pending_cursor_save(&mut self, debounce: std::time::Duration) -> bool {
        let Some(queued_at) = self.pending_cursor_save_since else {
            return false;
        };
        if queued_at.elapsed() < debounce {
            return false;
        }
        // Only clear the pending marker on successful tmux write — otherwise
        // a transient failure would silently drop the queued save instead of
        // retrying on the next flush tick.
        if self.save_cursor() {
            self.pending_cursor_save_since = None;
            true
        } else {
            false
        }
    }

    /// Save repo filter to tmux global variable.
    pub fn save_repo_filter(&mut self) {
        if tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_REPO_FILTER,
            self.repo_filter.as_str(),
        ])
        .is_some()
        {
            self.last_saved_repo_filter = self.repo_filter.clone();
        }
    }

    /// Load all global state from tmux variables.
    /// Called at startup.
    pub fn load_from_tmux(&mut self) {
        let opts = tmux::get_all_global_options();
        // Startup has no live drag: every shared value is applied.
        self.apply_all(&opts, None);
    }

    /// Retry every shared write tmux rejected (filter, bottom tab,
    /// bottom percent, strip tab percent, the two split tabs, the two
    /// split widths, width lock, shared docked width), so an instance
    /// cannot drift from the shared values (see `SaveLatch::retry` for
    /// the latch rationale). Runs once per
    /// refresh tick, before the tick reads the shared options, so a
    /// successful retry is what that tick reads back. The writes must
    /// stay together: retrying only some would let the others drift
    /// and be adopted back by every other sidebar.
    pub fn retry_pending_saves(&mut self) {
        self.filter_latch
            .retry(self.status_filter, self.status_filter.as_str());
        self.tab_latch
            .retry(self.bottom_tab, self.bottom_tab.as_str());
        self.percent_latch
            .retry(self.bottom_percent, &self.bottom_percent.to_string());
        self.tab_percent_latch
            .retry(self.tab_percent, &self.tab_percent.to_string());
        self.split_tab_a_latch
            .retry(self.split_tab_a, self.split_tab_a.as_str());
        self.split_tab_b_latch
            .retry(self.split_tab_b, self.split_tab_b.as_str());
        self.split_agents_percent_latch.retry(
            self.split_agents_percent,
            &self.split_agents_percent.to_string(),
        );
        self.split_tab_b_percent_latch.retry(
            self.split_tab_b_percent,
            &self.split_tab_b_percent.to_string(),
        );
        // The width latch retries through its save method so the
        // `None` guard clears the flag instead of writing an empty
        // option; on the tick a retried width lands, `apply_all`
        // reads the same value back and advances the latch's
        // `last_saved`.
        if self.width_latch.save_pending {
            self.save_shared_docked_width();
        }
        self.lock_latch
            .retry(self.width_lock, if self.width_lock { "on" } else { "off" });
    }

    /// Apply the shared options read from tmux (filter, cursor, bottom
    /// tab, bottom percent, strip tab percent, repo filter, width lock,
    /// shared docked width, width bounds). Runs once at startup and on
    /// every refresh tick.
    ///
    /// `split_drag_axis` says whether the calling instance is mid split
    /// drag and on which axis (`AppState::split_drag`'s axis): the
    /// adoption gates are axis-specific, because each axis's drag moves
    /// only its own panel — a column drag's mouse owns the bottom
    /// panel's height, a strip drag's mouse the divider's column. While
    /// a drag is live the shared value of its own axis must not be
    /// adopted, or the next tick would move the panel out from under
    /// the mouse (the "a drag in progress is not reverted"
    /// requirement); the other axis's value adopts as usual. The
    /// caller passes the gate because the drag lives on `AppState` and
    /// this method is a `GlobalState` method.
    pub fn apply_all(
        &mut self,
        opts: &HashMap<String, String>,
        split_drag_axis: Option<LayoutKind>,
    ) {
        // The shared fields below adopt through their latches;
        // `SaveLatch::adopt` carries the whole adoption gate once
        // (while a write is pending the shared value stays unadopted,
        // and a landed write's new value is taken in like any other
        // shared field), and the closures carry each field's parse
        // semantics.
        if let Some(adopted) = self.filter_latch.adopt(opts, true, |o| {
            o.get(tmux::SIDEBAR_FILTER)
                .map(|raw| StatusFilter::from_label(raw))
        }) {
            self.status_filter = adopted;
        }
        // The cursor is shared as the pane it names, not as a row: each
        // instance's list holds its own rows for the same panes, so a
        // row number would land the peer's cursor on an unrelated pane.
        // The row follows from the id when the targets are rebuilt.
        if let Some(cursor_str) = opts.get(tmux::SIDEBAR_CURSOR)
            && let Some(pane_id) = parse_shared_cursor(cursor_str)
            && self.last_saved_cursor.as_deref() != Some(pane_id.as_str())
            // A local move whose debounced write has not landed yet wins
            // over anything another instance writes during that window;
            // the flush persists the user's pane and the ticks after it
            // adopt from tmux again.
            && self.pending_cursor_save_since.is_none()
        {
            self.last_saved_cursor = Some(pane_id.clone());
            self.cursor_pane_id = Some(pane_id);
        }
        if let Some(adopted) = self.tab_latch.adopt(opts, true, |o| {
            o.get(tmux::SIDEBAR_BOTTOM_TAB)
                .map(|raw| BottomTab::from_label(raw))
        }) {
            self.bottom_tab = adopted;
        }
        // The percent is the latch family with an extra adoption gate
        // and a defaulting parse: while a column split drag is live the
        // rule shows the drag's rows and the mouse owns the height, so
        // the shared percent stays unadopted this tick exactly as for a
        // pending write; after the release the following ticks adopt
        // the server value again. The gate is axis-specific: a strip
        // drag moves the divider, not the rule, so it does not block
        // this value. Unlike the label fields the percent parse has a
        // default, so it parses the whole option map: an unset, empty
        // or invalid option resolves to `DEFAULT_BOTTOM_PERCENT`
        // instead of being skipped. The strip tab percent below carries
        // the mirrored gate and the same defaulting parse against
        // `@sidebar_tab_width`.
        if let Some(adopted) =
            self.percent_latch
                .adopt(opts, split_drag_axis != Some(LayoutKind::Column), |o| {
                    Some(ui::bottom_percent_from_options(o))
                })
        {
            self.bottom_percent = adopted;
        }
        // The strip tab percent shares the split-drag gate on its own
        // axis: while a strip drag is live the strip's divider follows
        // the mouse, so the shared tab width stays unadopted this tick;
        // after the release the following ticks adopt the server value
        // again, and a column drag (which moves no divider) never
        // blocks it. The parse defaults like the bottom percent's, so
        // an unset, empty or invalid `@sidebar_tab_width` resolves to
        // `DEFAULT_TAB_PERCENT` and is adopted.
        if let Some(adopted) =
            self.tab_percent_latch
                .adopt(opts, split_drag_axis != Some(LayoutKind::Strip), |o| {
                    Some(ui::tab_percent_from_options(o))
                })
        {
            self.tab_percent = adopted;
        }
        // The split tabs adopt through the whole-map parse the spec
        // gives them: an unset, empty or foreign-group value selects
        // the pane's default (`activity` for pane A, `tasks` for pane
        // B), so a cleared option falls back server-wide like the
        // percents do. No drag moves a tab, so their adoption carries
        // no drag gate.
        if let Some(adopted) = self
            .split_tab_a_latch
            .adopt(opts, true, |o| Some(ui::split_tab_a_from_options(o)))
        {
            self.split_tab_a = adopted;
        }
        if let Some(adopted) = self
            .split_tab_b_latch
            .adopt(opts, true, |o| Some(ui::split_tab_b_from_options(o)))
        {
            self.split_tab_b = adopted;
        }
        // The two split widths share the strip drag gate with the tab
        // percent: while a strip drag is live the divider follows the
        // mouse, so the shared widths stay unadopted this tick; after
        // the release the following ticks adopt the server values
        // again, and a column drag (which moves no divider) never
        // blocks them. The parse defaults like the other percents, so
        // an unset, empty or out-of-range option resolves to
        // `DEFAULT_SPLIT_PERCENT` and is adopted.
        if let Some(adopted) = self.split_agents_percent_latch.adopt(
            opts,
            split_drag_axis != Some(LayoutKind::Strip),
            |o| Some(ui::split_agents_percent_from_options(o)),
        ) {
            self.split_agents_percent = adopted;
        }
        if let Some(adopted) = self.split_tab_b_percent_latch.adopt(
            opts,
            split_drag_axis != Some(LayoutKind::Strip),
            |o| Some(ui::split_tab_b_percent_from_options(o)),
        ) {
            self.split_tab_b_percent = adopted;
        }
        // The lock's parse defaults to `false`: an absent or cleared
        // `@sidebar_width_lock` reads as unlocked, so `set -gu`
        // releases every sidebar like an explicit "off".
        if let Some(lock) = self.lock_latch.adopt(opts, true, |o| {
            Some(matches!(
                o.get(tmux::SIDEBAR_WIDTH_LOCK).map(String::as_str),
                Some("on")
            ))
        }) {
            self.width_lock = lock;
        }
        // The shared docked width parses like the label fields: a
        // missing key — or junk like `15%` — adopts nothing.
        if let Some(shared) = self.width_latch.adopt(opts, true, |o| {
            o.get(tmux::SIDEBAR_DOCKED_WIDTH)
                .and_then(|s| s.parse::<u16>().ok())
                .map(Some)
        }) {
            self.shared_docked_width = shared;
        }
        // The parsed width bounds need no guard or latch: nothing in
        // the sidebar ever writes `@sidebar_min_width` or
        // `@sidebar_max_width`, so a peer's value can never race a
        // local one — each tick takes tmux's word verbatim. An absent
        // or non-numeric key resets the field to `None`, so clearing
        // the option falls back to `DEFAULT_MIN_WIDTH` / no ceiling
        // at the readers exactly like the per-tick `get_option` this
        // parse replaced.
        // The split threshold parses with a default like the percents:
        // an unset, empty or not-a-whole-number
        // `@sidebar_tab_split_width` resolves to
        // `DEFAULT_TAB_SPLIT_WIDTH`, and `0` stays `0` — the split
        // layout off. Adopted unconditionally: no sidebar instance
        // ever writes the option (design D9), so there is no local
        // value to protect.
        self.tab_split_width = ui::tab_split_width_from_options(opts);
        self.min_width = opts
            .get(tmux::SIDEBAR_MIN_WIDTH)
            .and_then(|s| s.trim().parse::<u32>().ok());
        self.max_width = opts
            .get(tmux::SIDEBAR_MAX_WIDTH)
            .and_then(|s| s.trim().parse::<u32>().ok());
        if let Some(repo_str) = opts.get(tmux::SIDEBAR_REPO_FILTER) {
            let tmux_repo = RepoFilter::from_label(repo_str);
            if tmux_repo != self.last_saved_repo_filter {
                self.repo_filter = tmux_repo.clone();
                self.last_saved_repo_filter = tmux_repo;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn opts(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ─── the shared-value latches (one mechanism, one suite) ────────
    //
    // The status filter, the bottom tab, the bottom percent, the strip
    // tab percent, the two split tabs and the two split widths ride
    // the same `SaveLatch` (the pending-write field, the save, the
    // retry and the adoption), so the latch mechanics are pinned once
    // with one fixture per latch instead of a cloned test per latch
    // per mechanic.
    //
    // In these tests every tmux call runs against a socket no server
    // listens on, so the writes deterministically take the rejection
    // branch.
    //
    // The percent keeps three dedicated tests: its parse adopts the
    // default for an absent, empty or invalid
    // `@sidebar_bottom_height`, and its adoption has the live
    // split-drag gate the other latches do not. The status filter's
    // parse/adopt semantics (unknown labels fall back to `All`) are
    // additionally pinned in `tests/state_tests.rs`.
    struct LatchFixture {
        /// The latch the fixture drives, for the assert messages.
        name: &'static str,
        /// The guard's shared value before the exercise: the value the
        /// fresh construction holds.
        old: &'static str,
        /// The user's local value the fixture changes the live field
        /// to, serialized for the option.
        local: &'static str,
        /// A third value a peer instance might share, serialized.
        peer: &'static str,
        /// The live field's constructed default, serialized.
        default: &'static str,
        /// Move the guard to `old` and the live field to `local`,
        /// nothing pending: the user changed it locally and no write
        /// has been handed to tmux yet.
        hold: fn(&mut GlobalState),
        /// Flag the write pending: tmux refused it and the retry tick
        /// has not landed it yet.
        set_pending: fn(&mut GlobalState),
        /// Hand the live value to the latch's `set -g` write.
        save: fn(&mut GlobalState),
        /// Run the shared retry tick (`retry_pending_saves`) with only
        /// this fixture's latch pending.
        retry: fn(&mut GlobalState),
        /// One adoption tick carrying only this latch's shared option
        /// at `value`, no drag live.
        apply_shared: fn(&mut GlobalState, value: &str),
        /// One adoption tick whose shared reading has other keys but
        /// not this latch's.
        apply_other: fn(&mut GlobalState),
        /// Whether this latch's write is pending.
        pending: fn(&GlobalState) -> bool,
        /// All eight latches' pendings in table order; the
        /// independence test skips this fixture's own `index`.
        pendings: fn(&GlobalState) -> Vec<bool>,
        /// This fixture's index in the table (and in `pendings`).
        index: usize,
        /// The guard (`last_saved`), serialized like the write.
        guard: fn(&GlobalState) -> String,
        /// The live field value, serialized like the write.
        live: fn(&GlobalState) -> String,
    }

    fn fixtures() -> [LatchFixture; 8] {
        [
            LatchFixture {
                name: "status filter",
                old: StatusFilter::All.as_str(),
                local: StatusFilter::Busy.as_str(),
                peer: StatusFilter::Waiting.as_str(),
                default: StatusFilter::All.as_str(),
                hold: hold_filter,
                set_pending: flag_filter_pending,
                save: save_filter,
                retry: retry_all,
                apply_shared: apply_filter,
                apply_other: apply_tab_key,
                pending: |g| g.filter_latch.save_pending,
                pendings: all_pendings,
                index: 0,
                guard: |g| g.filter_latch.last_saved.as_str().to_string(),
                live: |g| g.status_filter.as_str().to_string(),
            },
            LatchFixture {
                name: "bottom tab",
                old: BottomTab::Activity.as_str(),
                local: BottomTab::Tasks.as_str(),
                peer: BottomTab::GitStatus.as_str(),
                default: BottomTab::Activity.as_str(),
                hold: hold_tab,
                set_pending: flag_tab_pending,
                save: save_bottom_tab,
                retry: retry_all,
                apply_shared: apply_tab,
                apply_other: apply_filter_key,
                pending: |g| g.tab_latch.save_pending,
                pendings: all_pendings,
                index: 1,
                guard: |g| g.tab_latch.last_saved.as_str().to_string(),
                live: |g| g.bottom_tab.as_str().to_string(),
            },
            LatchFixture {
                name: "bottom percent",
                old: "30",
                local: "50",
                peer: "65",
                default: "30",
                hold: hold_percent,
                set_pending: flag_percent_pending,
                save: save_bottom_percent,
                retry: retry_all,
                apply_shared: apply_percent,
                apply_other: apply_filter_key,
                pending: |g| g.percent_latch.save_pending,
                pendings: all_pendings,
                index: 2,
                guard: |g| g.percent_latch.last_saved.to_string(),
                live: |g| g.bottom_percent.to_string(),
            },
            LatchFixture {
                name: "strip tab percent",
                old: "40",
                local: "60",
                peer: "25",
                default: "40",
                hold: hold_tab_percent,
                set_pending: flag_tab_percent_pending,
                save: save_tab_percent,
                retry: retry_all,
                apply_shared: apply_tab_percent,
                apply_other: apply_filter_key,
                pending: |g| g.tab_percent_save_pending(),
                pendings: all_pendings,
                index: 3,
                guard: |g| g.tab_percent_latch.last_saved.to_string(),
                live: |g| g.tab_percent.to_string(),
            },
            LatchFixture {
                name: "split tab A",
                // A two-value group: the guard starts at the group's
                // default and the peer re-selects the same other value
                // the local change holds, so the adoption path is the
                // one the fixture can exercise.
                old: BottomTab::Activity.as_str(),
                local: BottomTab::GitStatus.as_str(),
                peer: BottomTab::GitStatus.as_str(),
                default: BottomTab::Activity.as_str(),
                hold: hold_split_tab_a,
                set_pending: flag_split_tab_a_pending,
                save: save_split_tab_a,
                retry: retry_all,
                apply_shared: apply_split_tab_a,
                apply_other: apply_filter_key,
                pending: |g| g.split_tab_a_latch.save_pending,
                pendings: all_pendings,
                index: 4,
                guard: |g| g.split_tab_a_latch.last_saved.as_str().to_string(),
                live: |g| g.split_tab_a.as_str().to_string(),
            },
            LatchFixture {
                name: "split tab B",
                old: BottomTab::Tasks.as_str(),
                local: BottomTab::Thought.as_str(),
                peer: BottomTab::Thought.as_str(),
                default: BottomTab::Tasks.as_str(),
                hold: hold_split_tab_b,
                set_pending: flag_split_tab_b_pending,
                save: save_split_tab_b,
                retry: retry_all,
                apply_shared: apply_split_tab_b,
                apply_other: apply_filter_key,
                pending: |g| g.split_tab_b_latch.save_pending,
                pendings: all_pendings,
                index: 5,
                guard: |g| g.split_tab_b_latch.last_saved.as_str().to_string(),
                live: |g| g.split_tab_b.as_str().to_string(),
            },
            LatchFixture {
                name: "split agents percent",
                old: "33",
                local: "50",
                peer: "65",
                default: "33",
                hold: hold_split_agents_percent,
                set_pending: flag_split_agents_percent_pending,
                save: save_split_agents_percent,
                retry: retry_all,
                apply_shared: apply_split_agents_percent,
                apply_other: apply_filter_key,
                pending: |g| g.split_agents_percent_latch.save_pending,
                pendings: all_pendings,
                index: 6,
                guard: |g| g.split_agents_percent_latch.last_saved.to_string(),
                live: |g| g.split_agents_percent.to_string(),
            },
            LatchFixture {
                name: "split tab B percent",
                old: "33",
                local: "50",
                peer: "65",
                default: "33",
                hold: hold_split_tab_b_percent,
                set_pending: flag_split_tab_b_percent_pending,
                save: save_split_tab_b_percent,
                retry: retry_all,
                apply_shared: apply_split_tab_b_percent,
                apply_other: apply_filter_key,
                pending: |g| g.split_tab_b_percent_latch.save_pending,
                pendings: all_pendings,
                index: 7,
                guard: |g| g.split_tab_b_percent_latch.last_saved.to_string(),
                live: |g| g.split_tab_b_percent.to_string(),
            },
        ]
    }

    /// Every latch's pending flag, in the fixture table's order. The
    /// independence tests skip the fixture's own `index`.
    fn all_pendings(g: &GlobalState) -> Vec<bool> {
        vec![
            g.filter_latch.save_pending,
            g.tab_latch.save_pending,
            g.percent_latch.save_pending,
            g.tab_percent_latch.save_pending,
            g.split_tab_a_latch.save_pending,
            g.split_tab_b_latch.save_pending,
            g.split_agents_percent_latch.save_pending,
            g.split_tab_b_percent_latch.save_pending,
        ]
    }

    fn hold_filter(g: &mut GlobalState) {
        g.filter_latch.last_saved = StatusFilter::All;
        g.status_filter = StatusFilter::Busy;
    }

    fn hold_tab(g: &mut GlobalState) {
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;
    }

    fn hold_percent(g: &mut GlobalState) {
        g.percent_latch.last_saved = 30;
        g.bottom_percent = 50;
    }

    fn hold_tab_percent(g: &mut GlobalState) {
        g.tab_percent_latch.last_saved = 40;
        g.tab_percent = 60;
    }

    fn hold_split_tab_a(g: &mut GlobalState) {
        g.split_tab_a_latch.last_saved = BottomTab::Activity;
        g.split_tab_a = BottomTab::GitStatus;
    }

    fn hold_split_tab_b(g: &mut GlobalState) {
        g.split_tab_b_latch.last_saved = BottomTab::Tasks;
        g.split_tab_b = BottomTab::Thought;
    }

    fn hold_split_agents_percent(g: &mut GlobalState) {
        g.split_agents_percent_latch.last_saved = ui::DEFAULT_SPLIT_PERCENT;
        g.split_agents_percent = 50;
    }

    fn hold_split_tab_b_percent(g: &mut GlobalState) {
        g.split_tab_b_percent_latch.last_saved = ui::DEFAULT_SPLIT_PERCENT;
        g.split_tab_b_percent = 50;
    }

    fn flag_filter_pending(g: &mut GlobalState) {
        g.filter_latch.save_pending = true;
    }

    fn flag_tab_pending(g: &mut GlobalState) {
        g.tab_latch.save_pending = true;
    }

    fn flag_percent_pending(g: &mut GlobalState) {
        g.percent_latch.save_pending = true;
    }

    fn flag_tab_percent_pending(g: &mut GlobalState) {
        g.tab_percent_latch.save_pending = true;
    }

    fn flag_split_tab_a_pending(g: &mut GlobalState) {
        g.split_tab_a_latch.save_pending = true;
    }

    fn flag_split_tab_b_pending(g: &mut GlobalState) {
        g.split_tab_b_latch.save_pending = true;
    }

    fn flag_split_agents_percent_pending(g: &mut GlobalState) {
        g.split_agents_percent_latch.save_pending = true;
    }

    fn flag_split_tab_b_percent_pending(g: &mut GlobalState) {
        g.split_tab_b_percent_latch.save_pending = true;
    }

    fn save_filter(g: &mut GlobalState) {
        g.save_filter();
    }

    fn save_bottom_tab(g: &mut GlobalState) {
        g.save_bottom_tab();
    }

    fn save_bottom_percent(g: &mut GlobalState) {
        g.save_bottom_percent();
    }

    fn save_tab_percent(g: &mut GlobalState) {
        g.save_tab_percent();
    }

    fn save_split_tab_a(g: &mut GlobalState) {
        g.save_split_tab_a();
    }

    fn save_split_tab_b(g: &mut GlobalState) {
        g.save_split_tab_b();
    }

    fn save_split_agents_percent(g: &mut GlobalState) {
        g.save_split_agents_percent();
    }

    fn save_split_tab_b_percent(g: &mut GlobalState) {
        g.save_split_tab_b_percent();
    }

    fn retry_all(g: &mut GlobalState) {
        g.retry_pending_saves();
    }

    fn apply_filter(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_FILTER, value)]), None);
    }

    fn apply_tab(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, value)]), None);
    }

    fn apply_percent(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_HEIGHT, value)]), None);
    }

    fn apply_tab_percent(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_TAB_WIDTH, value)]), None);
    }

    fn apply_split_tab_a(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_SPLIT_TAB_A, value)]), None);
    }

    fn apply_split_tab_b(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_SPLIT_TAB_B, value)]), None);
    }

    fn apply_split_agents_percent(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_SPLIT_AGENTS_WIDTH, value)]), None);
    }

    fn apply_split_tab_b_percent(g: &mut GlobalState, value: &str) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_SPLIT_TAB_B_WIDTH, value)]), None);
    }

    fn apply_tab_key(g: &mut GlobalState) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "activity")]), None);
    }

    fn apply_filter_key(g: &mut GlobalState) {
        g.apply_all(&opts(&[(tmux::SIDEBAR_FILTER, "busy")]), None);
    }

    #[test]
    fn a_rejected_write_keeps_the_guard_and_sets_the_latch() {
        // tmux refused the write, so only the write changes: the guard
        // stays at the save-less value, the user's choice stays in the
        // live field, and the latch is what the retry tick keys on.
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);

            (f.save)(&mut g);

            assert_eq!(
                (f.guard)(&g),
                f.old,
                "{}: only an accepted write may advance the guard",
                f.name
            );
            assert!(
                (f.pending)(&g),
                "{}: a rejected write must set the latch",
                f.name
            );
            assert_eq!(
                (f.live)(&g),
                f.local,
                "{}: a rejected write must not drop the user's choice",
                f.name
            );
        }
    }

    #[test]
    fn a_retry_that_tmux_rejects_again_keeps_latching() {
        // A retried write tmux rejects again must keep the latch (the
        // next tick retries again) and bump nothing: neither the guard
        // nor the live value may move, and the other two latches stay
        // untouched.
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);
            (f.set_pending)(&mut g);

            (f.retry)(&mut g);

            assert!(
                (f.pending)(&g),
                "{}: a rejected retry keeps the latch set",
                f.name
            );
            for (other, pending) in (f.pendings)(&g).into_iter().enumerate() {
                if other == f.index {
                    continue;
                }
                assert!(!pending, "{}: the latches are independent", f.name);
            }
            assert_eq!(
                (f.guard)(&g),
                f.old,
                "{}: the retry must not advance the guard",
                f.name
            );
            assert_eq!(
                (f.live)(&g),
                f.local,
                "{}: the retry must not drop the user's choice",
                f.name
            );
        }
    }

    #[test]
    fn a_pending_save_is_not_overwritten_by_the_shared_value() {
        // While the write is pending, tmux still holds the stale value
        // this instance is trying to replace; adopting it would revert
        // the user's choice.
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);
            (f.set_pending)(&mut g);

            (f.apply_shared)(&mut g, f.old);

            assert_eq!(
                (f.live)(&g),
                f.local,
                "{}: the stale shared value must not overwrite the pending save",
                f.name
            );
        }
    }

    #[test]
    fn a_pending_save_wins_over_a_peers_shared_value_too() {
        // Even a peer's genuinely new value must not overwrite a local
        // choice whose write is still being retried: the retry path
        // pushes the local value back to the server.
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);
            (f.set_pending)(&mut g);

            // The tick after the failed retry still must not adopt
            // over the pending write.
            (f.apply_shared)(&mut g, f.peer);

            assert_eq!(
                (f.live)(&g),
                f.local,
                "{}: a peer's shared value must not overwrite the pending save",
                f.name
            );
        }
    }

    #[test]
    fn the_constructed_default_stays_when_the_shared_key_is_absent() {
        // A fresh server has no shared values yet: the adopt gate
        // parses the reading and finds nothing for the label fields,
        // so adopting fires never and the constructed default stays.
        for f in fixtures() {
            let mut g = GlobalState::new();
            assert_eq!(
                (f.live)(&g),
                f.default,
                "{}: precondition, the constructed default",
                f.name
            );

            // The reading carries another latch's key but not this one.
            (f.apply_other)(&mut g);

            assert_eq!(
                (f.live)(&g),
                f.default,
                "{}: an unset shared value must not change the constructed default",
                f.name
            );
            assert!(
                !(f.pending)(&g),
                "{}: nothing was saved, so nothing may be pending",
                f.name
            );
        }
    }

    #[test]
    fn a_new_shared_value_is_adopted_and_advances_the_guard() {
        // A peer's landed write is taken in like any other shared
        // value, and adoption advances the guard so the next tick's
        // equality check matches it.
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);

            (f.apply_shared)(&mut g, f.peer);

            assert_eq!(
                (f.live)(&g),
                f.peer,
                "{}: a peer's value must be adopted",
                f.name
            );
            assert_eq!(
                (f.guard)(&g),
                f.peer,
                "{}: adoption advances the guard",
                f.name
            );
            assert!(
                !(f.pending)(&g),
                "{}: an adoption is not a pending write",
                f.name
            );
        }
    }

    #[test]
    fn an_unchanged_shared_value_is_not_re_adopted() {
        // The guard compares against the last saved value, so
        // re-reading an already-known value must not touch the field
        // (a local change whose write is on the wire stays put).
        for f in fixtures() {
            let mut g = GlobalState::new();
            (f.hold)(&mut g);

            (f.apply_shared)(&mut g, f.old);

            assert_eq!(
                (f.live)(&g),
                f.local,
                "{}: a shared value matching the guard must not overwrite a local change",
                f.name
            );
        }
    }

    // ─── the percent's own gates ─────────────────────────────────────

    #[test]
    fn a_live_column_drag_blocks_the_percent_adoption_and_it_resumes() {
        // Spec "a drag in progress is not reverted": while the drag is
        // live the rule follows the mouse, so a tick adopting the
        // server's percent would move it under the mouse. The gate is
        // axis-specific: the caller passes the drag's axis, and a
        // column drag is what blocks the bottom percent. After the
        // release, and the caller passing `None` again, the next ticks
        // adopt the shared value like always.
        let mut g = GlobalState::new();
        g.percent_latch.last_saved = 30;
        g.bottom_percent = 30;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_BOTTOM_HEIGHT, "65")]),
            Some(LayoutKind::Column),
        );

        assert_eq!(
            g.bottom_percent, 30,
            "a tick during a live column drag must not adopt the shared percent"
        );
        assert_eq!(
            g.percent_latch.last_saved, 30,
            "the blocked adoption must not advance the last_saved guard"
        );

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_HEIGHT, "65")]), None);

        assert_eq!(
            g.bottom_percent, 65,
            "after the release the shared value is adopted again"
        );
    }

    #[test]
    fn a_strip_drag_does_not_block_the_bottom_percent() {
        // The gates are axis-specific: a strip drag moves the divider,
        // not the column's rule, so the bottom percent adopts while it
        // runs.
        let mut g = GlobalState::new();
        g.percent_latch.last_saved = 30;
        g.bottom_percent = 30;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_BOTTOM_HEIGHT, "65")]),
            Some(LayoutKind::Strip),
        );

        assert_eq!(
            g.bottom_percent, 65,
            "a strip drag must not block the column's percent"
        );
    }

    // ─── the split widths' drag gate ─────────────────────────────

    #[test]
    fn a_live_strip_drag_blocks_the_split_widths_and_it_resumes() {
        // The split widths' twin of the tab-percent gate: while a strip
        // drag is live the divider follows the mouse, so a tick
        // adopting the server's split widths would move the panels
        // under the mouse. After the release the next ticks adopt
        // again.
        let mut g = GlobalState::new();
        g.split_agents_percent_latch.last_saved = ui::DEFAULT_SPLIT_PERCENT;
        g.split_agents_percent = ui::DEFAULT_SPLIT_PERCENT;
        g.split_tab_b_percent_latch.last_saved = ui::DEFAULT_SPLIT_PERCENT;
        g.split_tab_b_percent = ui::DEFAULT_SPLIT_PERCENT;

        g.apply_all(
            &opts(&[
                (tmux::SIDEBAR_SPLIT_AGENTS_WIDTH, "60"),
                (tmux::SIDEBAR_SPLIT_TAB_B_WIDTH, "45"),
            ]),
            Some(LayoutKind::Strip),
        );

        assert_eq!(
            g.split_agents_percent,
            ui::DEFAULT_SPLIT_PERCENT,
            "a tick during a live strip drag must not adopt the shared agent width"
        );
        assert_eq!(
            g.split_tab_b_percent,
            ui::DEFAULT_SPLIT_PERCENT,
            "a tick during a live strip drag must not adopt the shared pane B width"
        );
        assert_eq!(
            g.split_agents_percent_latch.last_saved,
            ui::DEFAULT_SPLIT_PERCENT,
            "the blocked adoption must not advance the last_saved guard"
        );

        g.apply_all(
            &opts(&[
                (tmux::SIDEBAR_SPLIT_AGENTS_WIDTH, "60"),
                (tmux::SIDEBAR_SPLIT_TAB_B_WIDTH, "45"),
            ]),
            None,
        );

        assert_eq!(
            g.split_agents_percent, 60,
            "after the release the shared value is adopted again"
        );
        assert_eq!(
            g.split_tab_b_percent, 45,
            "after the release the shared value is adopted again"
        );
    }

    #[test]
    fn a_column_drag_does_not_block_the_split_widths() {
        // The gates are axis-specific: a column drag moves the rule,
        // not any strip divider, so the split widths adopt while it
        // runs.
        let mut g = GlobalState::new();
        g.split_agents_percent_latch.last_saved = ui::DEFAULT_SPLIT_PERCENT;
        g.split_agents_percent = ui::DEFAULT_SPLIT_PERCENT;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_SPLIT_AGENTS_WIDTH, "60")]),
            Some(LayoutKind::Column),
        );

        assert_eq!(
            g.split_agents_percent, 60,
            "a column drag must not block the split widths"
        );
    }

    #[test]
    fn an_invalid_shared_split_width_adopts_the_default() {
        // The split-width parse has a default: an option value outside
        // 1..=100 or not an integer resolves to `DEFAULT_SPLIT_PERCENT`
        // and is adopted like any other shared value.
        for value in ["abc", "0", "101", ""] {
            let mut g = GlobalState::new();
            g.split_agents_percent_latch.last_saved = 50;
            g.split_agents_percent = 50;
            g.split_tab_b_percent_latch.last_saved = 50;
            g.split_tab_b_percent = 50;

            g.apply_all(
                &opts(&[
                    (tmux::SIDEBAR_SPLIT_AGENTS_WIDTH, value),
                    (tmux::SIDEBAR_SPLIT_TAB_B_WIDTH, value),
                ]),
                None,
            );

            assert_eq!(
                g.split_agents_percent,
                ui::DEFAULT_SPLIT_PERCENT,
                "an invalid shared value {value:?} must adopt the default"
            );
            assert_eq!(
                g.split_tab_b_percent,
                ui::DEFAULT_SPLIT_PERCENT,
                "an invalid shared value {value:?} must adopt the default"
            );
        }
    }

    #[test]
    fn a_live_strip_drag_does_not_block_the_split_tabs() {
        // No drag moves a tab: a strip drag blocks the widths, not the
        // split tabs' adoption.
        let mut g = GlobalState::new();
        g.split_tab_a_latch.last_saved = BottomTab::Activity;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_SPLIT_TAB_A, "git")]),
            Some(LayoutKind::Strip),
        );

        assert_eq!(
            g.split_tab_a,
            BottomTab::GitStatus,
            "a strip drag must not block the split tabs"
        );
    }

    // ─── the strip tab percent's own gates ─────────────────────────

    #[test]
    fn a_live_strip_drag_blocks_the_tab_percent_adoption_and_it_resumes() {
        // The tab percent's twin of the percent gate: while a strip
        // drag is live the strip's divider follows the mouse, so a tick
        // adopting the server's tab width would move it under the
        // mouse. The caller passes the drag's axis; after the release
        // the next ticks adopt again.
        let mut g = GlobalState::new();
        g.tab_percent_latch.last_saved = 40;
        g.tab_percent = 40;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_TAB_WIDTH, "65")]),
            Some(LayoutKind::Strip),
        );

        assert_eq!(
            g.tab_percent, 40,
            "a tick during a live strip drag must not adopt the shared tab width"
        );
        assert_eq!(
            g.tab_percent_latch.last_saved, 40,
            "the blocked adoption must not advance the last_saved guard"
        );

        g.apply_all(&opts(&[(tmux::SIDEBAR_TAB_WIDTH, "65")]), None);

        assert_eq!(
            g.tab_percent, 65,
            "after the release the shared value is adopted again"
        );
    }

    #[test]
    fn a_column_drag_does_not_block_the_tab_percent() {
        // The gates are axis-specific: a column drag moves the rule,
        // not the strip's divider, so the tab percent adopts while it
        // runs.
        let mut g = GlobalState::new();
        g.tab_percent_latch.last_saved = 40;
        g.tab_percent = 40;

        g.apply_all(
            &opts(&[(tmux::SIDEBAR_TAB_WIDTH, "65")]),
            Some(LayoutKind::Column),
        );

        assert_eq!(
            g.tab_percent, 65,
            "a column drag must not block the strip's tab width"
        );
    }

    #[test]
    fn an_invalid_shared_tab_width_adopts_the_default() {
        // The tab-percent parse has a default: an option value outside
        // 0..=100 or not an integer resolves to `DEFAULT_TAB_PERCENT`
        // and is adopted like any other shared value.
        for value in ["abc", "150", ""] {
            let mut g = GlobalState::new();
            g.tab_percent_latch.last_saved = 50;
            g.tab_percent = 50;

            g.apply_all(&opts(&[(tmux::SIDEBAR_TAB_WIDTH, value)]), None);

            assert_eq!(
                g.tab_percent,
                ui::DEFAULT_TAB_PERCENT,
                "an invalid shared value {value:?} must adopt the default"
            );
        }
    }

    #[test]
    fn a_cleared_tab_width_option_resets_to_the_default() {
        // With the option gone from the shared map the tab percent
        // resolves to the default, so clearing the option server-wide
        // brings every strip back to `DEFAULT_TAB_PERCENT`.
        let mut g = GlobalState::new();
        g.tab_percent_latch.last_saved = 50;
        g.tab_percent = 50;

        g.apply_all(&opts(&[]), None);

        assert_eq!(g.tab_percent, ui::DEFAULT_TAB_PERCENT);
        assert_eq!(g.tab_percent_latch.last_saved, ui::DEFAULT_TAB_PERCENT);
    }

    #[test]
    fn an_invalid_shared_height_adopts_the_default() {
        // The percent parse has a default: an option value outside
        // 0..=100 or not an integer resolves to `DEFAULT_BOTTOM_PERCENT`
        // and is adopted like any other shared value.
        for value in ["abc", "150", ""] {
            let mut g = GlobalState::new();
            g.percent_latch.last_saved = 50;
            g.bottom_percent = 50;

            g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_HEIGHT, value)]), None);

            assert_eq!(
                g.bottom_percent,
                ui::DEFAULT_BOTTOM_PERCENT,
                "an invalid shared value {value:?} must adopt the default"
            );
        }
    }

    #[test]
    fn a_cleared_height_option_resets_to_the_default() {
        // With the option gone from the shared map the percent resolves
        // to the default, so clearing the option server-wide brings
        // every instance back to `DEFAULT_BOTTOM_PERCENT`.
        let mut g = GlobalState::new();
        g.percent_latch.last_saved = 50;
        g.bottom_percent = 50;

        g.apply_all(&opts(&[]), None);

        assert_eq!(g.bottom_percent, ui::DEFAULT_BOTTOM_PERCENT);
        assert_eq!(g.percent_latch.last_saved, ui::DEFAULT_BOTTOM_PERCENT);
    }

    // ─── the shared cursor travels as a pane id ───────────────────

    #[test]
    fn a_shared_cursor_parses_as_a_pane_id() {
        assert_eq!(parse_shared_cursor("%3"), Some("%3".into()));
        assert_eq!(parse_shared_cursor(" %42 "), Some("%42".into()));
    }

    #[test]
    fn a_row_index_shared_cursor_names_no_pane() {
        // Versions before the cursor became an identity wrote the row
        // number into the same option. Reading it as an id would put
        // every instance's cursor on whatever row the number hits.
        for raw in ["3", "0", "", "  ", "%", "%x", "%3a", "%3 4", "$"] {
            assert_eq!(parse_shared_cursor(raw), None, "{raw:?} names no pane");
        }
    }

    #[test]
    fn an_adopted_cursor_is_not_written_back_over_itself() {
        // Adoption records the value it read, so the next tick sees its
        // own shared value and leaves the cursor alone.
        let mut g = GlobalState::new();
        g.apply_all(&opts(&[(tmux::SIDEBAR_CURSOR, "%7")]), None);
        assert_eq!(g.cursor_pane_id.as_deref(), Some("%7"));

        g.cursor_pane_id = Some("%9".into());
        g.apply_all(&opts(&[(tmux::SIDEBAR_CURSOR, "%7")]), None);

        assert_eq!(
            g.cursor_pane_id.as_deref(),
            Some("%9"),
            "a value this instance already read must not overwrite a newer local cursor"
        );
    }

    // ─── width lock / shared docked width adoption ─────────────
    //
    // The two fields ride the same shared-reading plumbing as filter
    // and tab: adopted under the latch's `last_saved` guard, skipped
    // while the matching write is pending.

    #[test]
    fn a_peers_width_and_lock_are_adopted_and_the_floor_parses() {
        // A peer enabled the lock with a seeded width; this instance's
        // tick reads both options and takes them in, advancing both
        // guards. The parsed floor rides the same map.
        let mut g = GlobalState::new();

        g.apply_all(
            &opts(&[
                (tmux::SIDEBAR_DOCKED_WIDTH, "45"),
                (tmux::SIDEBAR_WIDTH_LOCK, "on"),
                (tmux::SIDEBAR_MIN_WIDTH, "24"),
            ]),
            None,
        );

        assert!(g.width_lock, "a peer's lock=on must be adopted");
        assert!(
            g.lock_latch.last_saved,
            "adoption advances the guard like the other fields"
        );
        assert_eq!(g.shared_docked_width, Some(45));
        assert_eq!(g.width_latch.last_saved, Some(45));
        assert_eq!(g.min_width, Some(24));
    }

    #[test]
    fn a_local_width_whose_write_failed_is_not_reverted() {
        // The seed write was rejected, so tmux still holds the stale
        // width this instance is replacing. The pending latch must
        // keep it from being adopted back over the local intent.
        let mut g = GlobalState::new();
        g.apply_all(&opts(&[(tmux::SIDEBAR_DOCKED_WIDTH, "45")]), None);
        assert_eq!(g.shared_docked_width, Some(45));

        g.shared_docked_width = Some(35);
        g.width_latch.save_pending = true;

        g.apply_all(
            &opts(&[
                (tmux::SIDEBAR_DOCKED_WIDTH, "45"),
                (tmux::SIDEBAR_WIDTH_LOCK, "on"),
            ]),
            None,
        );

        assert_eq!(
            g.shared_docked_width,
            Some(35),
            "a pending width write must not be reverted by adoption"
        );
        // The lock branch was NOT latched, so it adopts normally.
        assert!(g.width_lock);
    }

    #[test]
    fn a_local_lock_whose_write_failed_is_not_reverted() {
        let mut g = GlobalState::new();
        g.width_lock = true;
        g.lock_latch.last_saved = false; // only an accepted write advances it
        g.lock_latch.save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_WIDTH_LOCK, "off")]), None);

        assert!(
            g.width_lock,
            "a pending lock write must not be reverted by adoption"
        );
    }

    #[test]
    fn missing_width_keys_leave_the_fresh_server_defaults() {
        // Spec scenario "a first instance on a fresh server": no lock,
        // no shared width — unlocked and unseeded just like the
        // pre-lock behavior.
        let mut g = GlobalState::new();
        assert!(!g.width_lock);
        assert_eq!(g.shared_docked_width, None);

        g.apply_all(
            &opts(&[
                (tmux::SIDEBAR_FILTER, "busy"),
                (tmux::SIDEBAR_REPO_FILTER, "app"),
            ]),
            None,
        );

        assert!(!g.width_lock, "no lock key means unlocked");
        assert_eq!(g.shared_docked_width, None, "no width key means unseeded");
    }

    #[test]
    fn a_tick_without_the_floor_option_resets_min_width_to_unset() {
        // `min_width` is adopted unconditionally and RESET when the key
        // is absent, so `tmux set -gu @sidebar_min_width` must drop the
        // instance back to the DEFAULT_MIN_WIDTH fallback at the
        // readers (and a value of 0 stays "disabled", not default).
        let mut g = GlobalState::new();
        g.min_width = Some(24);

        g.apply_all(&opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]), None);

        assert_eq!(
            g.min_width, None,
            "an absent floor option must reset min_width to unset"
        );
    }

    #[test]
    fn an_off_lock_is_not_adopted_as_a_change() {
        // `on` → locked is the only value the adoption reads as true;
        // "off" (the other thing sidebars write) parses as unlocked and
        // equals the fresh default, so nothing moves.
        let mut g = GlobalState::new();

        g.apply_all(&opts(&[(tmux::SIDEBAR_WIDTH_LOCK, "off")]), None);

        assert!(!g.width_lock);
        assert!(!g.lock_latch.save_pending);
    }

    #[test]
    fn a_non_numeric_docked_width_is_ignored() {
        // `@sidebar_width` may legitimately hold percentages; a
        // hand-set `@sidebar_docked_width` with junk must not corrupt
        // the field (the same defensive parse the cursor adoption
        // uses).
        let mut g = GlobalState::new();

        g.apply_all(&opts(&[(tmux::SIDEBAR_DOCKED_WIDTH, "15%")]), None);

        assert_eq!(g.shared_docked_width, None);
    }
}
