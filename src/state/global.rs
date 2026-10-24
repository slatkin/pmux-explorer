use std::collections::HashMap;
use std::time::Instant;

use crate::tmux;

use super::BottomTab;
use super::filter::{RepoFilter, StatusFilter};

/// One shared-option write latch: the value the paired `GlobalState`
/// field carried at its last successful tmux write, guarded by a retry
/// flag.
///
/// `save_*` writes the paired field's value to its shared tmux option
/// through [`Latch::record_write`]. A write tmux accepts advances
/// `last_saved` and clears the flag; a write tmux rejects leaves both,
/// arming the flag:
///
/// 1. every refresh tick retries the write (`retry_pending_*_save`)
///    before the tick reads the shared options, so a successful retry is
///    what that tick reads back and a transient failure cannot let this
///    instance drift from the shared value only to have its stale choice
///    adopted back by (or pushed onto) every other sidebar;
/// 2. `apply_all` skips adopting the option's value over the field:
///    the value tmux still holds is the stale one this instance is
///    trying to replace, so adopting it would revert the user's choice.
///
/// The four shared fields (status filter, bottom tab, width lock, shared
/// docked width) share this one implementation; their `save_*` methods
/// and `apply_all` arms differ only in which option they write and what
/// value adoption hands back.
pub(crate) struct Latch<T: Copy + PartialEq> {
    /// The paired field's value as it stood at the last successful
    /// write; the guard adoption compares against.
    pub(crate) last_saved: T,
    /// A write that tmux rejected and that is retried on every refresh
    /// tick. While it is set, the value tmux holds is the stale one
    /// this instance is trying to replace, so the shared value must not
    /// be adopted over the user's choice.
    pub(crate) save_pending: bool,
}

impl<T: Copy + PartialEq> Latch<T> {
    /// A latch over an option with nothing on the wire yet: `last_saved`
    /// starts at the paired field's constructed default.
    fn new(last_saved: T) -> Self {
        Self {
            last_saved,
            save_pending: false,
        }
    }

    /// Record one attempt to write the paired field's value to its
    /// tmux option. Only an accepted write may advance `last_saved` —
    /// a failed write must not cause the next sync to overwrite the
    /// user's choice — and a rejected write arms `save_pending`, so the
    /// next refresh tick retries the write instead of letting the
    /// instance drift from the shared value.
    fn record_write(&mut self, accepted: bool, written: T) {
        if accepted {
            self.last_saved = written;
            self.save_pending = false;
        } else {
            self.save_pending = true;
        }
    }

    /// `true` while a paired write has not landed: the tick must not
    /// adopt the option's value over the field's own.
    fn is_pending(&self) -> bool {
        self.save_pending
    }

