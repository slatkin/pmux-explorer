use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::SPAWN_BUTTON;
use super::row;
use crate::state::{AppState, Focus, LayoutKind};
use crate::ui::text::display_width;

/// Build a repo group's header: the repo name at column 0, with the spawn
/// button right-aligned on the same row. A group with no resolved
/// `repo_root` is not a git repository, so it gets no button. The recorded
/// spawn target keeps the bare repo name, because the button acts on the
/// repository, not on the heading.
fn repo_header_line(
    collected: &mut CollectedRows,
    group_name: &str,
    repo_root: Option<&str>,
    width: usize,
    color: ratatui::style::Color,
) -> Line<'static> {
    let Some(root) = repo_root else {
        return Line::from(Span::styled(
            group_name.to_string(),
            Style::default().fg(color),
        ));
    };
    let pad_width = width
        .saturating_sub(display_width(group_name))
        .saturating_sub(SPAWN_BUTTON.len());
    collected.pending_spawn.push((
        collected.lines.len(),
        group_name.to_string(),
        root.to_string(),
    ));
    Line::from(vec![
        Span::styled(group_name.to_string(), Style::default().fg(color)),
        Span::raw(" ".repeat(pad_width)),
        Span::styled(SPAWN_BUTTON, Style::default().fg(color)),
    ])
}

#[derive(Debug, Default)]
pub(super) struct CollectedRows {
    pub lines: Vec<Line<'static>>,
    pub line_to_row: Vec<Option<usize>>,
    pub pending_spawn: Vec<(usize, String, String)>,
    pub pending_remove: Vec<(usize, u16, String)>,
}

pub(super) fn collect(state: &AppState, width: u16) -> CollectedRows {
    let width = width as usize;
    let theme = &state.theme;

    let mut collected = CollectedRows::default();
    let filter = state.global.status_filter;
    // A strip has no rows to spare for a dedicated branch line: the
    // branch label rides the pane's status row instead.
    let inline_branch = state.layout_kind == LayoutKind::Strip;
    let mut first_group = true;
    let mut row_index: usize = 0;

    for group in &state.repo_groups {
        if !state.global.repo_filter.matches_group(&group.name) {
            continue;
        }
        let filtered_panes: Vec<_> = group
            .panes
            .iter()
            .filter(|(pane, _)| filter.matches(&pane.status))
            .collect();
        if filtered_panes.is_empty() {
            continue;
        }

        if !first_group {
            // Separate repo groups, but do not add a leading blank before
            // the first repo so the list starts immediately below the header.
            collected.lines.push(Line::from(""));
            collected.line_to_row.push(None);
        }
        first_group = false;

        let group_has_focused_pane = state
            .focus_state
            .focused_pane_id
            .as_ref()
            .is_some_and(|fid| group.panes.iter().any(|(p, _)| p.pane_id == *fid));

        // Plain repo header at column 0, with a `[+]` spawn button
        // right-aligned on the same row. Only a group with a resolved
        // repo_root is a git repository, so only that one gets the button.
        let color = if group_has_focused_pane {
            theme.accent
        } else {
            theme.text_active
        };
        let repo_root = group
            .panes
            .iter()
            .find_map(|(_, git)| git.repo_root.as_deref());
        let header = repo_header_line(&mut collected, &group.name, repo_root, width, color);
        collected.lines.push(header);
        collected.line_to_row.push(None);

        for (pane, git_info) in filtered_panes.iter() {
            collect_pane(
                &mut collected,
                state,
                pane,
                git_info,
                row_index,
                width,
                inline_branch,
            );
            row_index += 1;
        }
    }

    collected
}

