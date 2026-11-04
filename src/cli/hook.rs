use crate::event::{AgentEvent, Reported, resolve_adapter};
use crate::pane_state::transition;
use crate::pane_store::PaneStore;
use crate::tmux::{self, CLAUDE_AGENT};

use super::{read_stdin_json, tmux_pane};

mod activity;
mod context;
mod handlers;
mod notifications;

use notifications::notification_settings;

// ─── hook subcommand ────────────────────────────────────────────────────────

pub(crate) fn cmd_hook(args: &[String]) -> i32 {
    let agent_name = args.first().map(|s| s.as_str()).unwrap_or("");
    let event_name = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if agent_name.is_empty() || event_name.is_empty() {
        return 0;
    }

    let Some(adapter) = resolve_adapter(agent_name) else {
        return 0;
    };

    // Claude Code processes that pi started in its own pane (for example
    // the claude-bridge provider) inherit pi's TMUX_PANE and would clobber
    // the pi pane state that pi's own producer already reports.
    if ignored_under_pi(agent_name, std::env::var("PI_CODING_AGENT").ok().as_deref()) {
        return 0;
    }

    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }

    let input = read_stdin_json();
    let Some(event) = adapter.parse(event_name, &input) else {
        return 0;
    };
    let reported = adapter.reported(event_name, &input);

    // The pane store is created once per hook invocation and passed down;
    // every option write and read the event causes goes through it.
    let store = tmux::pane_store::TmuxPaneStore;
    handle_event(&pane, agent_name, &store, event, reported)
}

// ─── pi's Claude Code children ─────────────────────────────────────────────

/// True for a `claude` event whose environment carries a present, non-empty
/// `PI_CODING_AGENT`: the event comes from a Claude Code process that pi
/// started in its own pane, and pi's own producer already reports that pane.
fn ignored_under_pi(agent_name: &str, pi_coding_agent: Option<&str>) -> bool {
    agent_name == CLAUDE_AGENT && pi_coding_agent.is_some_and(|v| !v.is_empty())
}

// ─── event handler ──────────────────────────────────────────────────────────

/// Apply one event to one pane. `reported` is the field set the adapter
/// declared for the event; the handlers that run the transition consume it
/// to decide which identity fields the event owns.
fn handle_event(
    pane: &str,
    agent_name: &str,
    store: &dyn PaneStore,
    event: AgentEvent,
    reported: Reported,
) -> i32 {
    match &event {
        AgentEvent::SessionStart { .. } => {
            handlers::on_session_start(pane, &event, &reported, store)
        }
        AgentEvent::SessionEnd { end_reason } => {
            let notifications = notification_settings();
            handlers::on_session_end(pane, agent_name, end_reason, &notifications, store)
        }
        AgentEvent::UserPromptSubmit { .. } => {
            handlers::on_user_prompt_submit(pane, &event, &reported, store)
        }
        AgentEvent::Notification {
            agent,
            cwd,
            worktree,
            ..
        } => {
            let notifications = notification_settings();
            let ctx = context::make_ctx(agent, cwd, worktree);
            handlers::on_notification(pane, &event, &reported, &ctx, &notifications, store)
        }
        AgentEvent::Stop {
            agent,
            cwd,
            worktree,
            ..
        } => {
            let notifications = notification_settings();
            let ctx = context::make_ctx(agent, cwd, worktree);
            handlers::on_stop(pane, &event, &reported, &ctx, &notifications, store)
        }
        AgentEvent::StopFailure {
            agent,
            cwd,
            worktree,
            ..
        } => {
            let notifications = notification_settings();
            let ctx = context::make_ctx(agent, cwd, worktree);
            handlers::on_stop_failure(pane, &event, &reported, &ctx, &notifications, store)
        }
        AgentEvent::SubagentStart {
            agent_type,
            agent_id,
        } => handlers::on_subagent_start(pane, agent_type, agent_id.as_deref(), store),
        AgentEvent::SubagentStop { agent_id, .. } => {
            handlers::on_subagent_stop(pane, agent_id.as_deref(), store)
        }
        AgentEvent::ActivityLog { .. } => {
            // The transition owns the activity-log option writes: the
            // status move, the background-Bash command and the guarded
            // plan-mode permission write. The activity log file entry is
            // the handler's own effect.
            activity::handle_activity_log(pane, &event, &reported, store)
        }
        AgentEvent::PermissionDenied {
            agent,
            cwd,
            worktree,
            ..
        } => {
            let notifications = notification_settings();
            let ctx = context::make_ctx(agent, cwd, worktree);
            handlers::on_permission_denied(pane, &event, &reported, &ctx, &notifications, store)
        }
        AgentEvent::CwdChanged { .. } => {
            // The transition owns the location writes: the session id, the
            // cwd and the worktree pair, guarded while subagents are
            // active and gated on the event's reported fields.
            let snapshot = store.snapshot(pane);
            let writes = transition(&snapshot, &event, &reported);
            let _ = store.apply(pane, &writes);
            0
        }
        AgentEvent::TaskCreated { .. } => 0,
        AgentEvent::TaskCompleted {
            task_id,
            task_subject,
        } => {
            // The attention flag comes from the transition's TaskCompleted
            // arm, like every other event's option writes.
            let snapshot = store.snapshot(pane);
            let writes = transition(&snapshot, &event, &reported);
            let _ = store.apply(pane, &writes);
            let notifications = notification_settings();
            handlers::on_task_completed(
                pane,
                agent_name,
                task_id,
                task_subject,
                &notifications,
                store,
            )
        }
        AgentEvent::TeammateIdle {
            teammate_name,
            idle_reason,
            ..
        } => handlers::on_teammate_idle(pane, teammate_name, idle_reason, store),
        AgentEvent::WorktreeCreate => 0,
        AgentEvent::WorktreeRemove { .. } => handlers::on_worktree_remove(pane, store),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_under_pi_claude_with_true() {
        assert!(ignored_under_pi("claude", Some("true")));
    }

    #[test]
    fn ignored_under_pi_claude_with_any_other_non_empty_value() {
        assert!(ignored_under_pi("claude", Some("1")));
        assert!(ignored_under_pi("claude", Some("yes")));
    }

    #[test]
    fn ignored_under_pi_claude_with_empty_value_is_processed() {
        assert!(!ignored_under_pi("claude", Some("")));
    }

    #[test]
    fn ignored_under_pi_claude_with_absent_value_is_processed() {
        assert!(!ignored_under_pi("claude", None));
    }

    #[test]
    fn ignored_under_pi_other_agents_are_processed() {
        assert!(!ignored_under_pi("pi", Some("true")));
        assert!(!ignored_under_pi("codex", Some("true")));
        assert!(!ignored_under_pi("opencode", Some("true")));
    }
}
