use crate::desktop_notification;
use crate::desktop_notification::DesktopNotificationKind;
use crate::event::{AgentEvent, Reported};
use crate::pane_state::transition;
use crate::pane_store::PaneStore;

use super::super::context::{AgentContext, mark_task_reset};
use super::super::notifications::{
    NotifyLabels, NotifyPayload, notification_run_id, notify_lifecycle, set_notification_run_id,
    stop_body, stop_failure_body, stop_failure_fingerprint, task_completed_body,
    task_completed_fingerprint,
};

pub(in crate::cli::hook) fn on_user_prompt_submit(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    store: &dyn PaneStore,
) -> i32 {
    let snapshot = store.snapshot(pane);
    // The notification run id is a call-site write (the transition does not
    // own it), but it rides the same batch so the event still costs one
    // apply.
    let mut writes = transition(&snapshot, event, reported);
    set_notification_run_id(&mut writes);
    let _ = store.apply(pane, &writes);
    0
}

pub(in crate::cli::hook) fn on_stop(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    ctx: &AgentContext<'_>,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    let AgentEvent::Stop {
        last_message,
        response,
        ..
    } = event
    else {
        return 0;
    };
    let snapshot = store.snapshot(pane);
    let bg_shell_live = snapshot
        .bg_cmd
        .as_deref()
        .is_some_and(|cmd| !cmd.is_empty());
    let writes = transition(&snapshot, event, reported);
    let _ = store.apply(pane, &writes);
    // The batch above cleared the stale `@pane_subagents` list, so the
    // task-reset marker must consult the store's post-batch state — reading
    // the option again is what decides whether the marker lands.
    mark_task_reset(pane, store);

    if !bg_shell_live {
        let run_id = notification_run_id(pane, store);
        // Skip the generic Stop notification if an explicit TaskCompleted
        // stamp from the current run has already fired — otherwise Claude
        // Code's `TaskCompleted` → `Stop` sequence produces two desktop
        // notifications for the same logical completion.
        let already_notified = desktop_notification::has_run_scoped_stamp(
            pane,
            DesktopNotificationKind::TaskCompleted,
            run_id,
        );
        if !already_notified {
            let _ = notify_lifecycle(
                pane,
                NotifyLabels::FromCtx(ctx),
                notifications,
                run_id,
                NotifyPayload {
                    kind: DesktopNotificationKind::TaskCompleted,
                    event: desktop_notification::DesktopNotificationEvent::Stop,
                    fingerprint_suffix: "stop",
                    body: &stop_body(last_message),
                },
                store,
            );
        }
    }
    if let Some(resp) = response {
        println!("{resp}");
    }
    0
}

pub(in crate::cli::hook) fn on_stop_failure(
    pane: &str,
    event: &AgentEvent,
    reported: &Reported,
    ctx: &AgentContext<'_>,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    let AgentEvent::StopFailure { error, .. } = event else {
        return 0;
    };
    let snapshot = store.snapshot(pane);
    let writes = transition(&snapshot, event, reported);
    let _ = store.apply(pane, &writes);
    mark_task_reset(pane, store);
    let _ = notify_lifecycle(
        pane,
        NotifyLabels::FromCtx(ctx),
        notifications,
        None,
        NotifyPayload {
            kind: DesktopNotificationKind::TaskFailed,
            event: desktop_notification::DesktopNotificationEvent::StopFailure,
            fingerprint_suffix: stop_failure_fingerprint(error),
            body: &stop_failure_body(error),
        },
        store,
    );
    0
}

