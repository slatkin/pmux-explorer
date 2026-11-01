use super::meta::AgentContext;
use crate::pane_state::resolve_cwd;
use crate::pane_store::PaneStore;
use crate::tmux;

pub(in crate::cli::hook) fn repo_label_from_ctx(ctx: &AgentContext<'_>) -> Option<String> {
    let cwd = resolve_cwd(ctx.cwd, ctx.worktree);
    repo_label_from_path(cwd)
}

pub(in crate::cli::hook) fn repo_label_from_pane(
    pane: &str,
    store: &dyn PaneStore,
) -> Option<String> {
    let cwd = store.get(pane, tmux::PANE_CWD);
    if !cwd.is_empty() {
        return repo_label_from_path(&cwd);
    }
    let worktree = store.get(pane, tmux::PANE_WORKTREE_NAME);
    if !worktree.is_empty() {
        return Some(worktree);
    }
    None
}

pub(in crate::cli::hook) fn branch_label_from_ctx(ctx: &AgentContext<'_>) -> Option<String> {
    if let Some(wt) = ctx.worktree
        && !wt.branch.is_empty()
    {
        return Some(wt.branch.clone());
    }
    let cwd = resolve_cwd(ctx.cwd, ctx.worktree);
    current_branch(cwd)
}

pub(in crate::cli::hook) fn branch_label_from_pane(
    pane: &str,
    store: &dyn PaneStore,
) -> Option<String> {
    let wt_branch = store.get(pane, tmux::PANE_WORKTREE_BRANCH);
    if !wt_branch.is_empty() {
        return Some(wt_branch);
    }
    let cwd = store.get(pane, tmux::PANE_CWD);
    if cwd.is_empty() {
        None
    } else {
        current_branch(&cwd)
    }
}

pub(in crate::cli::hook) fn current_branch(path: &str) -> Option<String> {
    crate::git::run_git(path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD")
}

pub(in crate::cli::hook) fn repo_label_from_path(path: &str) -> Option<String> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let label = trimmed.rsplit('/').next().unwrap_or(trimmed).trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::WorktreeInfo;
    use crate::pane_store::MemPaneStore;

    #[test]
    fn repo_label_from_path_strips_trailing_slash() {
        assert_eq!(
            repo_label_from_path("/home/user/repo/"),
            Some("repo".into())
        );
        assert_eq!(repo_label_from_path("/home/user/repo"), Some("repo".into()));
    }

    #[test]
    fn repo_label_from_path_rejects_empty_or_root() {
        assert!(repo_label_from_path("").is_none());
        assert!(repo_label_from_path("/").is_none());
    }

    #[test]
    fn repo_label_from_ctx_prefers_worktree_original_repo_dir() {
        let wt = Some(WorktreeInfo {
            name: "feat".into(),
            path: "/tmp/wt".into(),
            branch: "feat".into(),
            original_repo_dir: "/home/user/repo".into(),
        });
        let ctx = AgentContext {
            agent: "claude",
            cwd: "/tmp/wt/src",
            worktree: &wt,
        };
        assert_eq!(repo_label_from_ctx(&ctx), Some("repo".into()));
    }

    #[test]
    fn repo_label_from_pane_prefers_pane_cwd_then_worktree_name() {
        let store = MemPaneStore::new();
        let pane = "%PANE_REPO";
        store.insert(pane, tmux::PANE_CWD, "/home/user/app");
        store.insert(pane, tmux::PANE_WORKTREE_NAME, "wt-name");

        assert_eq!(repo_label_from_pane(pane, &store), Some("app".into()));

        store.insert(pane, tmux::PANE_CWD, "");
        assert_eq!(repo_label_from_pane(pane, &store), Some("wt-name".into()));
    }

    #[test]
    fn repo_label_from_pane_returns_none_when_all_empty() {
        let store = MemPaneStore::new();
        let pane = "%EMPTY_REPO";
        assert!(repo_label_from_pane(pane, &store).is_none());
    }

    #[test]
    fn branch_label_from_ctx_prefers_worktree_branch() {
        let wt = Some(WorktreeInfo {
            name: "feat".into(),
            path: "/tmp/wt".into(),
            branch: "feature/xyz".into(),
            original_repo_dir: "/home/user/repo".into(),
        });
        let ctx = AgentContext {
            agent: "claude",
            cwd: "/tmp/wt/src",
            worktree: &wt,
        };
        assert_eq!(branch_label_from_ctx(&ctx), Some("feature/xyz".into()));
    }

    #[test]
    fn branch_label_from_pane_prefers_worktree_branch_option() {
        let store = MemPaneStore::new();
        let pane = "%PANE_BRANCH";
        store.insert(pane, tmux::PANE_WORKTREE_BRANCH, "feat/abc");
        store.insert(pane, tmux::PANE_CWD, "/tmp/somewhere");
        assert_eq!(
            branch_label_from_pane(pane, &store),
            Some("feat/abc".into())
        );
    }
}
