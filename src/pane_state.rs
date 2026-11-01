//! The write rules for an agent pane's `@pane_*` options: one pure
//! [`transition`] per producer event, the [`status_writes`] helper that owns
//! the status↔attention coupling, and the [`identity_guarded`] subagent
//! check. The rules are stated here once; the hook handlers apply the
//! result through a [`crate::pane_store::PaneStore`] as one batch.
//!
//! The status resolvers (`resolve_stop_status`,
//! `resolve_notification_status`, `is_action_required_wait_reason`) live
//! here too, so `Stop` and `Notification` statuses come from the same
//! module that writes them. Priority (highest → lowest):
//!
//! ```text
//! running > permission > background > waiting > idle
//! ```
//!
//! Non-obvious rule: action-required wait reasons (the permission class and
//! a blocking user prompt) must stay as `waiting` even when a background
//! shell is alive — the user still has to act on the prompt, so a live bg
//! shell does not lower the signal.

use crate::event::{AgentEvent, Reported, WorktreeInfo};
use crate::pane_store::{PaneSnapshot, PaneWrites};
use crate::pmux_task::PmuxTask;
use crate::time::now_epoch_secs;
use crate::tmux::{
    PANE_AGENT, PANE_ATTENTION, PANE_CWD, PANE_PERMISSION_MODE, PANE_PROMPT, PANE_PROMPT_SOURCE,
    PANE_SESSION_ID, PANE_STARTED_AT, PANE_STATUS, PANE_SUBAGENT_KIND, PANE_SUBAGENT_PARENT,
    PANE_SUBAGENT_TASK, PANE_SUBAGENTS, PANE_WAIT_REASON, PANE_WORKTREE_BRANCH, PANE_WORKTREE_NAME,
};
use crate::ui::text::wait_reason_entry;

/// The session-context fields a Claude-shaped event reports, the same set
/// the adapters declare. Tests use it to stand in for a real adapter's
/// `reported()` output.
#[cfg(test)]
pub(crate) const SESSION_CONTEXT: Reported = Reported::new(&["session_id", "cwd", "worktree"]);

/// The option writes one producer event causes, in the order the handlers
/// imply them. The keys are the `@pane_*` constants; the values are raw —
/// the store encodes them at `apply` time.
///
/// What the transition does not own stays with the handlers: the activity
/// log and the desktop notifications themselves, the notification run id
/// and dedup stamps, the task-store clear, and the pending teardown
/// markers. `ActivityLog` moves here in its own change step; `SessionEnd`,
/// `SubagentStart`, `SubagentStop` and `WorktreeRemove` keep their guarded
/// plain-list and read-modify-write paths; `TaskCreated` and
/// `WorktreeCreate` write nothing.
///
/// The one deliberate divergence from today's handler output is the
/// reported-fields rule: a reported field with an absent-or-empty value
/// clears its option, which changes `cwd` — an absent or empty `cwd` used
/// to be ignored, and now clears `@pane_cwd`.
pub fn transition(snapshot: &PaneSnapshot, event: &AgentEvent, reported: &Reported) -> PaneWrites {
    let mut writes = PaneWrites::new();
    match event {
        AgentEvent::SessionStart {
            agent,
            cwd,
            permission_mode,
            source,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            writes.unset(PANE_ATTENTION);
            writes.unset(PANE_STARTED_AT);
            writes.unset(PANE_WAIT_REASON);
            writes.unset(PANE_PROMPT);
            writes.unset(PANE_PROMPT_SOURCE);
            match source.as_str() {
                "resume" => writes.set(PANE_WAIT_REASON, "session_resumed"),
                "compact" => writes.set(PANE_WAIT_REASON, "session_resumed_compact"),
                _ => writes.unset(PANE_WAIT_REASON),
            }
            writes.merge(&status_writes("idle"));
        }
        AgentEvent::UserPromptSubmit {
            agent,
            cwd,
            permission_mode,
            prompt,
            worktree,
            session_id,
            pmux_task,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            writes.unset(PANE_ATTENTION);
            writes.merge(&status_writes("running"));
            if !prompt.is_empty() && !is_system_message(prompt) {
                writes.set(PANE_PROMPT, prompt.as_str());
                writes.set(PANE_PROMPT_SOURCE, "user");
            }
            if let Some(task) = pmux_task {
                push_pmux_identity(&mut writes, task);
            }
            writes.set(PANE_STARTED_AT, now_epoch_secs().to_string());
            writes.unset(PANE_WAIT_REASON);
        }
        AgentEvent::Stop {
            agent,
            cwd,
            permission_mode,
            last_message,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            writes.unset(PANE_ATTENTION);
            if !last_message.is_empty() {
                writes.set(PANE_PROMPT, last_message.as_str());
                writes.set(PANE_PROMPT_SOURCE, "response");
            }
            let bg_shell_live = live_bg_shell(snapshot);
            // `Stop` is emitted for the parent turn, and Claude Code `Task`
            // subagents are synchronous: once the parent reaches Stop, no
            // child should still be running. Treat any leftover list as
            // stale state from a missed or mismatched SubagentStop and
            // clear it before `mark_task_reset`, whose guard intentionally
            // skips writes while subagents are active. The clear itself is
            // not guarded — it is what ends the guard.
            writes.unset(PANE_SUBAGENTS);
            if bg_shell_live {
                writes.unset(PANE_WAIT_REASON);
            } else {
                writes.unset(PANE_STARTED_AT);
                writes.unset(PANE_WAIT_REASON);
            }
            writes.merge(&status_writes(resolve_stop_status(bg_shell_live)));
        }
        AgentEvent::StopFailure {
            agent,
            cwd,
            permission_mode,
            error,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            writes.unset(PANE_ATTENTION);
            writes.unset(PANE_STARTED_AT);
            writes.unset(PANE_WAIT_REASON);
            if !error.is_empty() {
                writes.set(PANE_WAIT_REASON, error.as_str());
            }
            writes.merge(&status_writes("error"));
        }
        AgentEvent::Notification {
            agent,
            cwd,
            permission_mode,
            wait_reason,
            meta_only,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            if *meta_only {
                return writes;
            }
            let bg_shell_live = live_bg_shell(snapshot);
            writes.merge(&status_writes(resolve_notification_status(
                wait_reason,
                bg_shell_live,
            )));
            writes.set(PANE_ATTENTION, "notification");
            if wait_reason.is_empty() {
                writes.unset(PANE_WAIT_REASON);
            } else {
                writes.set(PANE_WAIT_REASON, wait_reason.as_str());
            }
        }
        AgentEvent::PermissionDenied {
            agent,
            cwd,
            permission_mode,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                Some(agent),
                Some(permission_mode),
                cwd,
                worktree,
                session_id,
                reported,
            );
            writes.merge(&status_writes("waiting"));
            writes.set(PANE_ATTENTION, "notification");
            writes.set(PANE_WAIT_REASON, "permission_denied");
        }
        AgentEvent::CwdChanged {
            cwd,
            worktree,
            session_id,
            ..
        } => {
            push_identity(
                &mut writes,
                snapshot,
                None,
                None,
                cwd,
                worktree,
                session_id,
                reported,
            );
        }
        AgentEvent::TaskCompleted { .. } => {
            writes.set(PANE_ATTENTION, "notification");
        }
        AgentEvent::TeammateIdle {
            teammate_name,
            idle_reason,
            ..
        } => {
            writes.set(PANE_ATTENTION, "notification");
            let reason = if idle_reason.is_empty() {
                format!("teammate_idle:{teammate_name}")
            } else {
                format!("teammate_idle:{teammate_name}:{idle_reason}")
            };
            writes.set(PANE_WAIT_REASON, reason);
        }
        // The events above carry the transition's rules; every other event
        // keeps today's write path for now (see the module docs).
        _ => {}
    }
    writes
}

