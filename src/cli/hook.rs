use crate::event::{AgentEvent, Reported, resolve_adapter};
use crate::pane_state::transition;
use crate::pane_store::PaneStore;
use crate::tmux;

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
        AgentEvent::ActivityLog {
            tool_name,
            tool_input,
            tool_response,
        } => activity::handle_activity_log(pane, store, tool_name, tool_input, tool_response),
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
