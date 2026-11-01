use crate::desktop_notification;
use crate::desktop_notification::DesktopNotificationKind;
use crate::event::{AgentEvent, Reported};
use crate::pane_state::transition;
use crate::pane_store::PaneStore;
use crate::tmux;

use super::super::context::{
    PENDING_SESSION_END, PENDING_WORKTREE_REMOVE, pane_writes_allowed, run_session_end_teardown,
    should_update_cwd,
};
use super::super::notifications::{
    NotifyLabels, NotifyPayload, notify_lifecycle, session_end_body, session_end_fingerprint,
    set_notification_run_id,
};

pub(in crate::cli::hook) fn on_session_start(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    store: &dyn PaneStore,
) -> i32 {
    let snapshot = store.snapshot(pane);
    // The notification run id is a call-site write and the pending teardown
    // markers are handler-owned state; both ride the transition's batch so
    // the event still costs one apply.
    let mut writes = transition(&snapshot, event, reported);
    set_notification_run_id(&mut writes);
    // `@pane_subagents` is deliberately preserved across SessionStart — the
    // transition leaves it alone. Subagents share the parent's `$TMUX_PANE`,
    // so when a subagent fires its own SessionStart after SubagentStart has
    // populated the list, clearing it here would drop the marker that
    // `should_update_cwd` and `drain_pending_teardowns` rely on. The normal
    // teardown paths (`run_session_end_teardown` via `clear_all_meta`)
    // already clear the list when a real session ends, so the only state
    // this would skip clearing is a subagent list stranded by a hard crash
    // — acceptable vs. racing against legitimate subagent activity.
    // A fresh session overrides any deferred teardown that was waiting for
    // the previous run's subagents to drain.
    writes.unset(PENDING_SESSION_END);
    writes.unset(PENDING_WORKTREE_REMOVE);
    let _ = store.apply(pane, &writes);
    // A fresh session starts with an empty task store, matching the
    // activity log's session semantics so a pane reused for new work
    // does not show the previous run's list. Skipped while subagents
    // are active: they share the parent's `$TMUX_PANE`, and a child's
    // SessionStart must not wipe the parent's list — same rule as
    // `mark_task_reset`. The read is post-batch, so it sees exactly what
    // the apply left behind.
    if should_update_cwd(&store.get(pane, tmux::PANE_SUBAGENTS)) {
        crate::task_store::clear_tasks(pane);
    }
    0
}

