use crate::pane_store::PaneStore;
use crate::tmux;

/// Returns whether the pane's cwd should be updated.
/// When subagents are active, events may come from a subagent running in a
/// worktree, so we should NOT overwrite the parent agent's cwd.
pub(in crate::cli::hook) fn should_update_cwd(current_subagents: &str) -> bool {
    current_subagents.is_empty()
}

/// Returns true if pane-scoped writes from this hook event are safe to
/// apply to the pane's metadata. False while subagents are active so a
/// child hook cannot clobber the parent pane's identity. The subagent
/// list is read through the store like every other option access.
pub(in crate::cli::hook) fn pane_writes_allowed(pane: &str, store: &dyn PaneStore) -> bool {
    should_update_cwd(&store.get(pane, tmux::PANE_SUBAGENTS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_store::MemPaneStore;

    #[test]
    fn should_update_cwd_when_no_subagents() {
        assert!(should_update_cwd(""));
    }

    #[test]
    fn should_not_update_cwd_when_subagent_active() {
        assert!(!should_update_cwd("Explore:sub-1"));
    }

    #[test]
    fn should_not_update_cwd_when_multiple_subagents_active() {
        assert!(!should_update_cwd("Explore:sub-1,Plan:sub-2"));
    }

    #[test]
    fn pane_writes_allowed_tracks_subagent_presence() {
        let store = MemPaneStore::new();
        let pane = "%ALLOWED";
        assert!(pane_writes_allowed(pane, &store));
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        assert!(!pane_writes_allowed(pane, &store));
    }
}
