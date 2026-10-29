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
/// The status filter, the bottom tab, the bottom percent and the
/// strip tab percent each hold one `SaveLatch` instance, so the
/// save/retry/adopt mechanics below is written once and the four
/// shared fields share it.
struct SaveLatch<T> {
    /// The global option this latch's `set -g` writes go to.
    option: &'static str,
    /// Last value successfully written to tmux.
    last_saved: T,
    /// Whether a write tmux rejected is still being retried.
    save_pending: bool,
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
    /// Last cursor value successfully written to tmux.
    last_saved_cursor: usize,
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
            filter_latch: SaveLatch::new(tmux::SIDEBAR_FILTER, StatusFilter::All),
            tab_latch: SaveLatch::new(tmux::SIDEBAR_BOTTOM_TAB, BottomTab::Activity),
            percent_latch: SaveLatch::new(tmux::SIDEBAR_BOTTOM_HEIGHT, ui::DEFAULT_BOTTOM_PERCENT),
            tab_percent_latch: SaveLatch::new(tmux::SIDEBAR_TAB_WIDTH, ui::DEFAULT_TAB_PERCENT),
            last_saved_cursor: 0,
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

    /// Save cursor position to tmux global variable. Returns `true` when
    /// tmux accepted the write so callers can decide whether to clear a
    /// queued save or keep retrying.
    pub fn save_cursor(&mut self) -> bool {
        if tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_CURSOR,
            &self.selected_pane_row.to_string(),
        ])
        .is_some()
        {
            self.last_saved_cursor = self.selected_pane_row;
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
    /// bottom percent, strip tab percent), so an instance cannot drift
    /// from the shared values (see `SaveLatch::retry` for the latch
    /// rationale). Runs once per refresh tick, before the tick reads
    /// the shared options, so a successful retry is what that tick
    /// reads back. The writes must stay together: retrying only some
    /// would let the others drift and be adopted back by every other
    /// sidebar.
    pub fn retry_pending_saves(&mut self) {
        self.filter_latch
            .retry(self.status_filter, self.status_filter.as_str());
        self.tab_latch
            .retry(self.bottom_tab, self.bottom_tab.as_str());
        self.percent_latch
            .retry(self.bottom_percent, &self.bottom_percent.to_string());
        self.tab_percent_latch
            .retry(self.tab_percent, &self.tab_percent.to_string());
    }

    /// Apply the shared options read from tmux (filter, cursor, bottom
    /// tab, bottom percent, strip tab percent, repo filter). Runs once
    /// at startup and on every refresh tick.
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
        if let Some(cursor_str) = opts.get(tmux::SIDEBAR_CURSOR)
            && let Ok(n) = cursor_str.parse::<usize>()
            && n != self.last_saved_cursor
            // A local move whose debounced write has not landed yet wins
            // over anything another instance writes during that window;
            // the flush persists the user's row and the ticks after it
            // adopt from tmux again.
            && self.pending_cursor_save_since.is_none()
        {
            self.selected_pane_row = n;
            self.last_saved_cursor = n;
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

    // ─── the four shared-value latches (one mechanism, one suite) ───
    //
    // The status filter, the bottom tab, the bottom percent and the
    // strip tab percent ride the same `SaveLatch` (the pending-write
    // field, the save, the retry and the adoption), so the latch
    // mechanics are pinned once with one fixture per latch instead of
    // a cloned test quad per mechanic.
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
        /// The other latches' pendings (latch independence), one bool
        /// per other latch.
        other_pendings: fn(&GlobalState) -> Vec<bool>,
        /// The guard (`last_saved`), serialized like the write.
        guard: fn(&GlobalState) -> String,
        /// The live field value, serialized like the write.
        live: fn(&GlobalState) -> String,
    }

    fn fixtures() -> [LatchFixture; 4] {
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
                other_pendings: |g| {
                    vec![
                        g.tab_latch.save_pending,
                        g.percent_latch.save_pending,
                        g.tab_percent_latch.save_pending,
                    ]
                },
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
                other_pendings: |g| {
                    vec![
                        g.filter_latch.save_pending,
                        g.percent_latch.save_pending,
                        g.tab_percent_latch.save_pending,
                    ]
                },
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
                other_pendings: |g| {
                    vec![
                        g.filter_latch.save_pending,
                        g.tab_latch.save_pending,
                        g.tab_percent_latch.save_pending,
                    ]
                },
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
                other_pendings: |g| {
                    vec![
                        g.filter_latch.save_pending,
                        g.tab_latch.save_pending,
                        g.percent_latch.save_pending,
                    ]
                },
                guard: |g| g.tab_percent_latch.last_saved.to_string(),
                live: |g| g.tab_percent.to_string(),
            },
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
            let others = (f.other_pendings)(&g);
            assert!(
                others.into_iter().all(|pending| !pending),
                "{}: the latches are independent",
                f.name
            );
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
}