/// Emit one pane's rows into `collected`: the status/branch/body
/// lines with their `line_to_row` mappings, plus the pane-level
/// click target the drawn glyph carries — the `×` remove marker on a
/// spawn-created worktree's branch row.
fn collect_pane(
    collected: &mut CollectedRows,
    state: &AppState,
    pane: &crate::tmux::PaneInfo,
    git_info: &crate::group::PaneGitInfo,
    row_index: usize,
    width: usize,
    inline_branch: bool,
) {
    let theme = &state.theme;
    let is_selected =
        state.focus_state.focus == Focus::Panes && row_index == state.global.selected_pane_row;

    let is_active = state.focus_state.focused_pane_id.as_ref() == Some(&pane.pane_id);

    let pane_state = state.pane_state(&pane.pane_id);
    let ports = pane_state.map(|s| s.ports.as_slice());
    let task_progress = pane_state.and_then(|s| s.task_progress.as_ref());
    let status_line_idx = collected.lines.len();
    let pane_lines = row::render_pane_lines_inline_branch(
        pane,
        git_info,
        ports,
        task_progress,
        is_selected,
        is_active,
        width,
        &state.icons,
        theme,
        state.spinner_frame,
        state.now,
        inline_branch,
    );
    let pane_line_count = pane_lines.len();
    collected.lines.extend(pane_lines);
    for _ in 0..pane_line_count {
        collected.line_to_row.push(Some(row_index));
    }

    // The branch row is always `status_line_idx + 1` when
    // `branch_ports_row` emits a line (which requires a
    // non-empty branch). Look up the exact column of the
    // trailing `×` from the row helper so the click target
    // lines up with the rendered glyph even when the branch
    // name truncates.
    if pane.sidebar_spawned
        && git_info.is_worktree
        && pane_line_count >= 2
        && let Some(x) =
            row::sidebar_remove_marker_col(git_info, ports, true, width.saturating_sub(2))
    {
        collected
            .pending_remove
            .push((status_line_idx + 1, x, pane.pane_id.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{PaneGitInfo, RepoGroup};
    use crate::state::{AppState, StatusFilter};
    use crate::tmux::{AgentType, PaneInfo, PaneStatus, PermissionMode, WorktreeMetadata};

    fn make_pane(id: &str, status: PaneStatus) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            pane_active: false,
            status,
            attention: false,
            agent: AgentType::Claude,
            path: "/tmp/repo".into(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        }
    }

    #[test]
    fn collect_empty_repo_groups_produces_no_lines() {
        let state = AppState::new("%0".into());
        let collected = collect(&state, 40);
        assert!(collected.lines.is_empty());
        assert!(collected.line_to_row.is_empty());
        assert!(collected.pending_spawn.is_empty());
        assert!(collected.pending_remove.is_empty());
    }

    #[test]
    fn collect_skips_group_when_status_filter_excludes_all_panes() {
        let mut state = AppState::new("%0".into());
        // The group has only Running panes, so filter to Waiting to drop them all.
        state.global.status_filter = StatusFilter::Waiting;
        state.repo_groups = vec![RepoGroup {
            name: "repo".into(),
            has_focus: false,
            panes: vec![(make_pane("%1", PaneStatus::Running), PaneGitInfo::default())],
        }];
        let collected = collect(&state, 40);
        assert!(collected.lines.is_empty());
        assert!(collected.pending_spawn.is_empty());
    }

    #[test]
    fn collect_records_pending_spawn_when_repo_root_present() {
        let mut state = AppState::new("%0".into());
        let git_info = PaneGitInfo {
            repo_root: Some("/tmp/repo".into()),
            branch: None,
            is_worktree: false,
            worktree_name: None,
        };
        state.repo_groups = vec![RepoGroup {
            name: "repo".into(),
            has_focus: false,
            panes: vec![(make_pane("%1", PaneStatus::Running), git_info)],
        }];
        let collected = collect(&state, 40);
        assert_eq!(
            collected.pending_spawn.len(),
            1,
            "groups with a repo_root should emit a spawn target"
        );
        assert_eq!(collected.pending_spawn[0].1, "repo");
        assert_eq!(collected.pending_spawn[0].2, "/tmp/repo");
        // At least the header plus one pane row should have been pushed.
        assert!(!collected.lines.is_empty());
    }

    #[test]
    fn collect_no_pending_spawn_without_repo_root() {
        let mut state = AppState::new("%0".into());
        state.repo_groups = vec![RepoGroup {
            name: "raw-path".into(),
            has_focus: false,
            panes: vec![(make_pane("%1", PaneStatus::Running), PaneGitInfo::default())],
        }];
        let collected = collect(&state, 40);
        assert!(
            collected.pending_spawn.is_empty(),
            "groups without repo_root must not produce spawn targets"
        );
    }

    #[test]
    fn collect_in_a_strip_lifts_the_branch_onto_the_status_row() {
        // A strip draws no dedicated branch line: the status row
        // carries the branch label, so a branch-only pane emits a
        // single line under the repo header.
        let mut state = AppState::new("%0".into());
        state.layout_kind = crate::state::LayoutKind::Strip;
        let git_info = PaneGitInfo {
            repo_root: Some("/tmp/repo".into()),
            branch: Some("main".into()),
            is_worktree: false,
            worktree_name: None,
        };
        state.repo_groups = vec![RepoGroup {
            name: "repo".into(),
            has_focus: false,
            panes: vec![(make_pane("%1", PaneStatus::Running), git_info)],
        }];
        let collected = collect(&state, 40);
        let line_text = |line: &Line<'_>| -> String {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        };
        assert_eq!(collected.lines.len(), 2, "header + status row only");
        let status = line_text(&collected.lines[1]);
        assert!(
            status.contains("main"),
            "the status row carries the branch: {status:?}"
        );
    }

    #[test]
    fn collect_in_a_strip_keeps_the_ports_row() {
        // Only the branch name lifts: a pane with listening ports
        // still emits its second row for them.
        let mut state = AppState::new("%0".into());
        state.layout_kind = crate::state::LayoutKind::Strip;
        let git_info = PaneGitInfo {
            repo_root: Some("/tmp/repo".into()),
            branch: Some("main".into()),
            is_worktree: false,
            worktree_name: None,
        };
        state.repo_groups = vec![RepoGroup {
            name: "repo".into(),
            has_focus: false,
            panes: vec![(make_pane("%1", PaneStatus::Running), git_info)],
        }];
        state.pane_state_mut("%1").ports.extend_from_slice(&[8080]);
        let collected = collect(&state, 40);
        assert_eq!(collected.lines.len(), 3, "header + status row + ports row");
        let ports_line: String = collected.lines[2]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            ports_line.contains(":8080"),
            "the ports row still renders: {ports_line:?}"
        );
    }

    #[test]
    fn collect_in_a_strip_clamps_the_prompt_to_one_truncated_line() {
        // A strip has no rows to spare for a tall preview: the
        // last-conversation marker renders one ellipsis-closed line
        // where a column would wrap to three.
        let mut state = AppState::new("%0".into());
        state.layout_kind = crate::state::LayoutKind::Strip;
        let mut pane = make_pane("%1", PaneStatus::Running);
        pane.prompt =
            "a very long prompt that would wrap across several lines in the column layout".into();
        state.repo_groups = vec![RepoGroup {
            name: "repo".into(),
            has_focus: false,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        let collected = collect(&state, 40);
        assert_eq!(
            collected.lines.len(),
            3,
            "header + status row + one prompt line"
        );
        let prompt_line: String = collected.lines[2]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            prompt_line.contains('\u{2026}'),
            "the single prompt line must close with an ellipsis: {prompt_line:?}"
        );
    }

    #[test]
    fn collect_pending_spawn_grows_with_repo_root_bearing_groups() {
        let mut state = AppState::new("%0".into());
        let with_root = |root: &str, name: &str, pane_id: &str| RepoGroup {
            name: name.into(),
            has_focus: false,
            panes: vec![(
                make_pane(pane_id, PaneStatus::Running),
                PaneGitInfo {
                    repo_root: Some(root.into()),
                    branch: None,
                    is_worktree: false,
                    worktree_name: None,
                },
            )],
        };
        state.repo_groups = vec![
            with_root("/repo/a", "a", "%1"),
            with_root("/repo/b", "b", "%2"),
            with_root("/repo/c", "c", "%3"),
        ];
        let collected = collect(&state, 40);
        assert_eq!(collected.pending_spawn.len(), 3);
    }
}
