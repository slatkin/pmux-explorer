use indexmap::IndexMap;

use crate::git::run_git;
use crate::tmux::{PaneInfo, SubagentParent};

/// Per-pane git metadata resolved from the pane's working directory.
#[derive(Debug, Clone, Default)]
pub struct PaneGitInfo {
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub is_worktree: bool,
    pub worktree_name: Option<String>,
}

/// A group of panes working in the same repository (or directory).
#[derive(Debug, Clone)]
pub struct RepoGroup {
    /// Display name: repo directory basename, or raw path for non-git
    pub name: String,
    /// Whether any pane in the group belongs to the focused (active) window
    pub has_focus: bool,
    /// Panes in this group, with their git info
    pub panes: Vec<(PaneInfo, PaneGitInfo)>,
}

/// Resolve git info for a single pane path.
pub fn resolve_pane_git_info(path: &str) -> PaneGitInfo {
    if path.is_empty() {
        return PaneGitInfo::default();
    }

    // Single git call for all three values (one line per arg)
    let combined = run_git(
        path,
        &[
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
            "--git-common-dir",
            "--git-dir",
        ],
    );
    let (branch, git_common_dir, git_dir) = match combined {
        Some(output) => {
            let mut lines = output.lines();
            let b = lines.next().map(|s| s.to_string());
            let c = lines.next().map(|s| s.to_string());
            let d = lines.next().map(|s| s.to_string());
            (b, c, d)
        }
        None => (None, None, None),
    };

    let is_worktree = match (&git_common_dir, &git_dir) {
        (Some(common), Some(dir)) => {
            let common_path = resolve_git_path(path, common);
            let dir_path = resolve_git_path(path, dir);
            common_path != dir_path
        }
        _ => false,
    };

    // --git-common-dir returns the .git dir of the main worktree;
    // its parent is the repo root, so worktrees share the same group key.
    let repo_root = git_common_dir
        .as_ref()
        .and_then(|common| {
            let abs = resolve_git_path(path, common);
            abs.parent().map(|p| p.to_string_lossy().to_string())
        })
        .or_else(|| run_git(path, &["rev-parse", "--show-toplevel"]));

    PaneGitInfo {
        repo_root,
        branch,
        is_worktree,
        worktree_name: None,
    }
}

/// Group all panes across all sessions by repo root.
/// Returns groups sorted alphabetically by display name (case-insensitive),
/// so the order is stable regardless of which pane is encountered first.
pub fn group_panes_by_repo(sessions: &[crate::tmux::SessionInfo]) -> Vec<RepoGroup> {
    let mut groups: IndexMap<String, RepoGroup> = IndexMap::new();
    let mut git_cache: std::collections::HashMap<String, PaneGitInfo> =
        std::collections::HashMap::new();

    for session in sessions {
        for window in &session.windows {
            for pane in &window.panes {
                add_pane_to_groups(&mut groups, &mut git_cache, window, pane);
            }
        }
    }

    let mut result: Vec<RepoGroup> = groups.into_values().collect();
    result.sort_by_key(|a| a.name.to_lowercase());
    for group in &mut result {
        order_group_with_nests(&mut group.panes);
    }
    result
}

/// Map each subagent pane of one group to the pane id of its **ultimate**
/// parent: the top of the parent chain that is reachable inside the group.
///
/// A chain follows [`SubagentParent::Pane`] while the named pane is in the
/// same group and is itself a subagent; it ends at the last pane before a
/// parent that is orphaned, outside the group, or not given at all. That
/// ends pane is the root. A chain that revisits a pane is a cycle: the
/// visited set stops it there and the root is the cycle pane first in
/// tmux order, which includes a pane that names itself as its parent.
/// Panes without a root (non-subagents, orphaned subagents) are absent.
pub fn nest_roots(panes: &[(PaneInfo, PaneGitInfo)]) -> std::collections::HashMap<String, String> {
    let mut order: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut parents: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for (index, (pane, _)) in panes.iter().enumerate() {
        order.insert(pane.pane_id.as_str(), index);
        if let Some(SubagentParent::Pane(parent_id)) =
            pane.subagent.as_ref().map(|info| &info.parent)
        {
            parents.insert(pane.pane_id.as_str(), parent_id.as_str());
        }
    }

    let mut roots = std::collections::HashMap::new();
    for (pane, _) in panes {
        let Some(root) = ultimate_root(pane.pane_id.as_str(), &parents, &order) else {
            continue;
        };
        if root != pane.pane_id {
            roots.insert(pane.pane_id.clone(), root.to_string());
        }
    }
    roots
}