    /// Take a value read back from the shared options in as the paired
    /// field's new content. Returns it only when it is genuinely new —
    /// something a peer wrote rather than this instance's own last
    /// write — and leaves both the latch and the caller otherwise
    /// unchanged.
    fn adopt(&mut self, shared: T) -> Option<T> {
        if self.save_pending || shared == self.last_saved {
            return None;
        }
        self.last_saved = shared;
        Some(shared)
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
    // Four shared fields ride the one [`Latch`] implementation: the
    // status filter, the bottom tab, the width lock, and the shared
    // docked width. Each latch pairs a `last_saved` value with a
    // pending-write flag; its full rationale — retry per refresh tick,
    // no adoption over a pending write — lives once on the type. The
    // width latch's flag additionally gates the width guard's adopt
    // side (design D4), and the two width latches are read by sibling
    // state/app modules; hence the `pub(crate)` fields.
    filter_latch: Latch<StatusFilter>,
    /// The tab latch; see [`Latch`] for the semantics it carries.
    tab_latch: Latch<BottomTab>,
    /// On/off switch for the shared docked width; mirrored to and
    /// adopted from the `@sidebar_width_lock` tmux option.
    pub width_lock: bool,
    /// The docked width every sidebar converges on while the lock is
    /// on (`None` until one exists); mirrored to and adopted from the
    /// `@sidebar_docked_width` tmux option. `shared_` because
    /// `AppState.docked_width` already names the pop-out capture.
    pub shared_docked_width: Option<u16>,
    /// The lock latch. `pub(crate)` so sibling state/app modules
    /// (input's `w` handler and the state tests) can drive the latch
    /// contract on a rejected write.
    pub(crate) lock_latch: Latch<bool>,
    /// The width latch. While its write is pending the width guard must
    /// also skip its adopt side: the value tmux still holds is the
    /// stale shared width this instance is replacing (design D4), and
    /// adopting it back would resize the pane over the user's hand.
    pub(crate) width_latch: Latch<Option<u16>>,
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
            filter_latch: Latch::new(StatusFilter::All),
            tab_latch: Latch::new(BottomTab::Activity),
            width_lock: false,
            shared_docked_width: None,
            lock_latch: Latch::new(false),
            width_latch: Latch::new(None),
            min_width: None,
            last_saved_cursor: 0,
            last_saved_repo_filter: RepoFilter::All,
            pending_cursor_save_since: None,
        }
    }

    /// Save filter to tmux global variable — the status filter's
    /// latch write; see [`Latch`] for the accept/reject contract.
    pub fn save_filter(&mut self) {
        let accepted = tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_FILTER,
            self.status_filter.as_str(),
        ])
        .is_some();
        self.filter_latch.record_write(accepted, self.status_filter);
    }

    /// Save the selected bottom tab to tmux global variable — the tab
    /// latch's write; see [`Latch`] for the accept/reject contract.
    pub fn save_bottom_tab(&mut self) {
        let accepted = tmux::run_tmux(&[
            "set",
            "-g",
            tmux::SIDEBAR_BOTTOM_TAB,
            self.bottom_tab.as_str(),
        ])
        .is_some();
        self.tab_latch.record_write(accepted, self.bottom_tab);
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
    /// width latch's write; see [`Latch`] for the accept/reject
    /// contract. Callers (the lock's enabling seed, the width guard's
    /// publish side) assign the desired width to `shared_docked_width`
    /// first, then call this.
    pub fn save_shared_docked_width(&mut self) {
        let Some(width) = self.shared_docked_width else {
            // Production always assigns Some first, so this branch is
            // only reachable from tests: with nothing to publish there
            // is nothing on the wire to reconcile, and a pending flag
            // would block adoption forever. Off the Latch path
            // deliberately: clearing the flag alone must not advance
            // `last_saved` to `None` when a width was saved before.
            self.width_latch.save_pending = false;
            return;
        };
        let accepted =
            tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_DOCKED_WIDTH, &width.to_string()])
                .is_some();
        self.width_latch.record_write(accepted, Some(width));
    }

    /// Save the width lock to tmux global variable — the lock latch's
    /// write; see [`Latch`] for the accept/reject contract. Toggling
    /// code assigns the intent to `width_lock` first, then calls this.
    pub fn save_width_lock(&mut self) {
        let label = if self.width_lock { "on" } else { "off" };
        let accepted = tmux::run_tmux(&["set", "-g", tmux::SIDEBAR_WIDTH_LOCK, label]).is_some();
        self.lock_latch.record_write(accepted, self.width_lock);
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
        if self.filter_latch.is_pending() {
            self.save_filter();
        }
    }

    /// Retry a bottom-tab write tmux rejected. Runs next to
    /// `retry_pending_filter_save` at the top of each refresh tick,
    /// before the tick reads the shared options, under the same latch
    /// rationale as the filter.
    pub fn retry_pending_tab_save(&mut self) {
        if self.tab_latch.is_pending() {
            self.save_bottom_tab();
        }
    }

    /// Retry a shared-width write tmux rejected — the width latch's
    /// retry; same tick placement and rationale. On the tick a retried
    /// width lands, `apply_all` reads the same value back and advances
    /// the width latch's `last_saved`.
    pub fn retry_pending_width_save(&mut self) {
        if self.width_latch.is_pending() {
            self.save_shared_docked_width();
        }
    }

    /// Retry a width-lock write tmux rejected — the lock latch's
    /// retry; same tick placement and rationale.
    pub fn retry_pending_lock_save(&mut self) {
        if self.lock_latch.is_pending() {
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
        if let Some(filter_str) = opts.get(tmux::SIDEBAR_FILTER)
            && let Some(status_filter) = self
                .filter_latch
                .adopt(StatusFilter::from_label(filter_str))
        {
            self.status_filter = status_filter;
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
        if let Some(tab_str) = opts.get(tmux::SIDEBAR_BOTTOM_TAB)
            && let Some(bottom_tab) = self.tab_latch.adopt(BottomTab::from_label(tab_str))
        {
            self.bottom_tab = bottom_tab;
        }
        if let Some(repo_str) = opts.get(tmux::SIDEBAR_REPO_FILTER) {
            let tmux_repo = RepoFilter::from_label(repo_str);
            if tmux_repo != self.last_saved_repo_filter {
                self.repo_filter = tmux_repo.clone();
                self.last_saved_repo_filter = tmux_repo;
            }
        }
        let tmux_lock = matches!(
            opts.get(tmux::SIDEBAR_WIDTH_LOCK).map(String::as_str),
            Some("on")
        );
        if let Some(lock) = self.lock_latch.adopt(tmux_lock) {
            self.width_lock = lock;
        }
        if let Some(width_str) = opts.get(tmux::SIDEBAR_DOCKED_WIDTH)
            && let Ok(width) = width_str.parse::<u16>()
            && let Some(shared) = self.width_latch.adopt(Some(width))
        {
            self.shared_docked_width = shared;
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
        g.filter_latch.last_saved = StatusFilter::All;
        g.status_filter = StatusFilter::Busy;
        g.filter_latch.save_pending = true;

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
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;

        g.save_bottom_tab();

        assert_eq!(
            g.tab_latch.last_saved,
            BottomTab::Activity,
            "only an accepted write may advance the tab latch's last_saved"
        );
        assert!(g.tab_latch.save_pending);
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
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;
        g.tab_latch.save_pending = true;

        g.retry_pending_tab_save();

        assert!(
            g.tab_latch.save_pending,
            "a rejected retry keeps the latch set"
        );
        assert!(
            !g.filter_latch.save_pending,
            "the two latches are independent"
        );
        assert_eq!(g.tab_latch.last_saved, BottomTab::Activity);
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
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::Tasks;
        g.tab_latch.save_pending = true;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "activity")]));

        assert_eq!(g.bottom_tab, BottomTab::Tasks);
    }

    #[test]
    fn a_pending_tab_save_wins_over_a_peer_value_too() {
        // Even a peer's genuinely new value must not overwrite a local
        // selection whose write is still being retried: the retry path
        // pushes the local tab back to the server.
        let mut g = GlobalState::new();
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::GitStatus;
        g.tab_latch.save_pending = true;

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
        g.tab_latch.last_saved = BottomTab::Activity;
        g.bottom_tab = BottomTab::Activity;

        g.apply_all(&opts(&[(tmux::SIDEBAR_BOTTOM_TAB, "tasks")]));

        assert_eq!(
            g.bottom_tab,
            BottomTab::Tasks,
            "a peer's tab selection must be adopted"
        );
        assert_eq!(
            g.tab_latch.last_saved,
            BottomTab::Tasks,
            "adoption advances the last_saved guard like the other fields"
        );
        assert!(!g.tab_latch.save_pending);
    }

    #[test]
    fn an_unchanged_shared_tab_is_not_re_adopted() {
        // The guard compares against the last saved tab, so re-reading
        // an already-known value must not touch the field (a local
        // change whose write is on the wire stays put).
        let mut g = GlobalState::new();
        g.tab_latch.last_saved = BottomTab::Tasks;
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
        g.apply_all(&opts(&[(tmux::SIDEBAR_DOCKED_WIDTH, "45")]));
        assert_eq!(g.shared_docked_width, Some(45));

        g.shared_docked_width = Some(35);
        g.width_latch.save_pending = true;

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
        g.lock_latch.last_saved = false; // only an accepted write advances it
        g.lock_latch.save_pending = true;

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
        assert!(!g.lock_latch.save_pending);
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
