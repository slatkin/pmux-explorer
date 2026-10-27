use std::path::Path;

use super::config::{BRANCH_PREFIX_OPTION, WORKTREE_DIR_OPTION};
use super::flow::WorktreeError;
use crate::{git, tmux};

/// All side effects `spawn_with` / `remove_with` depend on, extracted
/// so failure-path tests can script tmux/git errors without touching a
/// real tmux server or repository. The real implementation lives in
/// `RealEnv` below.
pub(crate) trait SpawnEnv {
    fn branch_prefix(&self) -> Option<String>;
    fn worktree_dir(&self) -> Option<String>;
    fn branch_is_free(&self, repo: &str, branch: &str) -> bool;
    /// Whether a branch currently exists in `repo`. Used by the
    /// remove flow to skip `git branch -D` when a previous partial
    /// success already dropped the branch, so retries converge.
    fn branch_exists(&self, repo: &str, branch: &str) -> bool;
    fn worktree_path_is_free(&self, path: &Path) -> bool;
    /// Whether the worktree directory is currently on disk. Distinct
    /// from [`Self::worktree_path_is_free`] only because the fake
    /// implementations in tests need opposite default polarities for
    /// spawn (`is_free=true`, picks a fresh slug) and remove
    /// (`exists=true`, runs the cleanup).
    fn worktree_path_exists(&self, path: &str) -> bool;
    fn worktree_add(
        &self,
        repo: &str,
        worktree_path: &str,
        branch: &str,
    ) -> Result<(), WorktreeError>;
    fn worktree_remove(&self, repo: &str, worktree_path: &str) -> Result<(), WorktreeError>;
    fn branch_delete(&self, repo: &str, branch: &str) -> Result<(), WorktreeError>;
    fn new_window(
        &self,
        session: &str,
        cwd: &str,
        name: &str,
    ) -> Result<(String, String), WorktreeError>;
    fn kill_window(&self, window_id: &str) -> Result<(), WorktreeError>;
    fn set_window_option(&self, window: &str, key: &str, value: &str) -> Result<(), WorktreeError>;
    fn send_command(&self, target: &str, command: &str) -> Result<(), WorktreeError>;
    fn display_message(&self, pane_id: &str, template: &str) -> String;
}

pub(super) struct RealEnv;

impl SpawnEnv for RealEnv {
    fn branch_prefix(&self) -> Option<String> {
        tmux::get_all_global_options()
            .get(BRANCH_PREFIX_OPTION)
            .cloned()
    }
    fn worktree_dir(&self) -> Option<String> {
        tmux::get_all_global_options()
            .get(WORKTREE_DIR_OPTION)
            .cloned()
    }
    fn branch_is_free(&self, repo: &str, branch: &str) -> bool {
        !git::branch_exists(repo, branch)
    }
    fn branch_exists(&self, repo: &str, branch: &str) -> bool {
        git::branch_exists(repo, branch)
    }
    fn worktree_path_is_free(&self, path: &Path) -> bool {
        !path.exists()
    }
    fn worktree_path_exists(&self, path: &str) -> bool {
        !path.is_empty() && Path::new(path).exists()
    }
    fn worktree_add(&self, repo: &str, path: &str, branch: &str) -> Result<(), WorktreeError> {
        git::worktree_add(repo, path, branch).map_err(WorktreeError::Git)
    }
    fn worktree_remove(&self, repo: &str, path: &str) -> Result<(), WorktreeError> {
        git::worktree_remove(repo, path).map_err(WorktreeError::Git)
    }
    fn branch_delete(&self, repo: &str, branch: &str) -> Result<(), WorktreeError> {
        git::branch_delete(repo, branch).map_err(WorktreeError::Git)
    }
    fn new_window(
        &self,
        session: &str,
        cwd: &str,
        name: &str,
    ) -> Result<(String, String), WorktreeError> {
        tmux::new_window(session, cwd, name).map_err(WorktreeError::Tmux)
    }
    fn kill_window(&self, window_id: &str) -> Result<(), WorktreeError> {
        tmux::kill_window(window_id).map_err(WorktreeError::Tmux)
    }
    fn set_window_option(&self, window: &str, key: &str, value: &str) -> Result<(), WorktreeError> {
        tmux::set_window_option(window, key, value).map_err(WorktreeError::Tmux)
    }
    fn send_command(&self, target: &str, command: &str) -> Result<(), WorktreeError> {
        tmux::send_command(target, command).map_err(WorktreeError::Tmux)
    }
    fn display_message(&self, pane_id: &str, template: &str) -> String {
        tmux::display_message(pane_id, template)
    }
}