/// Walk one pane's parent chain inside a group and return its root, or
/// `None` when the pane has no parent to follow at all.
fn ultimate_root<'a>(
    start: &'a str,
    parents: &std::collections::HashMap<&'a str, &'a str>,
    order: &std::collections::HashMap<&'a str, usize>,
) -> Option<&'a str> {
    let mut chain: Vec<&'a str> = vec![start];
    let mut seen: std::collections::HashMap<&'a str, usize> = std::collections::HashMap::new();
    seen.insert(start, 0);
    let mut current = start;
    loop {
        let next = parents
            .get(current)
            .copied()
            .filter(|parent_id| order.contains_key(parent_id));
        let Some(next) = next else {
            // No in-group parent left: an ancestor heads the reachable
            // chain; `start` itself is top-level.
            return (current != start).then_some(current);
        };
        let Some(&cycle_start) = seen.get(next) else {
            seen.insert(next, chain.len());
            chain.push(next);
            current = next;
            continue;
        };
        // A cycle: its root is the cycle pane tmux lists first.
        return chain[cycle_start..]
            .iter()
            .min_by_key(|pane_id| order.get(**pane_id).copied().unwrap_or(usize::MAX))
            .copied();
    }
}

/// Whether `pane_id` is drawn indented, that is, whether it has a root and
/// that root survived the filters.
pub fn nested_under(
    roots: &std::collections::HashMap<String, String>,
    visible_ids: &std::collections::HashSet<&str>,
    pane_id: &str,
) -> bool {
    roots
        .get(pane_id)
        .is_some_and(|root| visible_ids.contains(root.as_str()))
}

/// Reorder one group's panes so each root is directly followed by its
/// children, in their original (tmux) order. A pane that is not a child
/// keeps its relative position, and every pane appears exactly once.
fn order_group_with_nests(panes: &mut Vec<(PaneInfo, PaneGitInfo)>) {
    let roots = nest_roots(panes);
    if roots.is_empty() {
        return;
    }
    let mut children: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();
    for (index, (pane, _)) in panes.iter().enumerate() {
        if let Some(root) = roots.get(&pane.pane_id) {
            children.entry(root.as_str()).or_default().push(index);
        }
    }

    let mut order: Vec<usize> = Vec::with_capacity(panes.len());
    for (index, (pane, _)) in panes.iter().enumerate() {
        if roots.contains_key(&pane.pane_id) {
            continue;
        }
        order.push(index);
        if let Some(child_indices) = children.get(pane.pane_id.as_str()) {
            order.extend(child_indices.iter().copied());
        }
    }

    let mut slots: Vec<Option<(PaneInfo, PaneGitInfo)>> = panes.drain(..).map(Some).collect();
    let mut reordered = Vec::with_capacity(order.len());
    for index in order {
        if let Some(item) = slots[index].take() {
            reordered.push(item);
        }
    }
    *panes = reordered;
}

/// Fold one pane into its repo group, resolving git info through the cache.
fn add_pane_to_groups(
    groups: &mut IndexMap<String, RepoGroup>,
    git_cache: &mut std::collections::HashMap<String, PaneGitInfo>,
    window: &crate::tmux::WindowInfo,
    pane: &PaneInfo,
) {
    let mut git_info = cached_git_info(git_cache, &pane.path);
    apply_worktree_overrides(&mut git_info, &pane.worktree);

    let group_key = pane_group_key(&git_info, &pane.path);
    let display_name = repo_display_name(&group_key);
    let has_focus = window.window_active && pane.pane_active;

    let group = groups.entry(group_key).or_insert_with(|| RepoGroup {
        name: display_name,
        has_focus: false,
        panes: Vec::new(),
    });

    if has_focus {
        group.has_focus = true;
    }

    group.panes.push((pane.clone(), git_info));
}

