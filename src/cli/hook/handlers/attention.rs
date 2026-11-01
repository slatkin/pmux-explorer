use crate::desktop_notification;
use crate::desktop_notification::DesktopNotificationKind;
use crate::event::{AgentEvent, Reported};
use crate::pane_state::transition;
use crate::pane_store::PaneStore;

use super::super::context::AgentContext;
use super::super::notifications::{
    NotifyLabels, NotifyPayload, notification_body, notification_fingerprint, notify_lifecycle,
};

pub(in crate::cli::hook) fn on_notification(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    ctx: &AgentContext<'_>,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    let AgentEvent::Notification {
        wait_reason,
        meta_only,
        ..
    } = event
    else {
        return 0;
    };
    let snapshot = store.snapshot(pane);
    let writes = transition(&snapshot, event, reported);
    let _ = store.apply(pane, &writes);
    if *meta_only {
        return 0;
    }
    let _ = notify_lifecycle(
        pane,
        NotifyLabels::FromCtx(ctx),
        notifications,
        None,
        NotifyPayload {
            kind: DesktopNotificationKind::PermissionRequired,
            event: desktop_notification::DesktopNotificationEvent::Notification,
            fingerprint_suffix: notification_fingerprint(wait_reason),
            body: &notification_body(wait_reason),
        },
        store,
    );
    0
}

pub(in crate::cli::hook) fn on_permission_denied(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    ctx: &AgentContext<'_>,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    let snapshot = store.snapshot(pane);
    let writes = transition(&snapshot, event, reported);
    let _ = store.apply(pane, &writes);
    let _ = notify_lifecycle(
        pane,
        NotifyLabels::FromCtx(ctx),
        notifications,
        None,
        NotifyPayload {
            kind: DesktopNotificationKind::PermissionRequired,
            event: desktop_notification::DesktopNotificationEvent::PermissionDenied,
            fingerprint_suffix: "permission_denied",
            body: "Permission required",
        },
        store,
    );
    0
}