/// The option writes one status change causes, including the attention
/// handling `cmd_set_status` carries: a status of `clear` unsets the
/// status option and the attention flag, `running` and `idle` unset the
/// attention flag, and every other status leaves the attention flag
/// alone. The single owner of that coupling — the transition and
/// `cmd_set_status` both apply it.
pub fn status_writes(status: &str) -> PaneWrites {
    let mut writes = PaneWrites::new();
    if status == "clear" {
        writes.unset(PANE_STATUS);
        writes.unset(PANE_ATTENTION);
    } else {
        writes.set(PANE_STATUS, status);
        if matches!(status, "running" | "idle") {
            writes.unset(PANE_ATTENTION);
        }
    }
    writes
}

/// Whether the identity-scoped writes of an event may touch the pane:
/// false while the snapshot shows active subagents, true otherwise.
///
/// Subagents share the parent's `$TMUX_PANE`, so a child's hook events
/// must not rewrite the parent's identity. The guarded set is the spec's:
/// the agent label, the permission mode, the session id, the working
/// directory and the worktree pair, the task reset, the task-store clear,
/// the worktree-remove teardown, and `SessionEnd`. The guard does not
/// cover status, attention, prompt or the response preview — a child's
/// progress is still visible while it runs.
pub fn identity_guarded(snapshot: &PaneSnapshot) -> bool {
    snapshot.subagents.as_deref().is_none_or(str::is_empty)
}

/// Wait reasons that demand direct user action and must stay visible as
/// `waiting` even with a live background shell.
///
/// The class comes from the shared wait-reason table in `ui::text`, so a
/// reason mapped for display cannot silently stay demotable.
pub fn is_action_required_wait_reason(wait_reason: &str) -> bool {
    wait_reason_entry(wait_reason).is_some_and(|entry| entry.action_required)
}

/// Status `Stop` should land in.
pub fn resolve_stop_status(bg_shell_live: bool) -> &'static str {
    if bg_shell_live { "background" } else { "idle" }
}

/// Status `Notification` should land in.
pub fn resolve_notification_status(wait_reason: &str, bg_shell_live: bool) -> &'static str {
    if bg_shell_live && !is_action_required_wait_reason(wait_reason) {
        "background"
    } else {
        "waiting"
    }
}

/// Resolve the effective cwd for pane metadata.
/// When a worktree is active, prefer `original_repo_dir` so the sidebar
/// groups the pane under the original repository, not the worktree path.
pub fn resolve_cwd<'a>(raw_cwd: &'a str, worktree: &'a Option<WorktreeInfo>) -> &'a str {
    if let Some(wt) = worktree
        && !wt.original_repo_dir.is_empty()
    {
        return &wt.original_repo_dir;
    }
    raw_cwd
}

/// Check if a prompt is a system-injected message (not a real user prompt).
pub fn is_system_message(s: &str) -> bool {
    s.contains("<task-notification>") || s.contains("<system-reminder>") || s.contains("<task-")
}