/// Look up cached git info for a pane path, resolving (and caching) on miss.
fn cached_git_info(
    git_cache: &mut std::collections::HashMap<String, PaneGitInfo>,
    path: &str,
) -> PaneGitInfo {
    // Cache the base git info per path. `get` first avoids a key
    // clone on cache hits; misses fall through to `insert` which
    // owns the key plus the (expensive) git-command lookup.
    match git_cache.get(path) {
        Some(cached) => cached.clone(),
        None => {
            let resolved = resolve_pane_git_info(path);
            git_cache.insert(path.to_string(), resolved.clone());
            resolved
        }
    }
}

/// Override base git info with hook-provided worktree metadata when present.
/// (Claude Code provides this; Codex does not, so the git-command base
/// remains as fallback.)
fn apply_worktree_overrides(git_info: &mut PaneGitInfo, worktree: &crate::tmux::WorktreeMetadata) {
    if !worktree.name.is_empty() {
        git_info.worktree_name = Some(worktree.name.clone());
        git_info.is_worktree = true;
    }
    if !worktree.branch.is_empty() {
        git_info.branch = Some(worktree.branch.clone());
        git_info.is_worktree = true;
    }
}

/// Group key: repo root when known, otherwise the pane's raw path.
fn pane_group_key(git_info: &PaneGitInfo, path: &str) -> String {
    git_info
        .repo_root
        .clone()
        .unwrap_or_else(|| path.to_string())
}

/// Display name: the repo directory basename, or the raw key for non-git paths.
fn repo_display_name(group_key: &str) -> String {
    group_key
        .rsplit('/')
        .next()
        .unwrap_or(group_key)
        .to_string()
}

