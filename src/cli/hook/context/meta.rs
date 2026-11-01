use super::location::should_update_cwd;
use crate::event::WorktreeInfo;
use crate::pane_store::{PaneStore, PaneWrites};
use crate::tmux;

/// Label fields shared by the events that fire a desktop notification
/// (Notification / Stop / StopFailure): the agent name and the repo/branch
/// label sources. The identity fields (permission mode, session id) are
/// transition-owned now and never leave `handle_event`, so the context
/// carries only what the notification titles need. Passing this as a
/// single reference keeps each variant handler's signature short.
pub(in crate::cli::hook) struct AgentContext<'a> {
    pub(in crate::cli::hook) agent: &'a str,
    pub(in crate::cli::hook) cwd: &'a str,
    pub(in crate::cli::hook) worktree: &'a Option<WorktreeInfo>,
}

pub(in crate::cli::hook) fn make_ctx<'a>(
    agent: &'a str,
    cwd: &'a str,
    worktree: &'a Option<WorktreeInfo>,
) -> AgentContext<'a> {
    AgentContext {
        agent,
        cwd,
        worktree,
    }
}

/// Clear every lifecycle pane option the teardown owns, in one batch.
/// The keys come from the single sweep list `tmux::AGENT_SESSION_PANE_OPTIONS`
/// except the two handler-owned keys — `@pane_status` and `@pane_attention`
/// are set and cleared per event transition, and the closing
/// `status_writes("clear")` in the teardown covers both. The run-state pair
/// (`@pane_started_at` / `@pane_wait_reason`) is part of this wipe: the
/// transition owns their per-event writes, and a session end clears them
/// with everything else.
pub(in crate::cli::hook) fn clear_all_meta(pane: &str, store: &dyn PaneStore) {
    let mut writes = PaneWrites::new();
    for key in tmux::AGENT_SESSION_PANE_OPTIONS {
        if matches!(*key, tmux::PANE_STATUS | tmux::PANE_ATTENTION) {
            continue;
        }
        writes.unset(key);
    }
    let _ = store.apply(pane, &writes);
}

/// Write a task-reset marker to the activity log so `parse_task_progress`
/// treats the upcoming run as a fresh batch — otherwise in-progress or
/// abandoned tasks from a previous run would accumulate into the next one.
///
/// Skipped while subagents are still active so a parent Stop event doesn't
/// wipe task state children are still driving. The subagent read goes
/// through the store so a caller that just applied a Stop batch — which
/// clears the stale list — sees the post-batch state and the marker lands
/// the way it did when the handler unset the list inline.
pub(in crate::cli::hook) fn mark_task_reset(pane: &str, store: &dyn PaneStore) {
    if !should_update_cwd(&store.get(pane, tmux::PANE_SUBAGENTS)) {
        return;
    }
    if let Err(error) = crate::cli::hook::activity::write_activity_entry(
        pane,
        crate::activity::TASK_RESET_MARKER,
        "",
    ) {
        crate::cli::hook::activity::report_write_error(&error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_store::{MemPaneStore, PaneWrites};
    use std::fs;

    #[test]
    fn make_ctx_wires_all_fields() {
        let agent = "claude".to_string();
        let cwd = "/tmp".to_string();
        let worktree: Option<WorktreeInfo> = None;
        let ctx = make_ctx(&agent, &cwd, &worktree);
        assert_eq!(ctx.agent, "claude");
        assert_eq!(ctx.cwd, "/tmp");
        assert!(ctx.worktree.is_none());
    }

    #[test]
    fn make_ctx_preserves_worktree() {
        let agent = "codex".to_string();
        let cwd = "/src".to_string();
        let worktree = Some(WorktreeInfo {
            name: "feat".into(),
            path: "/tmp/wt".into(),
            branch: "feature/x".into(),
            original_repo_dir: "/home/user/repo".into(),
        });
        let ctx = make_ctx(&agent, &cwd, &worktree);
        assert_eq!(ctx.agent, "codex");
        assert_eq!(ctx.cwd, "/src");
        assert_eq!(ctx.worktree.as_ref().map(|w| w.name.as_str()), Some("feat"));
    }

    #[test]
    fn mark_task_reset_writes_marker_when_no_subagents() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_MARK_RESET";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        mark_task_reset(pane_id, &store);

        let content = fs::read_to_string(&path).unwrap();
        let marker = format!("|{}|", crate::activity::TASK_RESET_MARKER);
        assert!(content.contains(&marker), "marker not written: {content:?}");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn mark_task_reset_skips_while_subagents_active() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_MARK_RESET_SUBAGENT";
        store.insert(pane_id, tmux::PANE_SUBAGENTS, "Explore:abc");
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        mark_task_reset(pane_id, &store);

        // No marker should be written because subagents are still active.
        assert!(!path.exists(), "log file created while subagents active");
    }

    #[test]
    fn mark_task_reset_sees_the_post_batch_subagent_state() {
        // A Stop batch clears the stale subagent list first; the reset
        // marker must land because the store read reflects the batch.
        let store = MemPaneStore::new();
        let pane_id = "%CLI_MARK_RESET_AFTER_BATCH";
        let mut writes = PaneWrites::new();
        writes.unset(tmux::PANE_SUBAGENTS);
        store.apply(pane_id, &writes).unwrap();
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        mark_task_reset(pane_id, &store);

        assert!(
            path.exists(),
            "marker must land after the batch cleared the list"
        );
        fs::remove_file(&path).ok();
    }

    // ─── is_system_message lives with the transition rules ────────

    // ─── set_agent_meta ────────────────────────────────────────────

    #[test]
    fn clear_all_meta_drops_every_pane_option_we_own() {
        let store = MemPaneStore::new();
        let pane = "%CLEAR_ALL";
        for key in tmux::AGENT_SESSION_PANE_OPTIONS {
            store.insert(pane, key, "x");
        }

        clear_all_meta(pane, &store);

        // Every lifecycle key is cleared except `@pane_status` and
        // `@pane_attention`, which the closing `status_writes("clear")`
        // owns.
        for key in tmux::AGENT_SESSION_PANE_OPTIONS {
            let preserved = matches!(*key, tmux::PANE_STATUS | tmux::PANE_ATTENTION);
            assert_eq!(
                store.contains(pane, key),
                preserved,
                "expected {key} {}",
                if preserved { "preserved" } else { "cleared" }
            );
        }
    }
}
