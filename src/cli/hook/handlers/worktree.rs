use crate::pane_store::PaneStore;
use crate::tmux;

use super::super::context::{
    PENDING_WORKTREE_REMOVE, mark_pending, run_worktree_remove_teardown, should_update_cwd,
};

pub(in crate::cli::hook) fn on_worktree_remove(pane: &str, store: &dyn PaneStore) -> i32 {
    // If subagents are active, the removed worktree may belong to one of
    // them — we can't distinguish parent from child at this point, so the
    // safe default is to leave the parent's pane-scoped metadata intact.
    // Record the intent and let `on_subagent_stop` execute it once
    // children are gone.
    if !should_update_cwd(&store.get(pane, tmux::PANE_SUBAGENTS)) {
        mark_pending(pane, PENDING_WORKTREE_REMOVE, store);
        return 0;
    }
    run_worktree_remove_teardown(pane, store);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_store::MemPaneStore;

    #[test]
    fn on_worktree_remove_preserves_parent_state_when_subagents_active() {
        let store = MemPaneStore::new();
        let pane = "%PARENT_WT";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        store.insert(pane, tmux::PANE_WORKTREE_NAME, "parent-feat");
        store.insert(pane, tmux::PANE_WORKTREE_BRANCH, "feat/parent");
        store.insert(pane, tmux::PANE_CWD, "/repo/parent");

        on_worktree_remove(pane, &store);

        assert_eq!(store.get(pane, tmux::PANE_WORKTREE_NAME), "parent-feat");
        assert_eq!(store.get(pane, tmux::PANE_WORKTREE_BRANCH), "feat/parent");
        assert_eq!(store.get(pane, tmux::PANE_CWD), "/repo/parent");
    }

    #[test]
    fn on_worktree_remove_clears_state_when_no_subagents() {
        let store = MemPaneStore::new();
        let pane = "%LONE_WT";
        store.insert(pane, tmux::PANE_WORKTREE_NAME, "old");
        store.insert(pane, tmux::PANE_WORKTREE_BRANCH, "old");
        store.insert(pane, tmux::PANE_CWD, "/wt/old");

        on_worktree_remove(pane, &store);

        assert!(!store.contains(pane, tmux::PANE_WORKTREE_NAME));
        assert!(!store.contains(pane, tmux::PANE_WORKTREE_BRANCH));
        assert!(!store.contains(pane, tmux::PANE_CWD));
    }

    #[test]
    fn on_worktree_remove_defers_via_pending_marker_under_subagents() {
        let store = MemPaneStore::new();
        let pane = "%PENDING_WT";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");

        on_worktree_remove(pane, &store);

        assert!(
            store.contains(pane, PENDING_WORKTREE_REMOVE),
            "pending marker must be set when subagents are active"
        );
    }
}
