use crate::pane_store::{PaneStore, PaneWrites};
use crate::tmux;

use super::super::context::{append_subagent, drain_pending_teardowns, remove_subagent};

pub(in crate::cli::hook) fn on_subagent_start(
    pane: &str,
    agent_type: &str,
    agent_id: Option<&str>,
    store: &dyn PaneStore,
) -> i32 {
    // Claude Code always sends agent_id per the hooks spec; drop the
    // event silently if it's missing so the tree never gains an
    // untrackable entry.
    let Some(id) = agent_id.filter(|s| !s.is_empty()) else {
        return 0;
    };
    let current = store.get(pane, tmux::PANE_SUBAGENTS);
    let new_val = append_subagent(&current, agent_type, id);
    let mut writes = PaneWrites::new();
    writes.set(tmux::PANE_SUBAGENTS, new_val);
    let _ = store.apply(pane, &writes);
    0
}

pub(in crate::cli::hook) fn on_subagent_stop(
    pane: &str,
    agent_id: Option<&str>,
    store: &dyn PaneStore,
) -> i32 {
    let Some(id) = agent_id.filter(|s| !s.is_empty()) else {
        return 0;
    };
    let current = store.get(pane, tmux::PANE_SUBAGENTS);
    let drained_to_empty = match remove_subagent(&current, id) {
        None => false,
        Some(new_val) if new_val.is_empty() => {
            let mut writes = PaneWrites::new();
            writes.unset(tmux::PANE_SUBAGENTS);
            let _ = store.apply(pane, &writes);
            true
        }
        Some(new_val) => {
            let mut writes = PaneWrites::new();
            writes.set(tmux::PANE_SUBAGENTS, new_val);
            let _ = store.apply(pane, &writes);
            false
        }
    };
    // Once the last subagent stops, replay any teardown that was deferred
    // because subagents were active when SessionEnd / WorktreeRemove fired.
    if drained_to_empty {
        drain_pending_teardowns(pane, store);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::super::session::on_session_end;
    use super::super::worktree::on_worktree_remove;
    use super::*;
    use crate::cli::hook::context::{PENDING_SESSION_END, PENDING_WORKTREE_REMOVE};
    use crate::desktop_notification;
    use crate::pane_store::MemPaneStore;
    use std::fs;

    fn default_notifications() -> desktop_notification::DesktopNotificationSettings {
        desktop_notification::DesktopNotificationSettings {
            enabled: false,
            events: Default::default(),
        }
    }

    #[test]
    fn on_subagent_start_appends_to_list() {
        let store = MemPaneStore::new();
        let pane = "%SUB_START";
        on_subagent_start(pane, "Explore", Some("sub-1"), &store);
        assert_eq!(store.get(pane, tmux::PANE_SUBAGENTS), "Explore:sub-1");
        on_subagent_start(pane, "Plan", Some("sub-2"), &store);
        assert_eq!(
            store.get(pane, tmux::PANE_SUBAGENTS),
            "Explore:sub-1,Plan:sub-2"
        );
    }

    #[test]
    fn on_subagent_start_drops_event_without_id() {
        let store = MemPaneStore::new();
        let pane = "%SUB_NO_ID";
        on_subagent_start(pane, "Explore", None, &store);
        assert!(!store.contains(pane, tmux::PANE_SUBAGENTS));
        on_subagent_start(pane, "Explore", Some(""), &store);
        assert!(!store.contains(pane, tmux::PANE_SUBAGENTS));
    }

    // ─── deferred teardown regression tests ─────────────────────────
    //
    // These pin the invariant that WorktreeRemove fired while subagents
    // are active must not be lost forever — it is recorded as a pending
    // marker and replayed by `on_subagent_stop` once the subagent list
    // drains to empty.
    //
    // SessionEnd does NOT participate in the deferred-drain dance: we
    // can't tell a parent SessionEnd from a child's, and letting the
    // drain replay one on the wrong side risks wiping a live parent.

    #[test]
    fn session_end_while_subagents_active_is_a_no_op() {
        // Regression: previously `on_session_end` set PENDING_SESSION_END
        // whenever `@pane_subagents` was non-empty, and the next
        // `on_subagent_stop` would turn that marker into
        // `run_session_end_teardown`. Because subagents share the
        // parent's `$TMUX_PANE`, there is no way to guarantee the
        // SessionEnd came from the parent — so the safer default is to
        // skip the event entirely and leave the parent's state alone.
        let store = MemPaneStore::new();
        let pane = "%CHILD_SESSIONEND";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        store.insert(pane, tmux::PANE_AGENT, "claude");
        store.insert(pane, tmux::PANE_CWD, "/repo/parent");
        store.insert(pane, tmux::PANE_STATUS, "running");
        let log_path = crate::activity::log_file_path(pane);
        let _ = fs::create_dir_all(log_path.parent().unwrap());
        fs::write(&log_path, "1234567890|Read|main.rs\n").unwrap();

        on_session_end(pane, "claude", "", &default_notifications(), &store);
        assert!(
            !store.contains(pane, PENDING_SESSION_END),
            "child SessionEnd must not record a pending teardown"
        );
        // Every parent field must survive.
        assert!(store.contains(pane, tmux::PANE_AGENT));
        assert!(store.contains(pane, tmux::PANE_CWD));
        assert!(store.contains(pane, tmux::PANE_STATUS));
        assert!(log_path.exists());

        // A SubagentStop that drains the list to empty must not trigger a
        // teardown either — the pending marker was never recorded.
        on_subagent_stop(pane, Some("sub-1"), &store);
        assert!(
            store.contains(pane, tmux::PANE_AGENT),
            "SubagentStop draining an empty list must not tear down a live parent"
        );
        assert!(log_path.exists());
        assert!(
            !store.contains(pane, PENDING_WORKTREE_REMOVE),
            "the drain must not have run the worktree teardown either"
        );

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn pending_worktree_remove_drains_when_last_subagent_stops() {
        let store = MemPaneStore::new();
        let pane = "%PARENT_WT_DEFER";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        store.insert(pane, tmux::PANE_WORKTREE_NAME, "feat");
        store.insert(pane, tmux::PANE_WORKTREE_BRANCH, "feat");
        store.insert(pane, tmux::PANE_CWD, "/wt/feat");

        on_worktree_remove(pane, &store);
        assert!(
            store.contains(pane, PENDING_WORKTREE_REMOVE),
            "WorktreeRemove must be deferred via the pending marker"
        );
        assert!(store.contains(pane, tmux::PANE_WORKTREE_NAME));

        on_subagent_stop(pane, Some("sub-1"), &store);

        assert!(!store.contains(pane, tmux::PANE_WORKTREE_NAME));
        assert!(!store.contains(pane, tmux::PANE_WORKTREE_BRANCH));
        assert!(!store.contains(pane, tmux::PANE_CWD));
        assert!(
            !store.contains(pane, PENDING_WORKTREE_REMOVE),
            "pending marker must be cleared once teardown runs"
        );
    }

    #[test]
    fn pending_worktree_remove_waits_for_last_subagent() {
        // Equivalent of the old `pending_teardown_does_not_fire_until_subagents_empty`
        // but anchored on WorktreeRemove, which still uses the deferred
        // drain (SessionEnd dropped it intentionally — see the comment
        // above `session_end_while_subagents_active_is_a_no_op`).
        let store = MemPaneStore::new();
        let pane = "%PARENT_WT_PARTIAL";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1,Plan:sub-2");
        store.insert(pane, tmux::PANE_WORKTREE_NAME, "feat");
        store.insert(pane, tmux::PANE_WORKTREE_BRANCH, "feat");
        store.insert(pane, tmux::PANE_CWD, "/wt/feat");

        on_worktree_remove(pane, &store);
        assert!(store.contains(pane, PENDING_WORKTREE_REMOVE));

        // First child stops — list still has sub-2, teardown must NOT fire.
        on_subagent_stop(pane, Some("sub-1"), &store);
        assert!(
            store.contains(pane, tmux::PANE_WORKTREE_NAME),
            "teardown must wait for the LAST subagent"
        );
        assert!(store.contains(pane, PENDING_WORKTREE_REMOVE));

        // Last child stops — now teardown fires.
        on_subagent_stop(pane, Some("sub-2"), &store);
        assert!(!store.contains(pane, tmux::PANE_WORKTREE_NAME));
        assert!(!store.contains(pane, PENDING_WORKTREE_REMOVE));
    }
}