/// Whether the snapshot shows a live background shell: `@pane_bg_cmd`
/// present and non-empty.
fn live_bg_shell(snapshot: &PaneSnapshot) -> bool {
    snapshot
        .bg_cmd
        .as_deref()
        .is_some_and(|cmd| !cmd.is_empty())
}

/// Append the identity-scoped writes of one event: the agent label, the
/// permission mode, the session id, the working directory and the worktree
/// pair. Every one of them is skipped while subagents are active, and each
/// field also answers to the reported-fields rule: a field the event does
/// not report is left alone, and a reported field with an empty value
/// clears its option.
///
/// `agent` and `permission_mode` are `None` for the events that carry no
/// such field (`CwdChanged`); the agent label is written whenever the
/// event names one and the guard allows it.
#[allow(clippy::too_many_arguments)]
fn push_identity(
    writes: &mut PaneWrites,
    snapshot: &PaneSnapshot,
    agent: Option<&str>,
    permission_mode: Option<&str>,
    cwd: &str,
    worktree: &Option<WorktreeInfo>,
    session_id: &Option<String>,
    reported: &Reported,
) {
    if !identity_guarded(snapshot) {
        return;
    }
    if let Some(agent) = agent {
        writes.set(PANE_AGENT, agent);
    }
    if let Some(permission_mode) = permission_mode
        && !permission_mode.is_empty()
    {
        writes.set(PANE_PERMISSION_MODE, permission_mode);
    }
    if reported.contains("session_id") {
        match session_id.as_deref() {
            Some(sid) if !sid.is_empty() => writes.set(PANE_SESSION_ID, sid),
            _ => writes.unset(PANE_SESSION_ID),
        }
    }
    if reported.contains("cwd") {
        if cwd.is_empty() {
            writes.unset(PANE_CWD);
        } else {
            writes.set(PANE_CWD, resolve_cwd(cwd, worktree));
        }
    }
    if reported.contains("worktree") {
        push_worktree(writes, worktree);
    }
}

/// Append the worktree name and branch writes. A present worktree sets
/// each field it carries and clears each field that is empty, so an
/// explicit "no longer in this worktree branch" payload cannot leave
/// stale metadata behind; an absent worktree clears both.
fn push_worktree(writes: &mut PaneWrites, worktree: &Option<WorktreeInfo>) {
    match worktree {
        Some(wt) => {
            if wt.name.is_empty() {
                writes.unset(PANE_WORKTREE_NAME);
            } else {
                writes.set(PANE_WORKTREE_NAME, wt.name.as_str());
            }
            if wt.branch.is_empty() {
                writes.unset(PANE_WORKTREE_BRANCH);
            } else {
                writes.set(PANE_WORKTREE_BRANCH, wt.branch.as_str());
            }
        }
        None => {
            writes.unset(PANE_WORKTREE_NAME);
            writes.unset(PANE_WORKTREE_BRANCH);
        }
    }
}

