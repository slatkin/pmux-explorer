use super::meta::clear_all_meta;
use crate::pane_state::status_writes;
use crate::pane_store::{PaneStore, PaneWrites};
use crate::tmux::{self, AgentType};

/// Legacy pane-option name. Nothing sets this any more — a child
/// SessionEnd can't be distinguished from the parent's, so the
/// deferred-teardown dance was too dangerous to keep. The constant is
/// retained so `clear_all_meta` and `on_session_start` can still sweep
/// a stale marker left behind by a pre-fix install.
pub(in crate::cli::hook) const PENDING_SESSION_END: &str = tmux::PANE_PENDING_SESSION_END;
/// Tmux pane option set when WorktreeRemove is deferred because
/// subagents are still active. Drained by `on_subagent_stop` once
/// `@pane_subagents` becomes empty.
pub(in crate::cli::hook) const PENDING_WORKTREE_REMOVE: &str = tmux::PANE_PENDING_WORKTREE_REMOVE;

pub(in crate::cli::hook) fn mark_pending(pane: &str, key: &'static str, store: &dyn PaneStore) {
    let mut writes = PaneWrites::new();
    writes.set(key, "1");
    let _ = store.apply(pane, &writes);
}

/// Run any deferred teardown recorded by a previous call to
/// `on_worktree_remove`. Called from `on_subagent_stop` after the
/// subagent list drains to empty so the parent pane is finally cleaned
/// up instead of being stranded with stale metadata.
///
/// `on_session_end` no longer participates — a child SessionEnd racing
/// ahead of SubagentStop would otherwise replay the teardown against
/// the live parent. The SessionEnd path now bails out early instead.
pub(in crate::cli::hook) fn drain_pending_teardowns(pane: &str, store: &dyn PaneStore) {
    if !store.get(pane, PENDING_WORKTREE_REMOVE).is_empty() {
        run_worktree_remove_teardown(pane, store);
        let mut writes = PaneWrites::new();
        writes.unset(PENDING_WORKTREE_REMOVE);
        let _ = store.apply(pane, &writes);
    }
}

/// Side-effect body of the SessionEnd teardown. Invoked by
/// `on_session_end` when no subagents are active; subagent-active
/// SessionEnds are short-circuited before they reach this point.
///
/// A pi pane's thinking feed dies with its state (design D7): the agent
/// and session id are read from the pane options before `clear_all_meta`
/// wipes them, and the feed environment arrives as an argument the way
/// the polling-side sweeps take it, so tests inject a scratch directory
/// instead of touching the process env. Only a pi pane removes a feed;
/// a rejected session id removes nothing, and a missing file is not an
/// error.
pub(in crate::cli::hook) fn run_session_end_teardown(
    pane: &str,
    store: &dyn PaneStore,
    env: &crate::thinking_feed::FeedEnv,
) {
    // The agent and session id are read before the wipe below removes them.
    let agent = AgentType::from_label(&store.get(pane, tmux::PANE_AGENT));
    if let Some(agent) = agent {
        let session_id = store.get(pane, tmux::PANE_SESSION_ID);
        crate::thinking_feed::remove_pi_feed(agent, &session_id, env);
    }
    clear_all_meta(pane, store);
    // The final status write also clears the attention flag, so the old
    // leading `set_attention("clear")` folded into this one write.
    let _ = store.apply(pane, &status_writes("clear"));
    let log_path = crate::activity::log_file_path(pane);
    // Best effort: a log that survives teardown keeps stale tool rows until
    // the next SessionStart clears it, and teardown must not fail on it.
    let _ = std::fs::remove_file(log_path);
}

/// Side-effect body of the WorktreeRemove teardown. Same pattern as
/// `run_session_end_teardown` — single source of truth for both the inline
/// and deferred paths. The teardown's option writes are one batch through
/// the store: drop the worktree pair and the hook-set cwd so the query
/// layer falls back to `pane_current_path` instead of associating the pane
/// with a stale worktree path.
pub(in crate::cli::hook) fn run_worktree_remove_teardown(pane: &str, store: &dyn PaneStore) {
    // The `None` arm of `sync_worktree_meta`, expressed as a batch: both
    // worktree fields go.
    let mut writes = PaneWrites::new();
    writes.unset(tmux::PANE_WORKTREE_NAME);
    writes.unset(tmux::PANE_WORKTREE_BRANCH);
    writes.unset(tmux::PANE_CWD);
    let _ = store.apply(pane, &writes);
}