/// Resolve a possibly-relative git path to an absolute canonical path.
fn resolve_git_path(base: &str, git_path: &str) -> std::path::PathBuf {
    let p = if std::path::Path::new(git_path).is_absolute() {
        std::path::PathBuf::from(git_path)
    } else {
        std::path::PathBuf::from(base).join(git_path)
    };
    p.canonicalize().unwrap_or(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_git_info_returns_none_for_empty_path() {
        let info = resolve_pane_git_info("");
        assert!(info.branch.is_none());
        assert!(!info.is_worktree);
        assert!(info.repo_root.is_none());
    }

    #[test]
    fn resolve_git_info_for_real_repo() {
        // This test runs in the actual repo, so git commands work
        let info = resolve_pane_git_info(env!("CARGO_MANIFEST_DIR"));
        assert!(info.repo_root.is_some(), "should detect git repo");
        assert!(info.branch.is_some(), "should detect branch");
        let root = info.repo_root.unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        assert_eq!(root, manifest_dir, "repo root should be manifest dir");
    }

    #[test]
    fn worktree_and_main_share_same_repo_root() {
        // Both main and worktree should resolve to the same repo_root
        // We can only test the main worktree here, but verify the logic is consistent
        let info = resolve_pane_git_info(env!("CARGO_MANIFEST_DIR"));
        assert!(
            !info.is_worktree,
            "main checkout should not be detected as worktree"
        );
        assert!(info.repo_root.is_some());
    }

    // ─── resolve_git_path tests ─────────────────────────────────────

    #[test]
    fn resolve_git_path_absolute() {
        let result = resolve_git_path("/base", "/absolute/path");
        assert_eq!(result, std::path::PathBuf::from("/absolute/path"));
    }

    #[test]
    fn resolve_git_path_relative() {
        let result = resolve_git_path("/base/dir", "relative");
        assert_eq!(result, std::path::PathBuf::from("/base/dir/relative"));
    }

    // ─── group_panes_by_repo tests ──────────────────────────────────

    fn test_pane(id: &str, path: &str) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            pane_active: false,
            status: crate::tmux::PaneStatus::Running,
            attention: false,
            agent: crate::tmux::AgentType::Claude,
            path: path.into(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: crate::tmux::PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: crate::tmux::WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        }
    }

    fn test_window(panes: Vec<PaneInfo>, active: bool) -> crate::tmux::WindowInfo {
        crate::tmux::WindowInfo {
            window_id: "@0".into(),
            window_name: "test".into(),
            window_active: active,
            auto_rename: false,
            panes,
        }
    }

    fn test_session(windows: Vec<crate::tmux::WindowInfo>) -> crate::tmux::SessionInfo {
        crate::tmux::SessionInfo {
            session_name: "main".into(),
            windows,
        }
    }

    #[test]
    fn group_panes_empty_sessions() {
        let groups = group_panes_by_repo(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn group_panes_same_repo() {
        // Two panes in the same real repo should be grouped together
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pane1 = test_pane("%1", manifest_dir);
        let pane2 = test_pane("%2", manifest_dir);

        let sessions = vec![test_session(vec![test_window(vec![pane1, pane2], true)])];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(groups.len(), 1, "same repo path should produce one group");
        assert_eq!(groups[0].panes.len(), 2);
        assert_eq!(groups[0].panes[0].0.pane_id, "%1");
        assert_eq!(groups[0].panes[1].0.pane_id, "%2");
    }

    #[test]
    fn group_panes_non_git_path_uses_raw_path() {
        // A non-git path should use the raw path as the group key
        let pane = test_pane("%1", "/tmp/no-git-here");

        let sessions = vec![test_session(vec![test_window(vec![pane], true)])];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "no-git-here");
    }

    #[test]
    fn group_panes_display_name_is_basename() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pane = test_pane("%1", manifest_dir);

        let sessions = vec![test_session(vec![test_window(vec![pane], true)])];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(groups.len(), 1);
        let expected_name = std::path::Path::new(manifest_dir)
            .file_name()
            .unwrap()
            .to_string_lossy();
        assert_eq!(
            groups[0].name, expected_name,
            "display name should be repo basename"
        );
    }

    #[test]
    fn group_panes_has_focus_from_active_window_and_pane() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut pane = test_pane("%1", manifest_dir);
        pane.pane_active = true;

        let sessions = vec![test_session(vec![test_window(vec![pane], true)])];
        let groups = group_panes_by_repo(&sessions);

        assert!(
            groups[0].has_focus,
            "active pane in active window should set has_focus"
        );
    }

    #[test]
    fn group_panes_no_focus_when_window_inactive() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut pane = test_pane("%1", manifest_dir);
        pane.pane_active = true;

        let sessions = vec![test_session(vec![test_window(vec![pane], false)])]; // window_active=false
        let groups = group_panes_by_repo(&sessions);

        assert!(
            !groups[0].has_focus,
            "active pane in inactive window should not set has_focus"
        );
    }

    #[test]
    fn group_panes_empty_path_pane() {
        let pane = test_pane("%1", "");

        let sessions = vec![test_session(vec![test_window(vec![pane], true)])];
        let groups = group_panes_by_repo(&sessions);

        // Empty path pane should still be grouped (by empty key)
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn group_panes_multiple_sessions() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pane1 = test_pane("%1", manifest_dir);
        let pane2 = test_pane("%2", "/tmp/other-project");

        let sessions = vec![
            test_session(vec![test_window(vec![pane1], true)]),
            crate::tmux::SessionInfo {
                session_name: "other".into(),
                windows: vec![test_window(vec![pane2], false)],
            },
        ];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(
            groups.len(),
            2,
            "different repos across sessions should produce separate groups"
        );
    }

    #[test]
    fn group_panes_same_repo_across_sessions_merge_into_one_group() {
        // Regression for the `state.sessions` field removal: panes that
        // live in different tmux sessions but share the same repo path
        // must still collapse into a single `RepoGroup`. This is what
        // makes the sidebar usable across multi-session workflows.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let pane_session_a = test_pane("%1", manifest_dir);
        let pane_session_b = test_pane("%2", manifest_dir);

        let sessions = vec![
            crate::tmux::SessionInfo {
                session_name: "alpha".into(),
                windows: vec![test_window(vec![pane_session_a], true)],
            },
            crate::tmux::SessionInfo {
                session_name: "beta".into(),
                windows: vec![test_window(vec![pane_session_b], false)],
            },
        ];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(
            groups.len(),
            1,
            "panes in the same repo across sessions must merge into one group"
        );
        assert_eq!(groups[0].panes.len(), 2);
        let pane_ids: Vec<&str> = groups[0]
            .panes
            .iter()
            .map(|(p, _)| p.pane_id.as_str())
            .collect();
        assert!(pane_ids.contains(&"%1"));
        assert!(pane_ids.contains(&"%2"));
    }

    #[test]
    fn group_panes_sorted_by_name_case_insensitive() {
        // Groups should be sorted alphabetically regardless of encounter order
        let pane1 = test_pane("%1", "/tmp/zzz");
        let pane2 = test_pane("%2", "/tmp/Aaa");
        let pane3 = test_pane("%3", "/tmp/mmm");
        let pane4 = test_pane("%4", "/tmp/zzz");

        let sessions = vec![test_session(vec![test_window(
            vec![pane1, pane2, pane3, pane4],
            true,
        )])];
        let groups = group_panes_by_repo(&sessions);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Aaa");
        assert_eq!(groups[1].name, "mmm");
        assert_eq!(groups[2].name, "zzz");
        assert_eq!(groups[2].panes.len(), 2, "zzz should have 2 panes");
    }

    // ─── nesting tests ──────────────────────────────────────────────

    fn subagent_pane(id: &str, parent: crate::tmux::SubagentParent) -> PaneInfo {
        let mut pane = test_pane(id, env!("CARGO_MANIFEST_DIR"));
        pane.subagent = Some(crate::tmux::SubagentInfo {
            kind: crate::pmux_task::PmuxTaskKind::Spawned,
            task: None,
            parent,
        });
        pane
    }

    fn pane_parent(id: &str) -> crate::tmux::SubagentParent {
        crate::tmux::SubagentParent::Pane(id.to_string())
    }

    fn with_git(panes: Vec<PaneInfo>) -> Vec<(PaneInfo, PaneGitInfo)> {
        panes
            .into_iter()
            .map(|pane| (pane, PaneGitInfo::default()))
            .collect()
    }

    fn ids(panes: &[(PaneInfo, PaneGitInfo)]) -> Vec<&str> {
        panes
            .iter()
            .map(|(pane, _)| pane.pane_id.as_str())
            .collect()
    }

    /// Group the panes through `group_panes_by_repo` and return each group's
    /// pane ids in drawn order.
    fn grouped(panes: Vec<PaneInfo>) -> Vec<Vec<String>> {
        let sessions = vec![test_session(vec![test_window(panes, true)])];
        group_panes_by_repo(&sessions)
            .into_iter()
            .map(|group| group.panes.iter().map(|(p, _)| p.pane_id.clone()).collect())
            .collect()
    }

    #[test]
    fn nest_roots_maps_a_child_to_its_parent() {
        let panes = with_git(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%1", pane_parent("%0")),
        ]);
        let roots = nest_roots(&panes);

        assert_eq!(roots.get("%1").map(String::as_str), Some("%0"));
        assert!(!roots.contains_key("%0"), "a root maps to no parent");
    }

    #[test]
    fn nest_roots_maps_a_grandchild_to_the_same_root() {
        let panes = with_git(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%1", pane_parent("%0")),
            subagent_pane("%2", pane_parent("%1")),
        ]);
        let roots = nest_roots(&panes);

        assert_eq!(roots.get("%1").map(String::as_str), Some("%0"));
        assert_eq!(
            roots.get("%2").map(String::as_str),
            Some("%0"),
            "every depth resolves to the ultimate parent"
        );
    }

    #[test]
    fn nested_under_is_true_only_when_the_root_is_visible() {
        let panes = with_git(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%1", pane_parent("%0")),
        ]);
        let roots = nest_roots(&panes);
        let visible: std::collections::HashSet<&str> = ["%0", "%1"].into_iter().collect();

        assert!(
            nested_under(&roots, &visible, "%1"),
            "a child indents when its root is drawn"
        );
        assert!(
            !nested_under(&roots, &visible, "%0"),
            "a root is not indented"
        );
    }

    #[test]
    fn nested_under_is_false_when_a_filter_hides_the_root() {
        let panes = with_git(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%1", pane_parent("%0")),
        ]);
        let roots = nest_roots(&panes);
        let visible: std::collections::HashSet<&str> = ["%1"].into_iter().collect();

        assert!(
            !nested_under(&roots, &visible, "%1"),
            "a child draws top-level when its root is hidden"
        );
        assert!(
            !nested_under(&roots, &visible, "%2"),
            "a pane without a root never indents"
        );
    }

    #[test]
    fn group_panes_puts_a_child_directly_after_its_parent() {
        let groups = grouped(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%1", pane_parent("%0")),
            test_pane("%2", env!("CARGO_MANIFEST_DIR")),
        ]);

        assert_eq!(groups, vec![vec!["%0", "%1", "%2"]]);
    }

    #[test]
    fn group_panes_keeps_children_in_tmux_order() {
        let groups = grouped(vec![
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
            subagent_pane("%3", pane_parent("%0")),
            subagent_pane("%1", pane_parent("%0")),
        ]);

        assert_eq!(groups, vec![vec!["%0", "%3", "%1"]]);
    }

    #[test]
    fn group_panes_adds_no_pane_when_a_parent_follows_its_children() {
        let panes = vec![
            subagent_pane("%2", pane_parent("%1")),
            subagent_pane("%1", pane_parent("%0")),
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
        ];
        let input_count = panes.len();
        let groups = grouped(panes);

        // The children keep their tmux order (`%2` before `%1`); only the
        // root moves ahead of them.
        assert_eq!(groups, vec![vec!["%0", "%2", "%1"]]);
        assert_eq!(groups[0].len(), input_count, "no pane is lost or repeated");
    }

    #[test]
    fn nest_roots_ignores_a_parent_in_another_group() {
        let groups = grouped(vec![
            subagent_pane("%1", pane_parent("%0")),
            test_pane("%0", "/tmp/pmux-nest-other"),
        ]);

        assert_eq!(groups.len(), 2, "the two paths form two groups");
        let parent_group = groups
            .iter()
            .find(|ids| ids.iter().any(|id| id == "%0"))
            .expect("the parent's group");
        let child_group = groups
            .iter()
            .find(|ids| ids.iter().any(|id| id == "%1"))
            .expect("the child's group");
        assert_eq!(parent_group, &vec!["%0".to_string()]);
        assert_eq!(
            child_group,
            &vec!["%1".to_string()],
            "a cross-group child stays top-level"
        );
    }

    #[test]
    fn nest_roots_nests_children_under_an_orphaned_subagent() {
        let panes = with_git(vec![
            subagent_pane("%1", crate::tmux::SubagentParent::Orphaned),
            subagent_pane("%2", pane_parent("%1")),
        ]);
        let roots = nest_roots(&panes);

        assert_eq!(
            roots.get("%2").map(String::as_str),
            Some("%1"),
            "the reachable top of the chain is the orphan itself"
        );
        assert!(!roots.contains_key("%1"), "the orphan is a root");
    }

    #[test]
    fn group_panes_orders_children_after_an_orphaned_root() {
        let groups = grouped(vec![
            subagent_pane("%1", crate::tmux::SubagentParent::Orphaned),
            subagent_pane("%2", pane_parent("%1")),
            test_pane("%0", env!("CARGO_MANIFEST_DIR")),
        ]);

        assert_eq!(groups, vec![vec!["%1", "%2", "%0"]]);
    }

    #[test]
    fn nest_roots_breaks_a_two_pane_cycle_at_the_first_pane() {
        let panes = with_git(vec![
            subagent_pane("%0", pane_parent("%1")),
            subagent_pane("%1", pane_parent("%0")),
        ]);
        let roots = nest_roots(&panes);

        assert_eq!(roots.get("%1").map(String::as_str), Some("%0"));
        assert!(!roots.contains_key("%0"), "the first pane is the root");

        // The same cycle, listed the other way round, roots at `%1`.
        let flipped = with_git(vec![
            subagent_pane("%1", pane_parent("%0")),
            subagent_pane("%0", pane_parent("%1")),
        ]);
        assert_eq!(
            nest_roots(&flipped).get("%0").map(String::as_str),
            Some("%1")
        );
    }

    #[test]
    fn group_panes_keeps_every_pane_of_a_cycle() {
        let groups = grouped(vec![
            subagent_pane("%0", pane_parent("%1")),
            subagent_pane("%1", pane_parent("%0")),
        ]);

        assert_eq!(groups, vec![vec!["%0", "%1"]]);
    }

    #[test]
    fn nest_roots_treats_a_self_parent_as_a_root() {
        let panes = with_git(vec![subagent_pane("%0", pane_parent("%0"))]);
        let roots = nest_roots(&panes);

        assert!(roots.is_empty(), "a self-parent is its own root");
        assert_eq!(ids(&panes), vec!["%0"]);
    }

    #[test]
    fn group_panes_keeps_a_self_parented_pane_alone() {
        let groups = grouped(vec![
            subagent_pane("%0", pane_parent("%0")),
            test_pane("%1", env!("CARGO_MANIFEST_DIR")),
        ]);

        assert_eq!(groups, vec![vec!["%0", "%1"]]);
    }
}