pub(in crate::cli::hook) fn on_teammate_idle(
    pane: &str,
    teammate_name: &str,
    idle_reason: &str,
    store: &dyn PaneStore,
) -> i32 {
    let snapshot = store.snapshot(pane);
    let event = AgentEvent::TeammateIdle {
        teammate_name: teammate_name.to_string(),
        team_name: String::new(),
        idle_reason: idle_reason.to_string(),
    };
    let writes = transition(&snapshot, &event, &Reported::NONE);
    let _ = store.apply(pane, &writes);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_state::SESSION_CONTEXT;
    use crate::pane_store::MemPaneStore;
    use crate::tmux;

    fn default_notifications() -> desktop_notification::DesktopNotificationSettings {
        desktop_notification::DesktopNotificationSettings {
            enabled: false,
            events: Default::default(),
        }
    }

    fn ctx() -> AgentContext<'static> {
        AgentContext {
            agent: "claude",
            cwd: "/repo",
            worktree: &None,
        }
    }

    fn notification_event(wait_reason: &str, meta_only: bool) -> AgentEvent {
        AgentEvent::Notification {
            agent: "claude".into(),
            cwd: "/repo".into(),
            permission_mode: "default".into(),
            wait_reason: wait_reason.into(),
            meta_only,
            worktree: None,
            agent_id: None,
            session_id: None,
        }
    }

    #[test]
    fn on_teammate_idle_sets_attention_and_reason() {
        let store = MemPaneStore::new();
        let pane = "%TEAM";
        let exit = on_teammate_idle(pane, "alice", "", &store);
        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_ATTENTION), "notification");
        assert_eq!(
            store.get(pane, tmux::PANE_WAIT_REASON),
            "teammate_idle:alice"
        );
    }

    #[test]
    fn on_teammate_idle_includes_idle_reason_when_present() {
        let store = MemPaneStore::new();
        let pane = "%TEAM_REASON";
        on_teammate_idle(pane, "alice", "tokens_exhausted", &store);
        assert_eq!(
            store.get(pane, tmux::PANE_WAIT_REASON),
            "teammate_idle:alice:tokens_exhausted"
        );
    }

    #[test]
    fn on_notification_meta_only_skips_status_and_attention() {
        let store = MemPaneStore::new();
        let pane = "%NOTIF_META";
        on_notification(
            pane,
            &notification_event("permission", /* meta_only */ true),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        // meta_only=true must short-circuit before status/attention/wait_reason writes.
        assert!(!store.contains(pane, tmux::PANE_STATUS));
        assert!(!store.contains(pane, tmux::PANE_ATTENTION));
        assert!(!store.contains(pane, tmux::PANE_WAIT_REASON));
        // Agent meta should still be applied so the sidebar can render the pane.
        assert_eq!(store.get(pane, tmux::PANE_AGENT), "claude");
    }

    #[test]
    fn on_notification_sets_waiting_status_and_reason() {
        let store = MemPaneStore::new();
        let pane = "%NOTIF_WAIT";
        on_notification(
            pane,
            &notification_event("permission", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "waiting");
        assert_eq!(store.get(pane, tmux::PANE_ATTENTION), "notification");
        assert_eq!(store.get(pane, tmux::PANE_WAIT_REASON), "permission");
    }

    #[test]
    fn on_notification_keeps_background_when_softer_reason_and_bg_shell_live() {
        let store = MemPaneStore::new();
        let pane = "%NOTIF_BG_PREEMPT";
        store.insert(pane, tmux::PANE_BG_CMD, "cargo test");
        on_notification(
            pane,
            &notification_event("auth_success", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "background");
    }

    #[test]
    fn on_notification_permission_reason_preempts_background() {
        let store = MemPaneStore::new();
        let pane = "%NOTIF_PERM_OVER_BG";
        store.insert(pane, tmux::PANE_BG_CMD, "cargo test");
        on_notification(
            pane,
            &notification_event("permission_prompt", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "waiting");
    }

    #[test]
    fn on_notification_plain_permission_preempts_background() {
        // Claude's real `notification_type: "permission"` payload must
        // stay in `waiting` even with a live bg shell — the user has to
        // act on the prompt regardless.
        let store = MemPaneStore::new();
        let pane = "%NOTIF_PERM_PLAIN_OVER_BG";
        store.insert(pane, tmux::PANE_BG_CMD, "cargo test");
        on_notification(
            pane,
            &notification_event("permission", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "waiting");
    }

    #[test]
    fn on_notification_soft_reason_without_bg_still_sets_waiting() {
        let store = MemPaneStore::new();
        let pane = "%NOTIF_SOFT_NO_BG";
        on_notification(
            pane,
            &notification_event("auth_success", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "waiting");
    }

    #[test]
    fn on_notification_empty_wait_reason_clears_stale_value() {
        // Regression: an empty wait_reason used to be a no-op, which
        // left the previously-written reason on the pane. A later
        // notification that genuinely has no reason must drop the
        // stale one so the sidebar does not keep rendering the wrong
        // cause.
        let store = MemPaneStore::new();
        let pane = "%NOTIF_STALE";
        store.insert(pane, tmux::PANE_WAIT_REASON, "permission");

        on_notification(
            pane,
            &notification_event("", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );

        assert!(
            !store.contains(pane, tmux::PANE_WAIT_REASON),
            "empty wait_reason must clear a prior value"
        );
    }

    #[test]
    fn on_notification_stores_a_multiline_wait_reason_on_one_line() {
        // The apply-side encoder owns the newline/pipe/carriage-return
        // replacement, so a multi-line wait reason must come out of the
        // store as one field.
        let store = MemPaneStore::new();
        let pane = "%NOTIF_MULTILINE";
        on_notification(
            pane,
            &notification_event("line1\nline2|tail\rbeyond", /* meta_only */ false),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(
            store.get(pane, tmux::PANE_WAIT_REASON),
            "line1 line2 tail beyond"
        );
    }

    #[test]
    fn on_permission_denied_records_permission_denied_wait_reason() {
        let store = MemPaneStore::new();
        let pane = "%PD";
        on_permission_denied(
            pane,
            &AgentEvent::PermissionDenied {
                agent: "claude".into(),
                cwd: "/repo".into(),
                permission_mode: "default".into(),
                worktree: None,
                agent_id: None,
                session_id: None,
            },
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(store.get(pane, tmux::PANE_WAIT_REASON), "permission_denied");
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "waiting");
    }
}