/// Append the pmux subagent identity carried by a recognised task prompt.
/// A later match replaces the earlier identity; the task option is unset
/// when the matched prompt had no usable task name.
fn push_pmux_identity(writes: &mut PaneWrites, task: &PmuxTask) {
    writes.set(PANE_SUBAGENT_KIND, task.kind.as_str());
    writes.set(PANE_SUBAGENT_PARENT, task.parent_session_id.as_str());
    match &task.task {
        Some(name) => writes.set(PANE_SUBAGENT_TASK, name.as_str()),
        None => writes.unset(PANE_SUBAGENT_TASK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_store::PaneWrite;
    use crate::pmux_task::PmuxTaskKind;

    const AGENT: &str = "claude";
    const CWD: &str = "/repo";
    const PM: &str = "plan";
    const SES: &str = "sess-1";

    /// The previous statuses the transition must be independent of.
    const PREVIOUS_STATUSES: [&str; 4] = ["idle", "running", "waiting", "background"];

    fn snapshot_with_status(status: &str) -> PaneSnapshot {
        PaneSnapshot {
            status: Some(status.into()),
            ..PaneSnapshot::default()
        }
    }

    fn snapshot_with(status: &str, bg_cmd: &str, subagents: &str) -> PaneSnapshot {
        PaneSnapshot {
            status: Some(status.into()),
            bg_cmd: (!bg_cmd.is_empty()).then(|| bg_cmd.into()),
            subagents: (!subagents.is_empty()).then(|| subagents.into()),
            started_at: Some("1700".into()),
        }
    }

    fn entries(writes: &PaneWrites) -> Vec<(&'static str, Option<String>)> {
        writes
            .iter()
            .map(|write| match write {
                PaneWrite::Set(key, value) => (*key, Some(value.clone())),
                PaneWrite::Unset(key) => (*key, None),
            })
            .collect()
    }

    fn expected(list: &[(&'static str, Option<&str>)]) -> Vec<(&'static str, Option<String>)> {
        list.iter()
            .map(|(key, value)| (*key, value.map(str::to_string)))
            .collect()
    }

    fn worktree(name: &str, branch: &str) -> Option<WorktreeInfo> {
        Some(WorktreeInfo {
            name: name.into(),
            path: "/wt".into(),
            branch: branch.into(),
            original_repo_dir: CWD.into(),
        })
    }

    fn pmux_task(kind: PmuxTaskKind, parent: &str, task: Option<&str>) -> Option<PmuxTask> {
        Some(PmuxTask {
            kind,
            parent_session_id: parent.into(),
            task: task.map(str::to_string),
        })
    }

    fn session_start(source: &str) -> AgentEvent {
        AgentEvent::SessionStart {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            source: source.into(),
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn user_prompt_submit(prompt: &str) -> AgentEvent {
        AgentEvent::UserPromptSubmit {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            prompt: prompt.into(),
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
            pmux_task: None,
        }
    }

    fn stop(last_message: &str) -> AgentEvent {
        AgentEvent::Stop {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            last_message: last_message.into(),
            response: None,
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn stop_with_worktree(worktree: Option<WorktreeInfo>) -> AgentEvent {
        AgentEvent::Stop {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            last_message: String::new(),
            response: None,
            worktree,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn stop_failure(error: &str) -> AgentEvent {
        AgentEvent::StopFailure {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            error: error.into(),
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn notification(wait_reason: &str, meta_only: bool) -> AgentEvent {
        AgentEvent::Notification {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            wait_reason: wait_reason.into(),
            meta_only,
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn permission_denied() -> AgentEvent {
        AgentEvent::PermissionDenied {
            agent: AGENT.into(),
            cwd: CWD.into(),
            permission_mode: PM.into(),
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        }
    }

    fn cwd_changed(cwd: &str, session_id: Option<&str>) -> AgentEvent {
        AgentEvent::CwdChanged {
            cwd: cwd.into(),
            worktree: None,
            agent_id: None,
            session_id: session_id.map(str::to_string),
        }
    }

    fn teammate_idle(idle_reason: &str) -> AgentEvent {
        AgentEvent::TeammateIdle {
            teammate_name: "alice".into(),
            team_name: "team".into(),
            idle_reason: idle_reason.into(),
        }
    }

    // ─── 3.1 one expected write list per event, across previous statuses ──

    #[test]
    fn session_start_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &session_start(""),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    // set_agent_meta
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    // set_attention("clear")
                    (PANE_ATTENTION, None),
                    // clear_run_state
                    (PANE_STARTED_AT, None),
                    (PANE_WAIT_REASON, None),
                    // stale prompt dropped
                    (PANE_PROMPT, None),
                    (PANE_PROMPT_SOURCE, None),
                    // source "" drops a stale wait reason
                    (PANE_WAIT_REASON, None),
                    // set_status("idle")
                    (PANE_STATUS, Some("idle")),
                    (PANE_ATTENTION, None),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn session_start_resume_sets_the_resume_wait_reason_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &session_start("resume"),
                &SESSION_CONTEXT,
            );
            let list = entries(&writes);
            assert!(
                list.contains(&(PANE_WAIT_REASON, Some("session_resumed".into()))),
                "previous status {status}"
            );
            assert!(list.contains(&(PANE_STATUS, Some("idle".into()))));
        }
    }

    #[test]
    fn user_prompt_submit_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &user_prompt_submit("fix the bug"),
                &SESSION_CONTEXT,
            );
            let list = entries(&writes);
            // The start stamp is the current epoch seconds; pin its presence
            // and shape, not the clock reading.
            let started = list
                .iter()
                .find(|(key, _)| *key == PANE_STARTED_AT)
                .unwrap_or_else(|| panic!("previous status {status}: no start stamp"));
            assert!(
                matches!(started, (PANE_STARTED_AT, Some(v))
                    if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit())),
                "started_at must be a fresh epoch-seconds stamp, got {started:?}"
            );
            let without_clock: Vec<_> = list
                .into_iter()
                .filter(|(key, _)| *key != PANE_STARTED_AT)
                .collect();
            assert_eq!(
                without_clock,
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    // set_attention("clear")
                    (PANE_ATTENTION, None),
                    // set_status("running")
                    (PANE_STATUS, Some("running")),
                    (PANE_ATTENTION, None),
                    // the prompt itself
                    (PANE_PROMPT, Some("fix the bug")),
                    (PANE_PROMPT_SOURCE, Some("user")),
                    (PANE_WAIT_REASON, None),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn user_prompt_submit_skips_the_prompt_write_for_system_and_empty_prompts() {
        for prompt in ["<system-reminder>noise</system-reminder>", ""] {
            for status in PREVIOUS_STATUSES {
                let writes = transition(
                    &snapshot_with_status(status),
                    &user_prompt_submit(prompt),
                    &SESSION_CONTEXT,
                );
                let entries = entries(&writes);
                assert!(
                    !entries.iter().any(|(key, _)| *key == PANE_PROMPT),
                    "prompt {prompt:?} must not be stored (status {status})"
                );
                assert!(!entries.iter().any(|(key, _)| *key == PANE_PROMPT_SOURCE));
                // The run still advances.
                assert!(entries.contains(&(PANE_STATUS, Some("running".into()))));
            }
        }
    }

    #[test]
    fn user_prompt_submit_stores_the_pmux_subagent_identity() {
        for status in PREVIOUS_STATUSES {
            let event = AgentEvent::UserPromptSubmit {
                agent: "pi".into(),
                cwd: CWD.into(),
                permission_mode: PM.into(),
                prompt: "".into(),
                worktree: None,
                agent_id: None,
                session_id: Some(SES.into()),
                pmux_task: pmux_task(PmuxTaskKind::Spawned, "P1", Some("unit1 idle collapse")),
            };
            let writes = transition(&snapshot_with_status(status), &event, &SESSION_CONTEXT);
            let list = entries(&writes);
            assert!(list.contains(&(PANE_SUBAGENT_KIND, Some("spawned".into()))));
            assert!(list.contains(&(PANE_SUBAGENT_PARENT, Some("P1".into()))));
            assert!(list.contains(&(PANE_SUBAGENT_TASK, Some("unit1 idle collapse".into()))));

            // A match without a task name unsets the stale task option.
            let event = AgentEvent::UserPromptSubmit {
                agent: "pi".into(),
                cwd: CWD.into(),
                permission_mode: PM.into(),
                prompt: "".into(),
                worktree: None,
                agent_id: None,
                session_id: Some(SES.into()),
                pmux_task: pmux_task(PmuxTaskKind::Resumed, "P2", None),
            };
            let writes = transition(&snapshot_with_status(status), &event, &SESSION_CONTEXT);
            let list = entries(&writes);
            assert!(list.contains(&(PANE_SUBAGENT_KIND, Some("resumed".into()))));
            assert!(list.contains(&(PANE_SUBAGENT_PARENT, Some("P2".into()))));
            assert!(list.contains(&(PANE_SUBAGENT_TASK, None)));
        }
    }

    #[test]
    fn stop_without_background_shell_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &stop("all done"),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    // set_attention("clear")
                    (PANE_ATTENTION, None),
                    // the response preview
                    (PANE_PROMPT, Some("all done")),
                    (PANE_PROMPT_SOURCE, Some("response")),
                    // the stale subagent list is cleared
                    (PANE_SUBAGENTS, None),
                    // clear_run_state: no live bg shell
                    (PANE_STARTED_AT, None),
                    (PANE_WAIT_REASON, None),
                    // set_status("idle")
                    (PANE_STATUS, Some("idle")),
                    (PANE_ATTENTION, None),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn stop_with_background_shell_lands_in_background_and_keeps_the_run_state() {
        for status in PREVIOUS_STATUSES {
            let snapshot = PaneSnapshot {
                bg_cmd: Some("npm run dev".into()),
                ..snapshot_with_status(status)
            };
            let writes = transition(&snapshot, &stop(""), &SESSION_CONTEXT);
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    (PANE_ATTENTION, None),
                    // an empty last message stores no prompt
                    (PANE_SUBAGENTS, None),
                    // a live bg shell keeps the run state, only the wait
                    // reason goes
                    (PANE_WAIT_REASON, None),
                    // set_status("background") — attention is left alone here
                    (PANE_STATUS, Some("background")),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn stop_failure_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &stop_failure("rate_limit"),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    (PANE_ATTENTION, None),
                    // clear_run_state
                    (PANE_STARTED_AT, None),
                    (PANE_WAIT_REASON, None),
                    // the failure reason
                    (PANE_WAIT_REASON, Some("rate_limit")),
                    // set_status("error")
                    (PANE_STATUS, Some("error")),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn notification_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &notification("permission", false),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    // set_status("waiting") — attention untouched by the status
                    (PANE_STATUS, Some("waiting")),
                    // set_attention("notification")
                    (PANE_ATTENTION, Some("notification")),
                    (PANE_WAIT_REASON, Some("permission")),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn notification_soft_reason_with_live_bg_shell_lands_in_background() {
        for status in PREVIOUS_STATUSES {
            let snapshot = PaneSnapshot {
                bg_cmd: Some("cargo test".into()),
                ..snapshot_with_status(status)
            };
            let writes = transition(
                &snapshot,
                &notification("auth_success", false),
                &SESSION_CONTEXT,
            );
            let entries = entries(&writes);
            assert!(entries.contains(&(PANE_STATUS, Some("background".into()))));
            assert!(entries.contains(&(PANE_ATTENTION, Some("notification".into()))));
        }
    }

    #[test]
    fn notification_action_required_reason_preempts_a_live_bg_shell() {
        for reason in ["permission", "permission_prompt", "permission_denied"] {
            let snapshot = PaneSnapshot {
                bg_cmd: Some("cargo test".into()),
                ..snapshot_with_status("background")
            };
            let writes = transition(&snapshot, &notification(reason, false), &SESSION_CONTEXT);
            assert!(entries(&writes).contains(&(PANE_STATUS, Some("waiting".into()))));
        }
    }

    #[test]
    fn notification_meta_only_writes_the_identity_only() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &notification("permission", true),
                &SESSION_CONTEXT,
            );
            let list = expected(&[
                (PANE_AGENT, Some(AGENT)),
                (PANE_PERMISSION_MODE, Some(PM)),
                (PANE_SESSION_ID, Some(SES)),
                (PANE_CWD, Some(CWD)),
                (PANE_WORKTREE_NAME, None),
                (PANE_WORKTREE_BRANCH, None),
            ]);
            assert_eq!(entries(&writes), list, "previous status {status}");
        }
    }

    #[test]
    fn permission_denied_writes_the_handlers_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &permission_denied(),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_AGENT, Some(AGENT)),
                    (PANE_PERMISSION_MODE, Some(PM)),
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some(CWD)),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                    (PANE_STATUS, Some("waiting")),
                    (PANE_ATTENTION, Some("notification")),
                    (PANE_WAIT_REASON, Some("permission_denied")),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn cwd_changed_writes_the_location_options_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &cwd_changed("/other", Some(SES)),
                &SESSION_CONTEXT,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    // no agent label or permission mode: the event carries none
                    (PANE_SESSION_ID, Some(SES)),
                    (PANE_CWD, Some("/other")),
                    (PANE_WORKTREE_NAME, None),
                    (PANE_WORKTREE_BRANCH, None),
                ]),
                "previous status {status}"
            );
        }
    }

    #[test]
    fn task_completed_sets_the_attention_flag_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let event = AgentEvent::TaskCompleted {
                task_id: "t1".into(),
                task_subject: "ship it".into(),
            };
            let writes = transition(&snapshot_with_status(status), &event, &Reported::NONE);
            assert_eq!(
                entries(&writes),
                expected(&[(PANE_ATTENTION, Some("notification"))])
            );
        }
    }

    #[test]
    fn teammate_idle_sets_attention_and_the_wait_reason_for_every_previous_status() {
        for status in PREVIOUS_STATUSES {
            let writes = transition(
                &snapshot_with_status(status),
                &teammate_idle(""),
                &Reported::NONE,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_ATTENTION, Some("notification")),
                    (PANE_WAIT_REASON, Some("teammate_idle:alice")),
                ])
            );

            let writes = transition(
                &snapshot_with_status(status),
                &teammate_idle("tokens_exhausted"),
                &Reported::NONE,
            );
            assert_eq!(
                entries(&writes),
                expected(&[
                    (PANE_ATTENTION, Some("notification")),
                    (
                        PANE_WAIT_REASON,
                        Some("teammate_idle:alice:tokens_exhausted")
                    ),
                ])
            );
        }
    }

    #[test]
    fn events_that_keep_their_own_paths_write_nothing() {
        let snapshot = snapshot_with_status("running");
        let reported = SESSION_CONTEXT;
        for event in [
            AgentEvent::SessionEnd {
                end_reason: "clear".into(),
            },
            AgentEvent::SubagentStart {
                agent_type: "Explore".into(),
                agent_id: Some("sub-1".into()),
            },
            AgentEvent::SubagentStop {
                agent_type: "Explore".into(),
                agent_id: Some("sub-1".into()),
                last_message: String::new(),
                transcript_path: String::new(),
            },
            AgentEvent::ActivityLog {
                tool_name: "Bash".into(),
                tool_input: serde_json::Value::Null,
                tool_response: serde_json::Value::Null,
            },
            AgentEvent::TaskCreated {
                task_id: "t1".into(),
                task_subject: "ship it".into(),
            },
            AgentEvent::WorktreeCreate,
            AgentEvent::WorktreeRemove {
                worktree_path: "/wt".into(),
            },
        ] {
            let writes = transition(&snapshot, &event, &reported);
            assert!(
                writes.is_empty(),
                "{event:?} must not write through the transition yet"
            );
        }
    }

    // ─── 3.2 the subagent guard ────────────────────────────────────────

    const SUBAGENTS: &str = "Explore:sub-1";

    #[test]
    fn identity_guarded_reads_the_snapshot_subagent_list() {
        assert!(identity_guarded(&snapshot_with("running", "", "")));
        // An option set to an empty list reads as no subagents.
        assert!(identity_guarded(&snapshot_with("running", "", "")));
        assert!(!identity_guarded(&snapshot_with("running", "", SUBAGENTS)));
        assert!(!identity_guarded(&snapshot_with(
            "running",
            "",
            "Explore:sub-1,Plan:sub-2"
        )));
    }

    #[test]
    fn a_child_event_with_another_session_id_keeps_the_pane_identity() {
        let snapshot = snapshot_with("running", "", SUBAGENTS);
        let child = AgentEvent::SessionStart {
            agent: "claude".into(),
            cwd: "/repo/child".into(),
            permission_mode: "default".into(),
            source: String::new(),
            worktree: worktree("child-feat", "feat/child"),
            agent_id: None,
            session_id: Some("child-session".into()),
        };

        let writes = transition(&snapshot, &child, &SESSION_CONTEXT);

        let entries = entries(&writes);
        for key in [
            PANE_AGENT,
            PANE_PERMISSION_MODE,
            PANE_SESSION_ID,
            PANE_CWD,
            PANE_WORKTREE_NAME,
            PANE_WORKTREE_BRANCH,
        ] {
            assert!(
                !entries.iter().any(|(entry_key, _)| entry_key == &key),
                "{key} must not be written while subagents are active"
            );
        }
    }

    #[test]
    fn a_child_stop_still_writes_status_attention_and_prompt() {
        let snapshot = snapshot_with("running", "", SUBAGENTS);
        let writes = transition(&snapshot, &stop("child done"), &SESSION_CONTEXT);
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_STATUS, Some("idle".into()))));
        assert!(entries.contains(&(PANE_ATTENTION, None)));
        assert!(entries.contains(&(PANE_PROMPT, Some("child done".into()))));
        assert!(entries.contains(&(PANE_PROMPT_SOURCE, Some("response".into()))));
    }

    #[test]
    fn a_child_prompt_still_writes_status_attention_and_prompt() {
        let snapshot = snapshot_with("idle", "", SUBAGENTS);
        let writes = transition(&snapshot, &user_prompt_submit("go on"), &SESSION_CONTEXT);
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_STATUS, Some("running".into()))));
        assert!(
            entries
                .iter()
                .filter(|(key, _)| *key == PANE_ATTENTION)
                .count()
                >= 1
        );
        assert!(entries.contains(&(PANE_PROMPT, Some("go on".into()))));
        assert!(entries.contains(&(PANE_PROMPT_SOURCE, Some("user".into()))));
    }

    #[test]
    fn the_last_subagent_stopping_reopens_the_identity_writes() {
        // The same event with the subagent list drained writes the identity
        // again — the guard reads the snapshot, so the next event after the
        // last SubagentStop updates the options.
        let snapshot = snapshot_with("idle", "", "");
        let writes = transition(&snapshot, &session_start(""), &SESSION_CONTEXT);
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_AGENT, Some(AGENT.into()))));
        assert!(entries.contains(&(PANE_SESSION_ID, Some(SES.into()))));
    }

    #[test]
    fn a_child_event_with_another_cwd_clears_nothing_and_sets_nothing() {
        let snapshot = snapshot_with("running", "", SUBAGENTS);
        let writes = transition(
            &snapshot,
            &cwd_changed("/repo/child", Some("child-ses")),
            &SESSION_CONTEXT,
        );
        assert!(writes.is_empty(), "a guarded CwdChanged writes nothing");
    }

    // ─── 3.3 the reported-fields rule ──────────────────────────────────

    #[test]
    fn a_pi_stop_keeps_the_worktree() {
        // pi's `stop` reports cwd and session_id but not worktree, so the
        // stored worktree pair survives even though the payload carries none.
        let pi_stop_reported = Reported::new(&["last_assistant_message", "cwd", "session_id"]);
        let event = AgentEvent::Stop {
            agent: "pi".into(),
            cwd: CWD.into(),
            permission_mode: String::new(),
            last_message: String::new(),
            response: None,
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        };
        let writes = transition(&snapshot_with_status("running"), &event, &pi_stop_reported);
        let entries = entries(&writes);
        assert!(
            !entries
                .iter()
                .any(|(key, _)| *key == PANE_WORKTREE_NAME || *key == PANE_WORKTREE_BRANCH),
            "a pi stop must not touch the worktree pair"
        );
        // The fields it does report are still written.
        assert!(entries.contains(&(PANE_SESSION_ID, Some(SES.into()))));
        assert!(entries.contains(&(PANE_CWD, Some(CWD.into()))));
    }

    #[test]
    fn a_claude_stop_without_a_worktree_clears_the_worktree_pair() {
        let writes = transition(
            &snapshot_with_status("running"),
            &stop(""),
            &SESSION_CONTEXT,
        );
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_WORKTREE_NAME, None)));
        assert!(entries.contains(&(PANE_WORKTREE_BRANCH, None)));
    }

    #[test]
    fn a_reporting_event_with_a_worktree_sets_the_pair_and_resolves_the_cwd() {
        let event = stop_with_worktree(worktree("feat-x", "feat/x"));
        let writes = transition(&snapshot_with_status("running"), &event, &SESSION_CONTEXT);
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_WORKTREE_NAME, Some("feat-x".into()))));
        assert!(entries.contains(&(PANE_WORKTREE_BRANCH, Some("feat/x".into()))));
        // resolve_cwd prefers the worktree's original repository directory.
        assert!(entries.contains(&(PANE_CWD, Some(CWD.into()))));
    }

    #[test]
    fn a_reporting_event_with_a_partially_empty_worktree_clears_the_empty_field() {
        let event = stop_with_worktree(worktree("new-name", ""));
        let writes = transition(&snapshot_with_status("running"), &event, &SESSION_CONTEXT);
        let entries = entries(&writes);
        assert!(entries.contains(&(PANE_WORKTREE_NAME, Some("new-name".into()))));
        assert!(entries.contains(&(PANE_WORKTREE_BRANCH, None)));
    }

    #[test]
    fn a_reporting_notification_without_cwd_clears_it() {
        let event = AgentEvent::Notification {
            agent: AGENT.into(),
            cwd: String::new(),
            permission_mode: PM.into(),
            wait_reason: "permission".into(),
            meta_only: false,
            worktree: None,
            agent_id: None,
            session_id: Some(SES.into()),
        };
        let writes = transition(&snapshot_with_status("running"), &event, &SESSION_CONTEXT);
        assert!(entries(&writes).contains(&(PANE_CWD, None)));
    }

    #[test]
    fn a_reporting_event_with_an_empty_session_id_clears_it() {
        for session_id in [None, Some(String::new())] {
            let event = AgentEvent::Stop {
                agent: AGENT.into(),
                cwd: CWD.into(),
                permission_mode: PM.into(),
                last_message: String::new(),
                response: None,
                worktree: None,
                agent_id: None,
                session_id,
            };
            let writes = transition(&snapshot_with_status("running"), &event, &SESSION_CONTEXT);
            assert!(
                entries(&writes).contains(&(PANE_SESSION_ID, None)),
                "an empty session id must clear the option"
            );
        }
    }

    #[test]
    fn an_unreported_field_keeps_the_stored_value() {
        // An event that reports nothing (an invalid payload, or an event
        // whose adapter declares no session context) clears nothing and
        // sets nothing in the identity fields — even though its extracted
        // fields are all empty.
        let event = AgentEvent::Notification {
            agent: AGENT.into(),
            cwd: String::new(),
            permission_mode: PM.into(),
            wait_reason: "permission".into(),
            meta_only: false,
            worktree: None,
            agent_id: None,
            session_id: None,
        };
        let writes = transition(&snapshot_with_status("running"), &event, &Reported::NONE);
        let entries = entries(&writes);
        for key in [
            PANE_SESSION_ID,
            PANE_CWD,
            PANE_WORKTREE_NAME,
            PANE_WORKTREE_BRANCH,
        ] {
            assert!(
                !entries.iter().any(|(entry_key, _)| entry_key == &key),
                "{key} must not be touched when the event reports nothing"
            );
        }
        // The unguarded writes still happen.
        assert!(entries.contains(&(PANE_STATUS, Some("waiting".into()))));
    }

    // ─── 3.4 the status↔attention coupling ─────────────────────────────

    #[test]
    fn status_writes_clear_unsets_status_and_attention() {
        assert_eq!(
            entries(&status_writes("clear")),
            expected(&[(PANE_STATUS, None), (PANE_ATTENTION, None)])
        );
    }

    #[test]
    fn status_writes_running_and_idle_unset_attention() {
        for status in ["running", "idle"] {
            assert_eq!(
                entries(&status_writes(status)),
                expected(&[(PANE_STATUS, Some(status)), (PANE_ATTENTION, None)]),
                "status {status}"
            );
        }
    }

    #[test]
    fn status_writes_waiting_and_error_leave_attention_alone() {
        for status in ["waiting", "error", "background"] {
            assert_eq!(
                entries(&status_writes(status)),
                expected(&[(PANE_STATUS, Some(status))]),
                "status {status}"
            );
        }
    }

    // ─── the moved status resolvers ────────────────────────────────────

    #[test]
    fn is_action_required_wait_reason_allowlist() {
        assert!(is_action_required_wait_reason("permission"));
        assert!(is_action_required_wait_reason("permission_prompt"));
        assert!(is_action_required_wait_reason("permission_denied"));
        assert!(is_action_required_wait_reason("elicitation_dialog"));

        assert!(!is_action_required_wait_reason("auth_success"));
        assert!(!is_action_required_wait_reason("rate_limit"));
        assert!(!is_action_required_wait_reason("session_resumed"));
        assert!(!is_action_required_wait_reason("teammate_idle:alice"));
        assert!(!is_action_required_wait_reason(""));
    }

    #[test]
    fn pi_prompt_kinds_keep_waiting_under_live_bg_shell() {
        for reason in ["select", "confirm", "input", "editor", "custom", "question"] {
            assert!(
                is_action_required_wait_reason(reason),
                "{reason} should be action-required"
            );
            assert_eq!(
                resolve_notification_status(reason, true),
                "waiting",
                "{reason} should stay waiting under a live bg shell"
            );
            assert_eq!(resolve_notification_status(reason, false), "waiting");
        }
    }

    #[test]
    fn resolve_stop_status_prefers_background_when_bg_shell_live() {
        assert_eq!(resolve_stop_status(true), "background");
        assert_eq!(resolve_stop_status(false), "idle");
    }

    #[test]
    fn resolve_notification_status_permission_always_waiting() {
        // The action-required class survives regardless of bg shell state.
        assert_eq!(resolve_notification_status("permission", true), "waiting");
        assert_eq!(resolve_notification_status("permission", false), "waiting");
        assert_eq!(
            resolve_notification_status("permission_prompt", true),
            "waiting"
        );
    }

    #[test]
    fn resolve_notification_status_soft_reason_downgrades_to_background_when_bg_live() {
        assert_eq!(
            resolve_notification_status("auth_success", true),
            "background"
        );
        assert_eq!(
            resolve_notification_status("rate_limit", true),
            "background"
        );
        assert_eq!(resolve_notification_status("", true), "background");
    }

    #[test]
    fn resolve_notification_status_soft_reason_without_bg_stays_waiting() {
        assert_eq!(
            resolve_notification_status("auth_success", false),
            "waiting"
        );
        assert_eq!(resolve_notification_status("", false), "waiting");
    }

    // ─── resolve_cwd and is_system_message, moved next to the rules ────

    #[test]
    fn resolve_cwd_prefers_worktree_original_repo_dir() {
        assert_eq!(resolve_cwd("/tmp/wt/src", &worktree("feat", "feat")), CWD);
    }

    #[test]
    fn resolve_cwd_falls_back_to_raw_cwd() {
        assert_eq!(resolve_cwd("/tmp/project", &None), "/tmp/project");
    }

    #[test]
    fn resolve_cwd_worktree_empty_original_falls_back() {
        let wt = Some(WorktreeInfo {
            name: "feat".into(),
            path: "/tmp/wt".into(),
            branch: "feat".into(),
            original_repo_dir: "".into(),
        });
        assert_eq!(resolve_cwd("/tmp/wt/src", &wt), "/tmp/wt/src");
    }

    #[test]
    fn system_message_shapes_are_recognised() {
        assert!(is_system_message(
            "<task-notification><task-id>abc</task-id></task-notification>"
        ));
        assert!(is_system_message(
            "<system-reminder>some reminder</system-reminder>"
        ));
        assert!(is_system_message("<task-id>abc</task-id>"));
        assert!(!is_system_message("fix the bug"));
        assert!(!is_system_message(""));
        assert!(is_system_message(
            "hello <system-reminder>noise</system-reminder> world"
        ));
    }
}
