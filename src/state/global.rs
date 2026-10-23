use std::collections::HashMap;
use std::time::Instant;

use crate::tmux;

use super::BottomTab;
use super::filter::{RepoFilter, StatusFilter};

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
    // Four pending-write latches share this struct: the filter latch
    // (last_saved_filter / filter_save_pending), the bottom-tab latch
    // (last_saved_bottom_tab / tab_save_pending), the shared-width
    // latch (last_saved_shared_docked_width / width_save_pending) and
    // the lock latch (last_saved_width_lock / lock_save_pending). The
    // later three are deliberate verbatim copies of the filter's —
    // same save/retry calls, same rejection branch, same tick
    // placement — recorded as duplicated code instead of a generic
    // helper; the full latch rationale lives once on
    // `filter_save_pending` and the later latches point at it.
    last_saved_filter: StatusFilter,
    /// A filter write that tmux rejected and that is retried on every
    /// refresh tick. While it is set, the value tmux holds is the stale
    /// one this instance is trying to replace, so the shared value must
    /// not be adopted over the user's choice.
    filter_save_pending: bool,
    /// Last bottom-tab value successfully written to tmux; the tab
    /// latch's copy of `last_saved_filter`.
    last_saved_bottom_tab: BottomTab,
    /// The tab latch's copy of `filter_save_pending`; see that field
    /// and the shared latch comment above for the full rationale.
    tab_save_pending: bool,
    /// On/off switch for the shared docked width; mirrored to and
    /// adopted from the `@sidebar_width_lock` tmux option.
    pub width_lock: bool,
    /// The docked width every sidebar converges on while the lock is
    /// on (`None` until one exists); mirrored to and adopted from the
    /// `@sidebar_docked_width` tmux option. `shared_` because
    /// `AppState.docked_width` already names the pop-out capture.
    pub shared_docked_width: Option<u16>,
    /// Last width-lock value successfully written to tmux; the lock
    /// latch's copy of `last_saved_filter`. `pub(crate)` so sibling
    /// state/app modules can drive the latch test contract (input
    /// tests assert the latch arms on a rejected write).
    pub(crate) last_saved_width_lock: bool,
    /// The lock latch's copy of `filter_save_pending`; see that field
    /// and the shared latch comment above for the full rationale.
    pub(crate) lock_save_pending: bool,
    /// Last shared width successfully written to tmux; the width
    /// latch's copy of `last_saved_filter`.
    pub(crate) last_saved_shared_docked_width: Option<u16>,
    /// The width latch's copy of `filter_save_pending`; see that field
    /// for the full rationale. While it is set the width guard must
    /// also skip its adopt side: the value tmux still holds is the
    /// stale shared width this instance is replacing (design D4), and
    /// adopting it back would resize the pane over the user's hand.
    pub(crate) width_save_pending: bool,
    /// Parsed `@sidebar_min_width` as carried on the per-tick reading,
    /// `None` when the option is unset or not numeric. Adopted as-is
    /// every tick with no guard or latch: no sidebar instance ever
    /// writes the option, so there is no local value to protect, and
    /// the width-lock floor falls back to `DEFAULT_MIN_WIDTH` when
    /// unset.
    pub min_width: Option<u32>,
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
            last_saved_filter: StatusFilter::All,
            filter_save_pending: false,
            last_saved_bottom_tab: BottomTab::Activity,
            tab_save_pending: false,
            width_lock: false,
            shared_docked_width: None,
            last_saved_width_lock: false,
            lock_save_pending: false,
            last_saved_shared_docked_width: None,
            width_save_pending: false,
            min_width: None,
            last_saved_cursor: 0,
            last_saved_repo_filter: RepoFilter::All,
            pending_cursor_save_since: None,
        }
    }

    /// Save filter to tmux global variable.
    /// Only updates `last_saved_filter` on success so that a failed
    /// write does not cause sync to overwrite the user's choice, and
    /// sets `filter_save_pending` so the next refresh tick retries the
    /// write instead of letting the instance drift from the shared
    /// value.
    pub fn save_filter(&mut self) {
        if tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_FILTER,
            self.status_filter.as_str(),
        ])
        .is_some()
        {
            self.last_saved_filter = self.status_filter;
            self.filter_save_pending = false;
        } else {
            self.filter_save_pending = true;
        }
    }

    /// Save the selected bottom tab to tmux global variable — the
    /// tab latch's deliberate copy of the filter's `save_filter`; see
    /// the shared latch comment on the fields for what the copy
    /// keeps identical and why.
    pub fn save_bottom_tab(&mut self) {
        if tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_BOTTOM_TAB,
            self.bottom_tab.as_str(),
        ])
        .is_some()
        {
            self.last_saved_bottom_tab = self.bottom_tab;
            self.tab_save_pending = false;
        } else {
            self.tab_save_pending = true;
        }
    }

    /// Assign the selected bottom tab and persist it to the shared
    /// tmux option. The only writers of the shared tab (the manual
    /// cycle key, the tab-title click) funnel through here so the
    /// whole server converges within one refresh tick.
    pub fn set_bottom_tab(&mut self, tab: BottomTab) {
        self.bottom_tab = tab;
        self.save_bottom_tab();
    }

    /// Save the shared docked width to tmux global variable — the
    /// width latch's deliberate copy of the filter's `save_filter`;
    /// see the shared latch comment on the fields for what the copy
    /// keeps identical and why. Callers (the lock's enabling seed, the
    /// width guard's publish side) assign the desired width to
    /// `shared_docked_width` first, then call this.
    pub fn save_shared_docked_width(&mut self) {
        let Some(width) = self.shared_docked_width else {
            // Production always assigns Some first, so this branch is
            // only reachable from tests: with nothing to publish there
            // is nothing on the wire to reconcile, and a pending flag
            // would block adoption forever — clear it instead.
            self.width_save_pending = false;
            return;
        };
        if tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_DOCKED_WIDTH, &width.to_string()]).is_some()
        {
            self.last_saved_shared_docked_width = Some(width);
            self.width_save_pending = false;
        } else {
            self.width_save_pending = true;
        }
    }

    /// Save the width lock to tmux global variable — the lock latch's
    /// deliberate copy of the filter's `save_filter`; see the shared
    /// latch comment on the fields for what the copy keeps identical
    /// and why. Toggling code assigns the intent to `width_lock`
    /// first, then calls this.
    pub fn save_width_lock(&mut self) {
        let label = if self.width_lock { "on" } else { "off" };
        if tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_WIDTH_LOCK, label]).is_some() {
            self.last_saved_width_lock = self.width_lock;
            self.lock_save_pending = false;
        } else {
            self.lock_save_pending = true;
        }
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
        self.apply_all(&opts);
    }

    /// Retry a filter write tmux rejected, so an instance cannot drift
    /// from the shared value and later have its stale choice adopted by
    /// (or pushed onto) every other sidebar. Called once per refresh
    /// tick, before the tick reads the shared options, so a successful
    /// retry is what that tick reads back.
    pub fn retry_pending_filter_save(&mut self) {
        if self.filter_save_pending {
            self.save_filter();
        }
    }

    /// Retry a bottom-tab write tmux rejected. Runs next to
    /// `retry_pending_filter_save` at the top of each refresh tick,
    /// before the tick reads the shared options, under the same latch
    /// rationale as the filter.
    pub fn retry_pending_tab_save(&mut self) {
        if self.tab_save_pending {
            self.save_bottom_tab();
        }
    }

    /// Retry a shared-width write tmux rejected — the width latch's
    /// copy of `retry_pending_tab_save`; same tick placement and
    /// rationale. On the tick a retried width lands, `apply_all` reads
    /// the same value back and advances `last_saved_shared_docked_width`.
    pub fn retry_pending_width_save(&mut self) {
        if self.width_save_pending {
            self.save_shared_docked_width();
        }
    }

    /// Retry a width-lock write tmux rejected — the lock latch's copy
    /// of `retry_pending_tab_save`; same tick placement and rationale.
    pub fn retry_pending_lock_save(&mut self) {
        if self.lock_save_pending {
            self.save_width_lock();
        }
    }

    /// Retry every shared write tmux rejected (filter, bottom tab,
    /// shared docked width, width lock), so an instance cannot drift
    /// from the shared values. Runs once
    /// per refresh tick, before the tick reads the shared options, so
    /// a successful retry is what that tick reads back. The writes
    /// must stay paired: retrying only one would let the other drift
    /// and be adopted back by every other sidebar.
    pub fn retry_pending_saves(&mut self) {
        self.retry_pending_filter_save();
        self.retry_pending_tab_save();
        self.retry_pending_width_save();
        self.retry_pending_lock_save();
    }

    /// Apply the shared options read from tmux (filter, cursor, bottom
    /// tab, repo filter, width lock, shared docked width, min width).
    /// Runs once at startup and on every refresh tick.
    pub fn apply_all(&mut self, opts: &HashMap<String, String>) {
        // While a filter write is pending, tmux holds the value this
        // instance is trying to replace. Adopting it would revert the
        // user's choice, so the filter is left alone until the write
        // lands.
        if !self.filter_save_pending
            && let Some(filter_str) = opts.get(tmux::SIDEBAR_FILTER)
        {
            let tmux_filter = StatusFilter::from_label(filter_str);
            if tmux_filter != self.last_saved_filter {
                self.status_filter = tmux_filter;
                self.last_saved_filter = tmux_filter;
            }
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
        // The tab adoption below is the filter branch above's
        // deliberate copy: while a tab write is pending the shared
        // value stays unadopted, and a landed write's new value is
        // taken in like any other shared field.
        if !self.tab_save_pending
            && let Some(tab_str) = opts.get(tmux::SIDEBAR_BOTTOM_TAB)
        {
            let tmux_tab = BottomTab::from_label(tab_str);
            if tmux_tab != self.last_saved_bottom_tab {
                self.bottom_tab = tmux_tab;
                self.last_saved_bottom_tab = tmux_tab;
            }
        }
        if let Some(repo_str) = opts.get(tmux::SIDEBAR_REPO_FILTER) {
            let tmux_repo = RepoFilter::from_label(repo_str);
            if tmux_repo != self.last_saved_repo_filter {
                self.repo_filter = tmux_repo.clone();
                self.last_saved_repo_filter = tmux_repo;
            }
        }
        // The lock and width branches below are the filter branch's
        // deliberate copies: while the matching write is pending the
        // shared value stays unadopted, and a landed write's new value
        // is taken in like any other shared field.
        let tmux_lock = matches!(
            opts.get(tmux::SIDEBAR_WIDTH_LOCK).map(String::as_str),
            Some("on")
        );
        if !self.lock_save_pending && tmux_lock != self.last_saved_width_lock {
            self.width_lock = tmux_lock;
            self.last_saved_width_lock = tmux_lock;
        }
        if !self.width_save_pending
            && let Some(width_str) = opts.get(tmux::SIDEBAR_DOCKED_WIDTH)
            && let Ok(width) = width_str.parse::<u16>()
            && Some(width) != self.last_saved_shared_docked_width
        {
            self.shared_docked_width = Some(width);
            self.last_saved_shared_docked_width = Some(width);
        }
        // The parsed floor needs no guard or latch: nothing in the
        // sidebar ever writes `@sidebar_min_width`, so a peer's value
        // can never race a local one — each tick takes tmux's word
        // verbatim. An absent or non-numeric key resets the field to
        // `None`, so clearing the option falls back to
        // `DEFAULT_MIN_WIDTH` at the readers exactly like the
        // per-tick `get_option` this parse replaced.
        self.min_width = opts
            .get(tmux::SIDEBAR_MIN_WIDTH)
            .and_then(|s| s.trim().parse::<u32>().ok());
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

    #[test]
    fn a_pending_filter_save_is_not_overwritten_by_the_shared_value() {
        // tmux rejected this instance's write, so the value tmux still
        // holds is the stale one the instance is trying to replace.
        // Adopting it would revert the user's choice.
        let mut g = GlobalState::new();
        g.last_saved_filter = StatusFilter::All;
        g.status_filter = StatusFilter::Busy;
        g.filter_save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_FILTER, "all")]));

        assert_eq!(g.status_filter, StatusFilter::Busy);
    }

    #[test]
    fn a_rejected_tab_write_keeps_the_last_saved_value_and_sets_the_latch() {
        // In tests every tmux call runs against a socket no server
        // listens on, so `save_bottom_tab` deterministically takes the
        // rejection branch: the guard stays at the save-less value and
        // the write is flagged pending instead.
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;

        g.save_bottom_tab();

        assert_eq!(
            g.last_saved_bottom_tab,
            BottomTab::Activity,
            "only an accepted write may advance last_saved_bottom_tab"
        );
        assert!(g.tab_save_pending);
        assert_eq!(
            g.bottom_tab,
            BottomTab::Tasks,
            "a rejected write must not drop the user's selection"
        );
    }

    #[test]
    fn retry_pending_tab_save_keeps_latching_while_tmux_rejects() {
        // A retried write that tmux rejects again must keep the latch
        // (the next tick retries again) and bump nothing: neither the
        // guard nor the selection may move, and the tab latch stays
        // independent of the filter's.
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;
        g.tab_save_pending = true;

        g.retry_pending_tab_save();

        assert!(g.tab_save_pending, "a rejected retry keeps the latch set");
        assert!(!g.filter_save_pending, "the two latches are independent");
        assert_eq!(g.last_saved_bottom_tab, BottomTab::Activity);
        assert_eq!(g.bottom_tab, BottomTab::Tasks);

        // The tick after the failed retry still must not adopt over the
        // pending write.
        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "git")]));
        assert_eq!(g.bottom_tab, BottomTab::Tasks);
    }

    #[test]
    fn a_pending_tab_save_is_not_overwritten_by_the_shared_value() {
        // tmux rejected this instance's tab write; the stale tab it
        // holds must not revert the user's choice on adoption.
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;
        g.tab_save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "activity")]));

        assert_eq!(g.bottom_tab, BottomTab::Tasks);
    }

    #[test]
    fn a_pending_tab_save_wins_over_a_peer_value_too() {
        // Even a peer's genuinely new value must not overwrite a local
        // selection whose write is still being retried: the retry path
        // pushes the local tab back to the server.
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Activity;
        g.bottom_tab = BottomTab::GitStatus;
        g.tab_save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "tasks")]));

        assert_eq!(g.bottom_tab, BottomTab::GitStatus);
    }

    #[test]
    fn no_shared_tab_key_leaves_the_activity_default() {
        // `AppState::new` (and `GlobalState::new`) start on Activity; a
        // fresh server has no `@sidebar_bottom_tab` key, so adoption
        // fires never and the default stays.
        let mut g = GlobalState::new();
        assert_eq!(g.bottom_tab, BottomTab::Activity);

        g.apply_all(&opts(&[(tmux::SIDEBAR_FILTER, "busy")]));

        assert_eq!(
            g.bottom_tab,
            BottomTab::Activity,
            "an unset tab option must not change the constructed default"
        );
    }

    #[test]
    fn a_new_tab_value_from_tmux_is_adopted() {
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Activity;
        g.bottom_tab = BottomTab::Activity;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "tasks")]));

        assert_eq!(
            g.bottom_tab,
            BottomTab::Tasks,
            "a peer's tab selection must be adopted"
        );
        assert_eq!(
            g.last_saved_bottom_tab,
            BottomTab::Tasks,
            "adoption advances the last_saved guard like the other fields"
        );
        assert!(!g.tab_save_pending);
    }

    #[test]
    fn an_unchanged_shared_tab_is_not_re_adopted() {
        // The guard compares against the last saved tab, so re-reading
        // an already-known value must not touch the field (a local
        // change whose write is on the wire stays put).
        let mut g = GlobalState::new();
        g.last_saved_bottom_tab = BottomTab::Tasks;
        g.bottom_tab = BottomTab::GitStatus;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "tasks")]));

        assert_eq!(
            g.bottom_tab,
            BottomTab::GitStatus,
            "a shared value matching last_saved must not overwrite a local change"
        );
    }

    // ─── width lock / shared docked width adoption ─────────────
    //
    // The two fields ride the same shared-reading plumbing as filter
    // and tab: adopted under a `last_saved_*` guard, skipped while the
    // matching write is pending. Verified in the same shape above.

    #[test]
    fn a_peers_width_and_lock_are_adopted_and_the_floor_parses() {
        // A peer enabled the lock with a seeded width; this instance's
        // tick reads both options and takes them in, advancing both
        // guards. The parsed floor rides the same map.
        let mut g = GlobalState::new();

        g.apply_all(&opts(&[
            (tmux::SIDEBAR_DOCKED_WIDTH, "45"),
            (tmux::SIDEBAR_WIDTH_LOCK, "on"),
            (tmux::SIDEBAR_MIN_WIDTH, "24"),
        ]));

        assert!(g.width_lock, "a peer's lock=on must be adopted");
        assert!(
            g.last_saved_width_lock,
            "adoption advances the guard like the other fields"
        );
        assert_eq!(g.shared_docked_width, Some(45));
        assert_eq!(g.last_saved_shared_docked_width, Some(45));
        assert_eq!(g.min_width, Some(24));
    }

    #[test]
    fn a_local_width_whose_write_failed_is_not_reverted() {
        // The seed write was rejected, so tmux still holds the stale
        // width this instance is replacing. The pending latch must
        // keep it from being adopted back over the local intent.
        let mut g = GlobalState::new();
        g.apply_all(&opts(&[(tmux::SIDEBAR_DOCKED_WIDTH, "45")]));
        assert_eq!(g.shared_docked_width, Some(45));

        g.shared_docked_width = Some(35);
        g.width_save_pending = true;

        g.apply_all(&opts(&[
            (tmux::SIDEBAR_DOCKED_WIDTH, "45"),
            (tmux::SIDEBAR_WIDTH_LOCK, "on"),
        ]));

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
        g.last_saved_width_lock = false; // only an accepted write advances it
        g.lock_save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_WIDTH_LOCK, "off")]));

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

        g.apply_all(&opts(&[
            (tmux::SIDEBAR_FILTER, "busy"),
            (tmux::SIDEBAR_REPO_FILTER, "app"),
        ]));

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

        g.apply_all(&opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]));

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

        g.apply_all(&opts(&[(tmux::SIDEBAR_WIDTH_LOCK, "off")]));

        assert!(!g.width_lock);
        assert!(!g.lock_save_pending);
    }

    #[test]
    fn a_non_numeric_docked_width_is_ignored() {
        // `@sidebar_width` may legitimately hold percentages; a
        // hand-set `@sidebar_docked_width` with junk must not corrupt
        // the field (the same defensive parse the cursor adoption
        // uses).
        let mut g = GlobalState::new();

        g.apply_all(&opts(&[(tmux::SIDEBAR_DOCKED_WIDTH, "15%")]));

        assert_eq!(g.shared_docked_width, None);
    }
}
