use std::collections::HashMap;
use std::time::Instant;

use crate::tmux;

use super::filter::{RepoFilter, StatusFilter};

/// State shared across all sidebar instances via tmux global variables.
/// Read once at startup and re-applied from the shared options on every
/// refresh tick.
pub struct GlobalState {
    pub status_filter: StatusFilter,
    pub selected_pane_row: usize,
    pub repo_filter: RepoFilter,
    /// Last filter value successfully written to tmux.
    last_saved_filter: StatusFilter,
    /// A filter write that tmux rejected and that is retried on every
    /// refresh tick. While it is set, the value tmux holds is the stale
    /// one this instance is trying to replace, so the shared value must
    /// not be adopted over the user's choice.
    filter_save_pending: bool,
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
            last_saved_filter: StatusFilter::All,
            filter_save_pending: false,
            last_saved_cursor: 0,
            last_saved_repo_filter: RepoFilter::All,
            pending_cursor_save_since: None,
        }
    }

    /// Save filter to tmux global variable.
    /// Only updates `last_saved_filter` on success so that a failed write
    /// does not cause sync to overwrite the user's choice, and leaves
    /// `filter_save_pending` set so the next refresh tick retries the
    /// write instead of letting the instance drift from the shared value.
    pub fn save_filter(&mut self) -> bool {
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
            true
        } else {
            self.filter_save_pending = true;
            false
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

    /// Apply the shared options read from tmux (filter, cursor, repo
    /// filter). Runs once at startup and on every refresh tick.
    /// Retry a filter write tmux rejected, so an instance cannot drift
    /// from the shared value and later have its stale choice adopted by
    /// (or pushed onto) every other sidebar. Called once per refresh
    /// tick, before the tick reads the shared options, so a successful
    /// retry is what that tick reads back.
    pub fn retry_pending_filter_save(&mut self) -> bool {
        if !self.filter_save_pending {
            return false;
        }
        self.save_filter()
    }

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
    fn retry_reports_nothing_to_do_without_a_pending_save() {
        let mut g = GlobalState::new();
        assert!(!g.retry_pending_filter_save());
    }
}