pub(in crate::cli::hook) fn on_task_completed(
    pane: &str,
    agent_name: &str,
    task_id: &str,
    task_subject: &str,
    notifications: &desktop_notification::DesktopNotificationSettings,
    store: &dyn PaneStore,
) -> i32 {
    let _ = notify_lifecycle(
        pane,
        NotifyLabels::FromPane { agent: agent_name },
        notifications,
        None,
        NotifyPayload {
            kind: DesktopNotificationKind::TaskCompleted,
            event: desktop_notification::DesktopNotificationEvent::TaskCompleted,
            fingerprint_suffix: task_completed_fingerprint(task_id, task_subject),
            body: &task_completed_body(task_subject),
        },
        store,
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_state::SESSION_CONTEXT;
    use crate::pane_store::{MemPaneStore, PaneSnapshot, PaneWrites};
    use crate::pmux_task::{PmuxTask, PmuxTaskKind};
    use crate::tmux;
    use std::cell::Cell;

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

    fn prompt_event(prompt: &str, pmux_task: Option<PmuxTask>) -> AgentEvent {
        AgentEvent::UserPromptSubmit {
            agent: "claude".into(),
            cwd: "/repo".into(),
            permission_mode: "default".into(),
            prompt: prompt.into(),
            worktree: None,
            agent_id: None,
            session_id: None,
            pmux_task,
        }
    }

    fn stop_event(last_message: &str) -> AgentEvent {
        AgentEvent::Stop {
            agent: "claude".into(),
            cwd: "/repo".into(),
            permission_mode: "default".into(),
            last_message: last_message.into(),
            response: None,
            worktree: None,
            agent_id: None,
            session_id: None,
        }
    }

    fn stop_failure_event(error: &str) -> AgentEvent {
        AgentEvent::StopFailure {
            agent: "claude".into(),
            cwd: "/repo".into(),
            permission_mode: "default".into(),
            error: error.into(),
            worktree: None,
            agent_id: None,
            session_id: None,
        }
    }

    fn pmux_task(kind: PmuxTaskKind, parent: &str, task: Option<&str>) -> Option<PmuxTask> {
        Some(PmuxTask {
            kind,
            parent_session_id: parent.to_string(),
            task: task.map(str::to_string),
        })
    }

    /// A `MemPaneStore` that counts `apply` calls so a test can pin that a
    /// handler applies the transition output as one batch.
    struct CountingStore {
        inner: MemPaneStore,
        applies: Cell<usize>,
    }

    impl CountingStore {
        fn new() -> Self {
            CountingStore {
                inner: MemPaneStore::new(),
                applies: Cell::new(0),
            }
        }

        fn set(&self, pane: &str, key: &str, value: &str) {
            self.inner.insert(pane, key, value);
        }

        fn get(&self, pane: &str, key: &str) -> String {
            self.inner.get(pane, key)
        }

        fn contains(&self, pane: &str, key: &str) -> bool {
            self.inner.contains(pane, key)
        }

        fn applies(&self) -> usize {
            self.applies.get()
        }
    }

    impl PaneStore for CountingStore {
        fn snapshot(&self, pane: &str) -> PaneSnapshot {
            self.inner.snapshot(pane)
        }

        fn get(&self, pane: &str, key: &str) -> String {
            self.inner.get(pane, key)
        }

        fn apply(&self, pane: &str, writes: &PaneWrites) -> Result<(), String> {
            self.applies.set(self.applies.get() + 1);
            self.inner.apply(pane, writes)
        }
    }

    #[test]
    fn on_user_prompt_submit_sets_running_and_stores_prompt() {
        let store = MemPaneStore::new();
        let pane = "%PROMPT";
        let exit = on_user_prompt_submit(
            pane,
            &prompt_event("fix the bug", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "running");
        assert_eq!(store.get(pane, tmux::PANE_PROMPT), "fix the bug");
        assert_eq!(store.get(pane, tmux::PANE_PROMPT_SOURCE), "user");
        assert!(store.contains(pane, tmux::PANE_STARTED_AT));
    }

    #[test]
    fn on_user_prompt_submit_ignores_system_messages() {
        let store = MemPaneStore::new();
        let pane = "%SYS_PROMPT";
        on_user_prompt_submit(
            pane,
            &prompt_event("<system-reminder>ignore me</system-reminder>", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert!(
            !store.contains(pane, tmux::PANE_PROMPT),
            "system messages should not be stored as user prompt"
        );
        // But status should still advance to running.
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "running");
    }

    #[test]
    fn on_user_prompt_submit_clears_stale_wait_reason_but_preserves_bg_cmd() {
        let store = MemPaneStore::new();
        let pane = "%PROMPT_CLEAR_WAIT";
        store.insert(pane, tmux::PANE_WAIT_REASON, "permission");
        store.insert(pane, tmux::PANE_BG_CMD, "npm run dev");
        on_user_prompt_submit(
            pane,
            &prompt_event("new prompt", None),
            &SESSION_CONTEXT,
            &store,
        );
        assert!(!store.contains(pane, tmux::PANE_WAIT_REASON));
        assert_eq!(
            store.get(pane, tmux::PANE_BG_CMD),
            "npm run dev",
            "bg command must survive a new user turn — the shell is still running",
        );
    }

    // ─── pmux subagent identity ────────────────────────────────────

    #[test]
    fn pmux_spawn_match_sets_identity_keeps_prompt_and_runs() {
        let store = MemPaneStore::new();
        let pane = "%PMUX_SPAWN";
        store.insert(pane, tmux::PANE_PROMPT, "previous prompt");

        let exit = on_user_prompt_submit(
            pane,
            &prompt_event(
                "",
                pmux_task(PmuxTaskKind::Spawned, "P1", Some("unit1 idle collapse")),
            ),
            &SESSION_CONTEXT,
            &store,
        );

        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_KIND), "spawned");
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_PARENT), "P1");
        assert_eq!(
            store.get(pane, tmux::PANE_SUBAGENT_TASK),
            "unit1 idle collapse"
        );
        // A recognised pmux task prompt is not stored as the pane prompt.
        assert_eq!(store.get(pane, tmux::PANE_PROMPT), "previous prompt");
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "running");
    }

    #[test]
    fn later_plain_prompt_keeps_subagent_identity() {
        let store = MemPaneStore::new();
        let pane = "%PMUX_PLAIN";
        on_user_prompt_submit(
            pane,
            &prompt_event(
                "",
                pmux_task(PmuxTaskKind::Spawned, "P1", Some("unit1 idle collapse")),
            ),
            &SESSION_CONTEXT,
            &store,
        );

        on_user_prompt_submit(
            pane,
            &prompt_event("please continue", None),
            &SESSION_CONTEXT,
            &store,
        );

        assert_eq!(
            store.get(pane, tmux::PANE_SUBAGENT_KIND),
            "spawned",
            "a non-matching prompt must not touch the identity"
        );
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_PARENT), "P1");
        assert_eq!(
            store.get(pane, tmux::PANE_SUBAGENT_TASK),
            "unit1 idle collapse"
        );
    }

    #[test]
    fn later_resume_match_replaces_subagent_identity() {
        let store = MemPaneStore::new();
        let pane = "%PMUX_RESUME";
        on_user_prompt_submit(
            pane,
            &prompt_event(
                "",
                pmux_task(PmuxTaskKind::Spawned, "P1", Some("unit1 idle collapse")),
            ),
            &SESSION_CONTEXT,
            &store,
        );

        on_user_prompt_submit(
            pane,
            &prompt_event(
                "",
                pmux_task(PmuxTaskKind::Resumed, "P2", Some("unit2 placement")),
            ),
            &SESSION_CONTEXT,
            &store,
        );

        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_KIND), "resumed");
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_PARENT), "P2");
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_TASK), "unit2 placement");
    }

    #[test]
    fn pmux_match_without_task_name_unsets_the_task_option() {
        let store = MemPaneStore::new();
        let pane = "%PMUX_NO_TASK";
        // A previous identity with a task must not leak into the new one.
        store.insert(pane, tmux::PANE_SUBAGENT_TASK, "stale task");

        on_user_prompt_submit(
            pane,
            &prompt_event("", pmux_task(PmuxTaskKind::Resumed, "P3", None)),
            &SESSION_CONTEXT,
            &store,
        );

        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_KIND), "resumed");
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENT_PARENT), "P3");
        assert!(
            !store.contains(pane, tmux::PANE_SUBAGENT_TASK),
            "a match without a task name must unset the task option, not write an empty value"
        );
    }

    // ─── Stop / StopFailure ────────────────────────────────────────

    #[test]
    fn on_stop_applies_the_transition_as_one_batch() {
        let store = CountingStore::new();
        let pane = "%STOP_ONE_BATCH";
        store.set(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        store.set(pane, tmux::PANE_STARTED_AT, "123");

        on_stop(
            pane,
            &stop_event(""),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );

        assert_eq!(
            store.applies(),
            1,
            "on_stop must apply the transition output as one write batch"
        );
        assert!(!store.contains(pane, tmux::PANE_SUBAGENTS));
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "idle");
        assert!(!store.contains(pane, tmux::PANE_STARTED_AT));
    }

    #[test]
    fn on_stop_with_background_shell_sets_background_status() {
        let store = MemPaneStore::new();
        let pane = "%STOP_BG";
        store.insert(pane, tmux::PANE_BG_CMD, "npm run dev");
        store.insert(pane, tmux::PANE_STARTED_AT, "123");

        let exit = on_stop(
            pane,
            &stop_event(""),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );

        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "background");
        assert_eq!(store.get(pane, tmux::PANE_STARTED_AT), "123");
    }

    #[test]
    fn on_stop_without_background_shell_sets_idle_status() {
        let store = MemPaneStore::new();
        let pane = "%STOP_IDLE";
        store.insert(pane, tmux::PANE_STARTED_AT, "123");

        on_stop(
            pane,
            &stop_event(""),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );

        assert_eq!(store.get(pane, tmux::PANE_STATUS), "idle");
        assert!(!store.contains(pane, tmux::PANE_STARTED_AT));
    }

    #[test]
    fn on_stop_clears_stale_subagents() {
        let store = MemPaneStore::new();
        let pane = "%STOP_STALE_SUBAGENTS";
        store.insert(
            pane,
            tmux::PANE_SUBAGENTS,
            "general-purpose:sub-1,general-purpose:sub-2",
        );

        on_stop(
            pane,
            &stop_event(""),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );

        assert!(
            !store.contains(pane, tmux::PANE_SUBAGENTS),
            "parent Stop must clear stale subagent list"
        );
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "idle");
    }

    #[test]
    fn on_stop_failure_records_error_wait_reason_and_error_status() {
        let store = MemPaneStore::new();
        let pane = "%STOP_FAIL";
        let exit = on_stop_failure(
            pane,
            &stop_failure_event("rate_limit"),
            &SESSION_CONTEXT,
            &ctx(),
            &default_notifications(),
            &store,
        );
        assert_eq!(exit, 0);
        assert_eq!(store.get(pane, tmux::PANE_STATUS), "error");
        assert_eq!(store.get(pane, tmux::PANE_WAIT_REASON), "rate_limit");
    }
}
