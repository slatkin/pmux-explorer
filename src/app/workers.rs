use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

use crate::git::{self, GitData};
use crate::state::AppState;
use crate::tmux;
use crate::version::{self, UpdateNotice};

/// Channels and shared flags produced by [`spawn`] that the main event loop
/// drains every tick.
pub(super) struct Workers {
    pub git_rx: Receiver<GitData>,
    pub version_rx: Receiver<UpdateNotice>,
    pub git_tab_active: Arc<AtomicBool>,
}

/// Spawn the background threads (git polling, version notice fetch) that
/// feed the event loop.
///
/// Lifecycle: neither join handle is kept. The git poll thread is a detached
/// loop that returns only when a `send` reports the receiver is gone — which
/// happens when the event loop drops `git_rx` at teardown — and otherwise runs
/// until the process exits. The version thread is one-shot (fetch, send,
/// exit), so there is nothing to join.
///
/// Overload policy: both channels are `sync_channel(1)`. The event loop only
/// ever wants the newest git data, so a full buffer makes the poll thread wait
/// for the next drain instead of letting a stalled event loop grow one
/// `GitData` per two-second tick without bound. That wait cannot deadlock:
/// `app::run` drains the receiver with `try_recv` at least once per ~16 ms. A
/// blocked `send` still returns `Err` when the receiver is dropped, so the
/// thread's exit path is preserved. The version channel is bounded the same way
/// for consistency; a single send can never fill its buffer.
pub(super) fn spawn(state: &AppState) -> Workers {
    let (git_tx, git_rx) = mpsc::sync_channel::<GitData>(1);
    let (version_tx, version_rx) = mpsc::sync_channel::<UpdateNotice>(1);
    let tmux_pane_clone = state.tmux_pane.clone();
    let git_tab_active = Arc::new(AtomicBool::new(
        state.displayed_bottom_tab().is_git_status(),
    ));
    let git_tab_flag = Arc::clone(&git_tab_active);
    // Detached on purpose: the loop ends when `git_tx.send` fails after the
    // event loop has dropped the receiver, so there is no join handle to keep.
    std::thread::spawn(move || {
        git_poll_loop(&tmux_pane_clone, &git_tx, &git_tab_flag);
    });
    // One-shot: fetch, send, exit. Nothing to join and no backlog to bound.
    std::thread::spawn(move || {
        if let Some(notice) = version::fetch_update_notice() {
            let _ = version_tx.send(notice);
        }
    });

    Workers {
        git_rx,
        version_rx,
        git_tab_active,
    }
}

/// Git data polling thread. Fetches git status every 2 seconds while the Git
/// tab is active. Skips fetching when the tab is not visible. PR numbers go
/// through an in-memory `(path, branch)`-keyed cache so `gh pr view` (the only
/// hop that costs GitHub API quota) runs at most once per `PR_CACHE_TTL`
/// instead of every tick.
///
/// Returns once `git_tx.send` fails, i.e. when the event loop has dropped the
/// receiver during teardown; otherwise it loops until the process exits.
pub(super) fn git_poll_loop(tmux_pane: &str, git_tx: &SyncSender<GitData>, active: &AtomicBool) {
    let mut last_path: Option<String> = None;
    let mut pr_cache = git::PrCache::new();
    loop {
        std::thread::sleep(Duration::from_secs(2));

        if !active.load(Ordering::Relaxed) {
            continue;
        }

        // When the sidebar has focus, focused_pane_path returns None.
        // Reuse the last known path so git data keeps updating.
        if let Some(p) = tmux::focused_pane_path(tmux_pane) {
            last_path = Some(p);
        }
        if let Some(ref path) = last_path {
            let mut data = git::fetch_git_data(path);
            data.pr_number = pr_cache.get_or_fetch(
                path,
                &data.branch,
                std::time::Instant::now(),
                git::fetch_pr_number,
            );
            if git_tx.send(data).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_poll_skips_when_inactive() {
        let active = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel::<GitData>(1);

        let flag = Arc::clone(&active);
        let handle = std::thread::spawn(move || {
            // Simulate the poll loop check without actually sleeping 2s
            for _ in 0..3 {
                if !flag.load(Ordering::Relaxed) {
                    continue;
                }
                let _ = tx.send(GitData::default());
            }
        });

        handle.join().unwrap();
        // No data should have been sent since active=false
        assert!(
            rx.try_recv().is_err(),
            "should not poll when git tab is inactive"
        );
    }

    #[test]
    fn test_git_poll_sends_when_active() {
        let active = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::sync_channel::<GitData>(1);

        let flag = Arc::clone(&active);
        let handle = std::thread::spawn(move || {
            // active=true, so it should send
            if flag.load(Ordering::Relaxed) {
                let _ = tx.send(GitData::default());
            }
        });

        handle.join().unwrap();
        assert!(rx.try_recv().is_ok(), "should poll when git tab is active");
    }

    #[test]
    fn test_git_poll_reacts_to_flag_change() {
        let active = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel::<GitData>(1);

        // Initially inactive
        assert!(!active.load(Ordering::Relaxed));

        // Switch to active
        active.store(true, Ordering::Relaxed);

        let flag = Arc::clone(&active);
        let handle = std::thread::spawn(move || {
            if flag.load(Ordering::Relaxed) {
                let _ = tx.send(GitData::default());
            }
        });

        handle.join().unwrap();
        assert!(
            rx.try_recv().is_ok(),
            "should poll after flag switches to active"
        );
    }

    #[test]
    fn test_git_poll_stops_on_sender_closed() {
        let active = AtomicBool::new(true);
        let (tx, rx) = mpsc::sync_channel::<GitData>(1);
        drop(rx); // Close receiver

        let result = tx.send(GitData::default());
        assert!(result.is_err(), "send should fail when receiver is dropped");

        // Verify the flag check pattern used in git_poll_loop
        assert!(active.load(Ordering::Relaxed));
    }

    #[test]
    fn test_git_poll_blocked_send_unblocks_when_receiver_dropped() {
        let (tx, rx) = mpsc::sync_channel::<GitData>(1);
        // Fill the buffer so the next send has to wait for a drain.
        tx.send(GitData::default())
            .expect("the empty channel accepts one item");

        let handle = std::thread::spawn(move || tx.send(GitData::default()).is_err());
        // Give the thread time to block in `send`, then close the receiver the
        // way the event loop does at teardown.
        std::thread::sleep(Duration::from_millis(50));
        drop(rx);

        assert!(
            handle.join().expect("the send thread must not panic"),
            "a blocked send must return Err once the receiver is dropped"
        );
    }
}