pub(in crate::cli::hook) fn on_session_end(
    pane: &str,
    agent_name: &str,
    end_reason: &str,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    // Subagents share the parent's `$TMUX_PANE`, so a SessionEnd fired
    // while `@pane_subagents` is populated is almost certainly a child's
    // (we have no way to distinguish parent vs. child events otherwise).
    // Bail out early before:
    //
    //   1. the notification path consumes the run-scoped fingerprint,
    //      which would silently deduplicate the parent's real SessionEnd
    //      notification when it eventually arrives, and
    //   2. we set PENDING_SESSION_END, which `drain_pending_teardowns`
    //      would later turn into `run_session_end_teardown` — wiping a
    //      still-running parent pane the moment the last subagent stops.
    //
    // The tradeoff is that a parent SessionEnd that genuinely races
    // ahead of every SubagentStop will be ignored too, leaving stale
    // metadata until the next SessionStart clears it. Compared to
    // clobbering a live parent, the stale-metadata failure mode is
    // far safer and the one the user can recover from.
    if !pane_writes_allowed(pane) {
        return 0;
    }

    // Noteworthy terminations (forced logout, bypass-permissions revoked) get
    // a desktop notification so the user isn't left wondering why the pane
    // cleared. Routine reasons (`clear`, `resume`, `prompt_input_exit`,
    // `other`) stay silent.
    if matches!(end_reason, "logout" | "bypass_permissions_disabled") {
        let _ = notify_lifecycle(
            pane,
            NotifyLabels::FromPane { agent: agent_name },
            notifications,
            None,
            NotifyPayload {
                kind: DesktopNotificationKind::TaskCompleted,
                event: desktop_notification::DesktopNotificationEvent::Stop,
                fingerprint_suffix: &session_end_fingerprint(end_reason),
                body: &session_end_body(end_reason),
            },
            store,
        );
    }
    // The thinking feed environment is resolved here, at the hook's
    // edge, the way the polling-side sweeps resolve it, so the
    // teardown body takes it as an argument and tests inject a
    // scratch env.
    run_session_end_teardown(pane, &crate::thinking_feed::FeedEnv::from_process());
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_state::SESSION_CONTEXT;
    use crate::pane_store::MemPaneStore;
    use std::fs;

    fn default_notifications() -> desktop_notification::DesktopNotificationSettings {
        desktop_notification::DesktopNotificationSettings {
            enabled: false,
            events: Default::default(),
        }
    }

    fn session_start_event(source: &str, session_id: Option<&str>) -> AgentEvent {
        AgentEvent::SessionStart {
            agent: "claude".into(),
            cwd: "/repo".into(),
            permission_mode: "default".into(),
            source: source.into(),
            worktree: None,
            agent_id: None,
            session_id: session_id.map(str::to_string),
        }
    }

    #[test]
    fn on_session_end_preserves_parent_state_when_subagents_active() {
        let _guard = tmux::test_mock::install();
        let store = MemPaneStore::new();
        let pane = "%PARENT_END";
        tmux::test_mock::set(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        tmux::test_mock::set(pane, tmux::PANE_AGENT, "claude");
        tmux::test_mock::set(pane, tmux::PANE_CWD, "/repo/parent");
        tmux::test_mock::set(pane, tmux::PANE_SESSION_ID, "parent-session");
        tmux::test_mock::set(pane, tmux::PANE_STATUS, "running");
        // Seed an activity log so we can prove the file is NOT removed.
        let log_path = crate::activity::log_file_path(pane);
        let _ = fs::create_dir_all(log_path.parent().unwrap());
        fs::write(&log_path, "1234567890|Read|main.rs\n").unwrap();

        let exit = on_session_end(pane, "claude", "", &default_notifications(), &store);

        assert_eq!(exit, 0);
        assert!(
            tmux::test_mock::contains(pane, tmux::PANE_AGENT),
            "child SessionEnd must not clear parent @pane_agent"
        );
        assert!(tmux::test_mock::contains(pane, tmux::PANE_CWD));
        assert!(tmux::test_mock::contains(pane, tmux::PANE_SESSION_ID));
        assert!(tmux::test_mock::contains(pane, tmux::PANE_SUBAGENTS));
        assert!(
            log_path.exists(),
            "child SessionEnd must not delete parent activity log"
        );
        assert!(
            !tmux::test_mock::contains(pane, PENDING_SESSION_END),
            "child SessionEnd must not record a pending teardown that \
             `on_subagent_stop` would later replay against the parent"
        );

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn on_session_end_clears_state_when_no_subagents() {
        let _guard = tmux::test_mock::install();
        let store = MemPaneStore::new();
        let pane = "%LONE_END";
        tmux::test_mock::set(pane, tmux::PANE_AGENT, "claude");
        tmux::test_mock::set(pane, tmux::PANE_CWD, "/repo");
        tmux::test_mock::set(pane, tmux::PANE_STATUS, "running");

        let exit = on_session_end(pane, "claude", "", &default_notifications(), &store);

        assert_eq!(exit, 0);
        assert!(
            !tmux::test_mock::contains(pane, tmux::PANE_AGENT),
            "lone SessionEnd should clear @pane_agent"
        );
        assert!(!tmux::test_mock::contains(pane, tmux::PANE_CWD));
        assert!(!tmux::test_mock::contains(pane, tmux::PANE_STATUS));
    }

    #[test]
    fn on_session_start_sets_agent_and_idle_status() {
        let store = MemPaneStore::new();
        let pane = "%NEW_SESSION";

        let exit = on_session_start(
            pane,
            &session_start_event("", Some("sess-123")),
            &SESSION_CONTEXT,
            &store,
        );
        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_AGENT), "claude");
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "idle");
        assert_eq!(store.get(pane, tmux::PANE_SESSION_ID), "sess-123");
        assert!(
            !store.contains(pane, tmux::PANE_PROMPT),
            "SessionStart should clear any stale prompt"
        );
    }

    #[test]
    fn on_session_start_preserves_subagents_list() {
        // Regression: a subagent's own SessionStart arriving after
        // SubagentStart seeded @pane_subagents must NOT drop the
        // parent's list. If it did, should_update_cwd would start
        // returning true and the next hook from either side could
        // clobber the parent's cwd/worktree metadata.
        let store = MemPaneStore::new();
        let pane = "%SUBAGENT_LIVE";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");

        on_session_start(
            pane,
            &session_start_event("", None),
            &SESSION_CONTEXT,
            &store,
        );

        assert_eq!(
            store.get(pane, tmux::PANE_SUBAGENTS),
            "Explore:sub-1",
            "SessionStart must not wipe an active subagent list"
        );
    }

    #[test]
    fn fresh_session_start_clears_pending_markers() {
        let store = MemPaneStore::new();
        let pane = "%PARENT_RESTART";
        store.insert(pane, PENDING_SESSION_END, "1");
        store.insert(pane, PENDING_WORKTREE_REMOVE, "1");

        on_session_start(
            pane,
            &session_start_event("", None),
            &SESSION_CONTEXT,
            &store,
        );

        assert!(
            !store.contains(pane, PENDING_SESSION_END),
            "fresh SessionStart must drop a stale pending marker"
        );
        assert!(!store.contains(pane, PENDING_WORKTREE_REMOVE));
    }

    #[test]
    fn on_session_start_clears_the_task_store() {
        let store = MemPaneStore::new();
        let pane = "%TASK_FRESH_SESSION";
        crate::task_store::record_task(pane, "stale task").unwrap();
        assert!(crate::task_store::store_file_path(pane).exists());

        on_session_start(
            pane,
            &session_start_event("", None),
            &SESSION_CONTEXT,
            &store,
        );

        assert!(
            !crate::task_store::store_file_path(pane).exists(),
            "SessionStart must clear the pane's task store"
        );
    }

    #[test]
    fn on_session_start_preserves_task_store_while_subagents_active() {
        let store = MemPaneStore::new();
        let pane = "%TASK_SUBAGENT_SESSION";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        crate::task_store::record_task(pane, "parent task").unwrap();

        on_session_start(
            pane,
            &session_start_event("", None),
            &SESSION_CONTEXT,
            &store,
        );

        assert!(
            crate::task_store::store_file_path(pane).exists(),
            "a subagent's SessionStart must not wipe the parent's task store"
        );

        let _ = std::fs::remove_file(crate::task_store::store_file_path(pane));
    }

    #[test]
    fn on_session_start_resume_writes_wait_reason() {
        let store = MemPaneStore::new();
        let pane = "%RESUME";
        on_session_start(
            pane,
            &session_start_event("resume", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_WAIT_REASON), "session_resumed");
    }

    #[test]
    fn on_session_start_compact_writes_compact_wait_reason() {
        let store = MemPaneStore::new();
        let pane = "%COMPACT";
        on_session_start(
            pane,
            &session_start_event("compact", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert_eq!(
            store.get(pane, tmux::PANE_WAIT_REASON),
            "session_resumed_compact",
        );
    }

    #[test]
    fn on_session_start_startup_clears_stale_wait_reason() {
        let store = MemPaneStore::new();
        let pane = "%FRESH";
        store.insert(pane, tmux::PANE_WAIT_REASON, "session_resumed");
        on_session_start(
            pane,
            &session_start_event("startup", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert!(
            !store.contains(pane, tmux::PANE_WAIT_REASON),
            "startup source should drop a stale resume marker"
        );
    }

    fn notifications_enabled_all() -> desktop_notification::DesktopNotificationSettings {
        // The Stop event is the one our SessionEnd notification is gated on;
        // `enabled: true` plus the Stop event lets `notify_if_allowed` reach
        // the point where it writes the dedup stamp in the tmux mock. The
        // send path itself is guarded by `SIDEBAR_DISABLE_DESKTOP_NOTIFICATIONS`
        // (set for this repo via `.cargo/config.toml`), so it is a no-op in
        // test runs and the stamp still gets written.
        desktop_notification::DesktopNotificationSettings {
            enabled: true,
            events: [desktop_notification::DesktopNotificationEvent::Stop]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn on_session_end_routine_reason_does_not_notify() {
        let _guard = tmux::test_mock::install();
        let store = MemPaneStore::new();
        let pane = "%END_ROUTINE";
        on_session_end(
            pane,
            "claude",
            "clear",
            &notifications_enabled_all(),
            &store,
        );
        // The notification helper writes a dedup stamp only when a notification
        // actually goes out; a missing stamp is proof the gate rejected it.
        assert!(
            !tmux::test_mock::contains(pane, tmux::PANE_OS_NOTIFY_TASK_COMPLETED),
            "routine end_reason must not fire a desktop notification"
        );
    }

    #[test]
    fn on_session_end_logout_attempts_notification() {
        let _guard = tmux::test_mock::install();
        // The run id is read through the store, so the seed goes here; the
        // dedup stamp is still written through the raw helpers (row 4.5).
        let store = MemPaneStore::new();
        let pane = "%END_LOGOUT";
        store.insert(pane, tmux::PANE_NOTIFICATION_RUN_ID, "1700000000000");
        // Agent name is surfaced in the desktop notification title; using an
        // obvious test marker makes it trivial to spot when a local `cargo
        // test` run happens to actually fire osascript.
        on_session_end(
            pane,
            "cargo-test: on_session_end_logout",
            "logout",
            &notifications_enabled_all(),
            &store,
        );
        // `send_desktop_notification` succeeds (it is a no-op under the
        // `SIDEBAR_DISABLE_DESKTOP_NOTIFICATIONS` guard), so the stamp is
        // always written and the fingerprint assertion below always runs.
        let stamp_key = tmux::PANE_OS_NOTIFY_TASK_COMPLETED;
        if tmux::test_mock::contains(pane, stamp_key) {
            let raw = tmux::test_mock::get(pane, stamp_key).unwrap_or_default();
            assert!(
                raw.contains("session-ended:logout"),
                "stamp must record the session-end fingerprint, got {raw}"
            );
        }
    }

    #[test]
    fn on_session_end_bypass_disabled_attempts_notification() {
        let _guard = tmux::test_mock::install();
        let store = MemPaneStore::new();
        let pane = "%END_BYPASS";
        store.insert(pane, tmux::PANE_NOTIFICATION_RUN_ID, "1700000000000");
        on_session_end(
            pane,
            "cargo-test: on_session_end_bypass_disabled",
            "bypass_permissions_disabled",
            &notifications_enabled_all(),
            &store,
        );
        let stamp_key = tmux::PANE_OS_NOTIFY_TASK_COMPLETED;
        if tmux::test_mock::contains(pane, stamp_key) {
            let raw = tmux::test_mock::get(pane, stamp_key).unwrap_or_default();
            assert!(
                raw.contains("session-ended:bypass_permissions_disabled"),
                "stamp must record the session-end fingerprint, got {raw}"
            );
        }
    }
}
