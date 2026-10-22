#[allow(dead_code, unused_imports)]
mod test_helpers;

use test_helpers::*;
use tmux_agent_sidebar::activity::ActivityEntry;
use tmux_agent_sidebar::group::{PaneGitInfo, RepoGroup};
use tmux_agent_sidebar::state::{
    AppState, BottomTab, Focus, GlobalState, PopupState, RepoFilter, RowTarget, ScrollState,
    StatusFilter,
};
use tmux_agent_sidebar::tmux::{
    self, AgentType, PaneInfo, PaneStatus, SessionInfo, WindowInfo, WorktreeMetadata,
};
use tmux_agent_sidebar::worktree;

// ─── State Transition Tests ────────────────────────────────────────

#[test]
fn test_move_pane_selection_bounds() {
    let mut state = make_state(vec![]);
    state.layout.pane_row_targets = vec![
        RowTarget {
            pane_id: "%1".into(),
        },
        RowTarget {
            pane_id: "%2".into(),
        },
    ];
    state.global.selected_pane_row = 0;
    state.move_pane_selection(1);
    assert_eq!(state.global.selected_pane_row, 1);
    state.move_pane_selection(1); // should not go past end
    assert_eq!(state.global.selected_pane_row, 1);
    state.move_pane_selection(-1);
    assert_eq!(state.global.selected_pane_row, 0);
    state.move_pane_selection(-1); // should not go below 0
    assert_eq!(state.global.selected_pane_row, 0);
}

#[test]
fn test_move_pane_selection_empty() {
    let mut state = make_state(vec![]);
    state.move_pane_selection(1);
    assert_eq!(state.global.selected_pane_row, 0);
}

#[test]
fn test_scroll_activity_bounds() {
    let mut state = make_state(vec![]);
    state.activity.entries = vec![
        ActivityEntry {
            timestamp: "10:00".into(),
            tool: "Read".into(),
            label: "a".into(),
        },
        ActivityEntry {
            timestamp: "10:01".into(),
            tool: "Edit".into(),
            label: "b".into(),
        },
        ActivityEntry {
            timestamp: "10:02".into(),
            tool: "Bash".into(),
            label: "c".into(),
        },
    ];
    state.activity.scroll.total_lines = 6;
    state.activity.scroll.visible_height = 4;
    state.activity.scroll.scroll(1);
    assert_eq!(state.activity.scroll.offset, 1);
    state.activity.scroll.scroll(5);
    assert_eq!(state.activity.scroll.offset, 2); // clamped to 6-4=2
    state.activity.scroll.scroll(-10);
    assert_eq!(state.activity.scroll.offset, 0);
}

// ─── line_to_row Mapping Tests ─────────────────────────────────────

#[test]
fn test_line_to_row_single_agent() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", vec![pane])];
    state.rebuild_row_targets();
    let _ = render_to_styled_string(&mut state, 28, 10);
    // repo header, agent status, idle hint
    assert_eq!(state.layout.line_to_row.len(), 3);
    assert_eq!(state.layout.line_to_row[0], None); // repo header
    assert_eq!(state.layout.line_to_row[1], Some(0)); // agent status
    assert_eq!(state.layout.line_to_row[2], Some(0)); // idle hint
}

#[test]
fn test_line_to_row_two_agents() {
    let pane1 = PaneInfo {
        pane_id: "%1".into(),
        pane_active: true,
        status: PaneStatus::Running,
        attention: false,
        agent: AgentType::Claude,
        path: "/home/user/project".into(),
        current_command: String::new(),
        prompt: String::new(),
        prompt_is_response: false,
        started_at: None,
        wait_reason: String::new(),
        permission_mode: tmux_agent_sidebar::tmux::PermissionMode::Default,
        subagents: vec![],
        pane_pid: None,
        worktree: WorktreeMetadata::default(),
        session_id: None,
        sidebar_spawned: false,
        pane_name: None,
        bg_shell_cmd: None,
        subagent: None,
    };
    let pane2 = PaneInfo {
        pane_id: "%2".into(),
        pane_active: false,
        status: PaneStatus::Idle,
        attention: false,
        agent: AgentType::Codex,
        path: "/home/user/project".into(),
        current_command: String::new(),
        prompt: String::new(),
        prompt_is_response: false,
        started_at: None,
        wait_reason: String::new(),
        permission_mode: tmux_agent_sidebar::tmux::PermissionMode::Default,
        subagents: vec![],
        pane_pid: None,
        worktree: WorktreeMetadata::default(),
        session_id: None,
        sidebar_spawned: false,
        pane_name: None,
        bg_shell_cmd: None,
        subagent: None,
    };

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane1.clone(), pane2.clone()],
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", vec![pane1, pane2])];
    state.rebuild_row_targets();
    let _ = render_to_styled_string(&mut state, 28, 10);
    // repo header, agent1, agent2 status+hint
    assert_eq!(state.layout.line_to_row.len(), 4);
    assert_eq!(state.layout.line_to_row[0], None); // repo header
    assert_eq!(state.layout.line_to_row[1], Some(0)); // agent 1
    assert_eq!(state.layout.line_to_row[2], Some(1)); // agent 2 status line
    assert_eq!(state.layout.line_to_row[3], Some(1)); // agent 2 idle hint
}

#[test]
fn test_line_to_row_with_prompt() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    pane.prompt = "hello".into();

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", vec![pane])];
    state.rebuild_row_targets();
    let _ = render_to_styled_string(&mut state, 28, 10);
    // repo header, agent status, prompt
    assert_eq!(state.layout.line_to_row.len(), 3);
    assert_eq!(state.layout.line_to_row[0], None); // repo header
    assert_eq!(state.layout.line_to_row[1], Some(0)); // agent status line
    assert_eq!(state.layout.line_to_row[2], Some(0)); // prompt line
}

#[test]
fn test_line_to_row_with_version_banner() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", vec![pane])];
    state.version_notice = Some(tmux_agent_sidebar::version::UpdateNotice {
        local_version: "0.2.6".into(),
        latest_version: "0.2.7".into(),
    });
    state.rebuild_row_targets();
    let _ = render_to_string(&mut state, 28, 10);
    // version banner should still stay out of the scrollable list
    assert_eq!(state.layout.line_to_row.len(), 3);
    assert_eq!(state.layout.line_to_row[0], None); // repo header
    assert_eq!(state.layout.line_to_row[1], Some(0)); // agent status line
    assert_eq!(state.layout.line_to_row[2], Some(0)); // idle hint
}

#[test]
fn test_secondary_header_click_on_i_opens_notices_popup_even_without_missing_hooks() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", vec![pane])];
    // make_state() seeds a missing hook group for general-purpose
    // tests; clear it here so the test actually exercises the
    // version-notice-only path described in the test name.
    state.notices.missing_hook_groups.clear();
    state.version_notice = Some(tmux_agent_sidebar::version::UpdateNotice {
        local_version: "0.2.6".into(),
        latest_version: "0.2.7".into(),
    });
    state.rebuild_row_targets();
    let _ = render_to_string(&mut state, 28, 10);

    state.handle_mouse_click(1, 0);
    assert!(
        state.is_notices_popup_open(),
        "i should stay clickable even when there are no missing hooks"
    );
}

// ─── Coverage Gap Tests ─────────────────────────────────────────────

#[test]
fn test_rebuild_row_targets_clamps_selection() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut p2 = pane.clone();
    p2.pane_id = "%2".into();
    let mut state = make_state(vec![]);
    state.repo_groups = vec![RepoGroup {
        name: "project".into(),
        has_focus: true,
        panes: vec![
            (pane.clone(), PaneGitInfo::default()),
            (p2.clone(), PaneGitInfo::default()),
        ],
    }];
    state.global.selected_pane_row = 1; // select second agent

    // Trigger rebuild
    state.rebuild_row_targets();
    assert_eq!(state.layout.pane_row_targets.len(), 2);

    // Now shrink to 1 agent
    state.repo_groups[0].panes.pop();
    state.global.selected_pane_row = 1; // still pointing at index 1
    state.rebuild_row_targets();
    // Should be clamped to 0
    assert_eq!(state.global.selected_pane_row, 0);
}

// find_focused_pane now queries tmux directly, so it can't be tested
// without a tmux session. The underlying logic (pick_active_pane) is
// tested via unit tests in tmux.rs. focused_pane_id is pub, so tests
// can set it directly.

#[test]
fn test_scroll_git_empty_is_noop() {
    let mut state = make_state(vec![]);
    state.scrolls.git.offset = 0;
    state.bottom_tab = BottomTab::GitStatus;
    state.scroll_bottom(5);
    assert_eq!(
        state.scrolls.git.offset, 0,
        "scrolling empty git should be no-op"
    );
}

// ─── State: scroll_git Tests ────────────────────────────────────────

#[test]
fn test_scroll_git_bounds() {
    let mut state = make_state(vec![]);
    state.git.unstaged_files = vec![tmux_agent_sidebar::git::GitFileEntry {
        status: 'M',
        name: "file.rs".into(),
        additions: 0,
        deletions: 0,
        path: String::new(),
    }];
    state.scrolls.git.total_lines = 8;
    state.scrolls.git.visible_height = 3;
    state.scrolls.git.offset = 0;

    state.scrolls.git.scroll(2);
    assert_eq!(state.scrolls.git.offset, 2);

    // Clamp to max (8 - 3 = 5)
    state.scrolls.git.scroll(10);
    assert_eq!(state.scrolls.git.offset, 5);

    // Clamp to 0
    state.scrolls.git.scroll(-100);
    assert_eq!(state.scrolls.git.offset, 0);
}

// ─── State: apply_git_data Tests ────────────────────────────────────

#[test]
fn test_apply_git_data() {
    use tmux_agent_sidebar::git::{GitData, GitFileEntry};

    let mut state = make_state(vec![]);
    let data = GitData {
        diff_stat: Some((10, 5)),
        branch: "feature/test".into(),
        ahead_behind: Some((2, 1)),
        staged_files: vec![GitFileEntry {
            status: 'M',
            name: "src/lib.rs".into(),
            additions: 10,
            deletions: 5,
            path: String::new(),
        }],
        unstaged_files: vec![],
        untracked_files: vec![],
        remote_url: "https://github.com/user/repo".into(),
        pr_number: Some("42".into()),
    };

    state.apply_git_data(data);

    assert_eq!(state.git.staged_files.len(), 1);
    assert_eq!(state.git.staged_files[0].status, 'M');
    assert_eq!(state.git.staged_files[0].name, "src/lib.rs");
    assert!(state.git.unstaged_files.is_empty());
    assert!(state.git.untracked_files.is_empty());
    assert_eq!(state.git.changed_file_count(), 1);
    assert_eq!(state.git.diff_stat, Some((10, 5)));
    assert_eq!(state.git.branch, "feature/test");
    assert_eq!(state.git.ahead_behind, Some((2, 1)));
    assert_eq!(state.git.remote_url, "https://github.com/user/repo");
    assert_eq!(state.git.pr_number, Some("42".into()));
}

// ─── State: new Tests ───────────────────────────────────────────────

#[test]
fn test_state_new_defaults() {
    let state = AppState::new("%99".into());
    assert_eq!(state.now, 0);
    assert_eq!(state.tmux_pane, "%99");
    assert!(state.repo_groups.is_empty());
    assert!(!state.focus_state.sidebar_focused);
    assert_eq!(state.focus_state.focus, Focus::Panes);
    assert_eq!(state.spinner_frame, 0);
    assert_eq!(state.global.selected_pane_row, 0);
    assert!(state.layout.pane_row_targets.is_empty());
    assert!(state.activity.entries.is_empty());
    assert_eq!(state.activity.scroll.offset, 0);
    assert_eq!(state.activity.max_entries, 50);
    assert_eq!(state.scrolls.panes.offset, 0);
    assert_eq!(state.scrolls.panes.total_lines, 0);
    assert_eq!(state.scrolls.panes.visible_height, 0);
    assert_eq!(state.bottom_tab, BottomTab::Activity);
    assert!(state.git.branch.is_empty());
    assert_eq!(state.scrolls.git.offset, 0);
    assert!(state.git.pr_number.is_none());
}

// ─── State: move_pane_selection return value Tests ─────────────────

#[test]
fn test_move_pane_selection_return_value() {
    let mut state = make_state(vec![]);
    state.layout.pane_row_targets = vec![
        RowTarget {
            pane_id: "%1".into(),
        },
        RowTarget {
            pane_id: "%2".into(),
        },
    ];
    state.global.selected_pane_row = 0;

    assert!(
        state.move_pane_selection(1),
        "should return true when moved"
    );
    assert!(
        !state.move_pane_selection(1),
        "should return false at boundary"
    );
    assert!(
        state.move_pane_selection(-1),
        "should return true when moved back"
    );
    assert!(
        !state.move_pane_selection(-1),
        "should return false at start"
    );
}

// find_focused_pane edge case tests were removed because the function now
// queries tmux directly. See tmux::find_active_pane tests instead.

// ─── State: scroll_bottom dispatch Tests ────────────────────────────

#[test]
fn test_scroll_bottom_dispatches_to_git() {
    let mut state = make_state(vec![]);
    state.bottom_tab = BottomTab::GitStatus;
    state.git.unstaged_files = vec![tmux_agent_sidebar::git::GitFileEntry {
        status: 'M',
        name: "file.rs".into(),
        additions: 0,
        deletions: 0,
        path: String::new(),
    }];
    state.scrolls.git.total_lines = 10;
    state.scrolls.git.visible_height = 3;
    state.scrolls.git.offset = 0;

    state.scroll_bottom(2);
    assert_eq!(state.scrolls.git.offset, 2);
}

#[test]
fn test_scroll_bottom_dispatches_to_activity() {
    let mut state = make_state(vec![]);
    state.bottom_tab = BottomTab::Activity;
    state.activity.entries = vec![ActivityEntry {
        timestamp: "10:00".into(),
        tool: "Read".into(),
        label: "a".into(),
    }];
    state.activity.scroll.total_lines = 10;
    state.activity.scroll.visible_height = 3;
    state.activity.scroll.offset = 0;

    state.scroll_bottom(2);
    assert_eq!(state.activity.scroll.offset, 2);
}

// ─── State: next_bottom_tab cycle Tests ─────────────────────────────

#[test]
fn test_next_bottom_tab_full_cycle() {
    let mut state = make_state(vec![]);
    assert_eq!(state.bottom_tab, BottomTab::Activity);
    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::Tasks);
    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::GitStatus);
    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::Activity);
}

// ─── State: scroll_activity empty Tests ─────────────────────────────

#[test]
fn test_scroll_activity_empty_is_noop() {
    let mut state = make_state(vec![]);
    state.activity.scroll.offset = 0;
    state.activity.scroll.scroll(5);
    assert_eq!(
        state.activity.scroll.offset, 0,
        "scrolling empty activity should be no-op"
    );
}

// ─── State: git tab active flag Tests ───────────────────────────────

#[test]
fn test_git_tab_active_after_tab_switch() {
    let mut state = make_state(vec![]);
    assert_eq!(state.bottom_tab, BottomTab::Activity);

    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::Tasks);

    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::GitStatus);

    state.next_bottom_tab();
    assert_eq!(state.bottom_tab, BottomTab::Activity);
}

// ─── State: global sync → rebuild consistency Tests ─────────────

#[test]
fn test_filter_change_rebuilds_row_targets() {
    use tmux_agent_sidebar::state::StatusFilter;

    let running_pane = PaneInfo {
        pane_id: "%1".into(),
        status: PaneStatus::Running,
        ..make_pane(AgentType::Claude, PaneStatus::Running)
    };
    let idle_pane = PaneInfo {
        pane_id: "%2".into(),
        status: PaneStatus::Idle,
        ..make_pane(AgentType::Claude, PaneStatus::Idle)
    };
    let mut state = make_state(vec![]);
    state.repo_groups = vec![make_repo_group("project", vec![running_pane, idle_pane])];

    // All filter shows both
    state.global.status_filter = StatusFilter::All;
    state.rebuild_row_targets();
    assert_eq!(state.layout.pane_row_targets.len(), 2);

    // Simulates sync_global_state setting filter to Running
    state.global.status_filter = StatusFilter::Busy;
    state.rebuild_row_targets();
    assert_eq!(state.layout.pane_row_targets.len(), 1);
    assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");

    // Simulates sync_global_state setting filter to Idle
    state.global.status_filter = StatusFilter::Idle;
    state.rebuild_row_targets();
    assert_eq!(state.layout.pane_row_targets.len(), 1);
    assert_eq!(state.layout.pane_row_targets[0].pane_id, "%2");
}

#[test]
fn test_cursor_sync_clamped_by_rebuild() {
    use tmux_agent_sidebar::state::StatusFilter;

    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
    let mut state = make_state(vec![]);
    state.repo_groups = vec![make_repo_group("project", vec![pane])];

    // Simulates sync_global_state setting cursor beyond bounds
    state.global.selected_pane_row = 5;
    state.global.status_filter = StatusFilter::All;
    state.rebuild_row_targets();
    // Should be clamped to last valid index
    assert_eq!(state.global.selected_pane_row, 0);
}

// ─── GlobalState tests ──────────────────────────────────────────────

fn make_opts(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn make_global() -> GlobalState {
    GlobalState::new()
}

// ─── apply_all shared-options sync tests ────────────────────────────
//
// apply_all is the only sync path: `load_from_tmux` runs it once at
// startup and the main loop runs it on every refresh tick. The
// `last_saved_*` guards are the only protection a window-activation
// flicker needs: a tick changes a field only when another instance
// wrote a value the guard does not already know about.

#[test]
fn apply_all_ignores_tmux_filter_matching_last_saved() {
    let mut g = make_global();
    g.status_filter = StatusFilter::Busy;

    let opts = make_opts(&[(tmux::SIDEBAR_FILTER, "all")]);
    g.apply_all(&opts);

    assert_eq!(
        g.status_filter,
        StatusFilter::Busy,
        "local filter change should not be overwritten when tmux matches last_saved"
    );
}

#[test]
fn apply_all_applies_filter_from_tmux() {
    let mut g = make_global();

    let opts = make_opts(&[(tmux::SIDEBAR_FILTER, "waiting")]);
    g.apply_all(&opts);

    assert_eq!(g.status_filter, StatusFilter::Waiting);
}

#[test]
fn apply_all_applies_cursor_from_tmux() {
    let mut g = make_global();

    let opts = make_opts(&[(tmux::SIDEBAR_CURSOR, "3")]);
    g.apply_all(&opts);

    assert_eq!(g.selected_pane_row, 3);
}

#[test]
fn apply_all_applies_repo_filter_from_tmux() {
    let mut g = make_global();

    let opts = make_opts(&[(tmux::SIDEBAR_REPO_FILTER, "my-app")]);
    g.apply_all(&opts);

    assert_eq!(g.repo_filter, RepoFilter::Repo("my-app".into()));
}

#[test]
fn apply_all_empty_opts_changes_nothing() {
    let mut g = make_global();
    g.status_filter = StatusFilter::Busy;
    g.repo_filter = RepoFilter::Repo("app".into());
    g.selected_pane_row = 2;

    g.apply_all(&std::collections::HashMap::new());

    assert_eq!(g.status_filter, StatusFilter::Busy);
    assert_eq!(g.repo_filter, RepoFilter::Repo("app".into()));
    assert_eq!(g.selected_pane_row, 2);
}

#[test]
fn apply_all_applies_error_filter_from_tmux() {
    let mut g = make_global();

    let opts = make_opts(&[(tmux::SIDEBAR_FILTER, "error")]);
    g.apply_all(&opts);

    assert_eq!(g.status_filter, StatusFilter::Error);
}

#[test]
fn apply_all_invalid_filter_defaults_to_all() {
    let mut g = make_global();
    g.status_filter = StatusFilter::Busy;

    // "garbage" parses as All, All == last_saved → no change
    let opts = make_opts(&[(tmux::SIDEBAR_FILTER, "garbage")]);
    g.apply_all(&opts);

    assert_eq!(
        g.status_filter,
        StatusFilter::Busy,
        "invalid filter string parsed as All should match last_saved and not overwrite"
    );
}

#[test]
fn apply_all_applies_all_three_from_tmux() {
    let mut g = make_global();

    let opts = make_opts(&[
        (tmux::SIDEBAR_FILTER, "error"),
        (tmux::SIDEBAR_CURSOR, "7"),
        (tmux::SIDEBAR_REPO_FILTER, "my-app"),
    ]);
    g.apply_all(&opts);

    assert_eq!(g.status_filter, StatusFilter::Error);
    assert_eq!(g.selected_pane_row, 7);
    assert_eq!(g.repo_filter, RepoFilter::Repo("my-app".into()));
}

// ─── last_saved guard tests (protects against save failure revert) ───

#[test]
fn apply_all_picks_up_change_from_another_instance() {
    // Simulates: this instance saved "running", another instance later
    // saved "waiting". The next tick should pick up "waiting".
    let mut g = make_global();

    // Startup: this instance starts with default (All)
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_FILTER, "running")]));
    assert_eq!(g.status_filter, StatusFilter::Busy);
    // last_saved_filter is now Running

    // Another instance changes filter to Waiting (writes to tmux)
    // The next tick applies the value the other instance wrote:
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_FILTER, "waiting")]));

    assert_eq!(
        g.status_filter,
        StatusFilter::Waiting,
        "apply_all should pick up a filter changed by another instance"
    );
}

#[test]
fn apply_all_picks_up_cursor_from_another_instance() {
    let mut g = make_global();

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "3")]));
    assert_eq!(g.selected_pane_row, 3);
    // last_saved_cursor is now 3

    // Another instance moves cursor to 7
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "7")]));

    assert_eq!(
        g.selected_pane_row, 7,
        "apply_all should pick up a cursor changed by another instance"
    );
}

#[test]
fn apply_all_does_not_revert_pending_cursor_save() {
    // The user navigates to row 5 but the debounced save has not fired
    // yet, so tmux still holds 2 (= last_saved_cursor). Ticks that run
    // before the flush must not drag the cursor back to 2.
    let mut g = make_global();

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "2")]));
    assert_eq!(g.selected_pane_row, 2);
    g.selected_pane_row = 5;
    g.queue_cursor_save();

    // Ticks run while the save is still pending (tmux value unchanged).
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "2")]));
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "2")]));

    assert_eq!(
        g.selected_pane_row, 5,
        "apply_all must not revert a cursor whose save is pending"
    );
}

#[test]
fn apply_all_does_not_clobber_a_pending_cursor_move_with_a_remote_value() {
    // The user navigates to row 5; before the debounced write lands,
    // another instance writes row 9 to tmux. The pending local move
    // must win — adopting 9 now would both show the wrong row and make
    // the flush persist the peer's value instead of the user's.
    let mut g = make_global();

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "2")]));
    assert_eq!(g.selected_pane_row, 2);
    g.selected_pane_row = 5;
    g.queue_cursor_save();

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_CURSOR, "9")]));

    assert_eq!(
        g.selected_pane_row, 5,
        "a remote cursor value must not clobber a pending local move"
    );
}

#[test]
fn apply_all_picks_up_cursor_and_repo_filter_from_another_instance() {
    // Both instances start from the same synced values; the other one
    // then writes a new cursor and repo filter, and this instance's
    // next tick adopts both in a single apply.
    let mut g = make_global();

    g.apply_all(&make_opts(&[
        (tmux::SIDEBAR_CURSOR, "1"),
        (tmux::SIDEBAR_REPO_FILTER, "app"),
    ]));
    assert_eq!(g.selected_pane_row, 1);
    assert_eq!(g.repo_filter, RepoFilter::Repo("app".into()));

    g.apply_all(&make_opts(&[
        (tmux::SIDEBAR_CURSOR, "6"),
        (tmux::SIDEBAR_REPO_FILTER, "other"),
    ]));

    assert_eq!(
        g.selected_pane_row, 6,
        "one tick should pick up the other instance's cursor"
    );
    assert_eq!(
        g.repo_filter,
        RepoFilter::Repo("other".into()),
        "one tick should pick up the other instance's repo filter"
    );
}

#[test]
fn apply_all_unchanged_tmux_value_leaves_local_change_alone() {
    // The user narrows the repo filter locally but the write has not
    // landed, so tmux keeps "app" — the same value as last_saved. Ticks
    // that keep re-reading it must not clobber the local choice.
    let mut g = make_global();

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]));
    assert_eq!(g.repo_filter, RepoFilter::Repo("app".into()));
    g.repo_filter = RepoFilter::Repo("mine".into());

    g.apply_all(&make_opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]));
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]));
    g.apply_all(&make_opts(&[(tmux::SIDEBAR_REPO_FILTER, "app")]));

    assert_eq!(
        g.repo_filter,
        RepoFilter::Repo("mine".into()),
        "an unchanged tmux value must not overwrite a local change"
    );
}

// ─── Spawn / Remove Popup State ───────────────────────────────────

fn spawn_state_with_repo() -> AppState {
    let mut state = make_state(vec![]);
    state.open_spawn_input_for_repo("myproj".into(), "/home/u/myproj".into(), None);
    state
}

#[test]
fn open_spawn_input_initialises_fields_to_defaults() {
    let state = spawn_state_with_repo();
    match &state.popup {
        PopupState::SpawnInput {
            input,
            target_repo,
            target_repo_root,
            agent_idx,
            mode_idx,
            field,
            ..
        } => {
            assert_eq!(input, "");
            assert_eq!(target_repo, "myproj");
            assert_eq!(target_repo_root, "/home/u/myproj");
            assert_eq!(*agent_idx, 0);
            assert_eq!(*mode_idx, 0);
            assert_eq!(*field, tmux_agent_sidebar::state::SpawnField::Task);
        }
        _ => panic!("expected SpawnInput popup"),
    }
}

#[test]
fn spawn_input_push_char_only_types_into_input_field() {
    let mut state = spawn_state_with_repo();
    state.spawn_input_push_char('h');
    state.spawn_input_push_char('i');
    // Move to agent field — pushing chars there must be a no-op.
    state.spawn_input_next_field();
    state.spawn_input_push_char('x');
    // Back to mode field — still a no-op.
    state.spawn_input_next_field();
    state.spawn_input_push_char('y');
    match &state.popup {
        PopupState::SpawnInput { input, .. } => assert_eq!(input, "hi"),
        _ => panic!(),
    }
}

#[test]
fn spawn_input_pop_char_removes_trailing_char_only_on_input_field() {
    let mut state = spawn_state_with_repo();
    for c in "abc".chars() {
        state.spawn_input_push_char(c);
    }
    state.spawn_input_pop_char();
    match &state.popup {
        PopupState::SpawnInput { input, .. } => assert_eq!(input, "ab"),
        _ => panic!(),
    }

    // On a non-input field, pop is a no-op.
    state.spawn_input_next_field();
    state.spawn_input_pop_char();
    match &state.popup {
        PopupState::SpawnInput { input, .. } => assert_eq!(input, "ab"),
        _ => panic!(),
    }
}

#[test]
fn spawn_input_field_wraps_forward_and_backward() {
    let mut state = spawn_state_with_repo();
    state.spawn_input_next_field();
    state.spawn_input_next_field();
    state.spawn_input_next_field(); // wraps back to Task
    match &state.popup {
        PopupState::SpawnInput { field, .. } => {
            assert_eq!(*field, tmux_agent_sidebar::state::SpawnField::Task)
        }
        _ => panic!(),
    }
    state.spawn_input_prev_field(); // should land on Mode
    match &state.popup {
        PopupState::SpawnInput { field, .. } => {
            assert_eq!(*field, tmux_agent_sidebar::state::SpawnField::Mode)
        }
        _ => panic!(),
    }
}

#[test]
fn spawn_input_cycle_changes_agent_and_resets_mode() {
    let mut state = spawn_state_with_repo();
    state.spawn_input_next_field(); // field = 1 (agent)
    // Cycle agent forward — expect agent_idx to advance.
    state.spawn_input_cycle(1);
    match &state.popup {
        PopupState::SpawnInput {
            agent_idx,
            mode_idx,
            ..
        } => {
            assert_eq!(*agent_idx, 1, "agent should advance");
            assert_eq!(*mode_idx, 0, "mode should reset when agent changes");
        }
        _ => panic!(),
    }
    // Cycle back — wraps to 0.
    state.spawn_input_cycle(-1);
    match &state.popup {
        PopupState::SpawnInput { agent_idx, .. } => assert_eq!(*agent_idx, 0),
        _ => panic!(),
    }
}

#[test]
fn spawn_input_cycle_on_mode_field_increments_mode_only() {
    let mut state = spawn_state_with_repo();
    state.spawn_input_next_field(); // agent
    state.spawn_input_next_field(); // mode
    state.spawn_input_cycle(1);
    match &state.popup {
        PopupState::SpawnInput {
            agent_idx,
            mode_idx,
            ..
        } => {
            assert_eq!(*agent_idx, 0);
            assert_eq!(*mode_idx, 1);
        }
        _ => panic!(),
    }
}

#[test]
fn spawn_input_cycle_on_input_field_is_noop() {
    let mut state = spawn_state_with_repo();
    // field 0 (input) — cycle should not touch agent/mode.
    state.spawn_input_cycle(1);
    state.spawn_input_cycle(-1);
    match &state.popup {
        PopupState::SpawnInput {
            agent_idx,
            mode_idx,
            ..
        } => {
            assert_eq!(*agent_idx, 0);
            assert_eq!(*mode_idx, 0);
        }
        _ => panic!(),
    }
}

#[test]
fn close_spawn_input_resets_popup() {
    let mut state = spawn_state_with_repo();
    state.close_spawn_input();
    assert!(matches!(state.popup, PopupState::None));
    assert!(!state.is_spawn_input_open());
}

#[test]
fn set_flash_and_take_flash_returns_then_clears_after_deadline() {
    let mut state = make_state(vec![]);
    state.set_flash("hello");
    assert_eq!(state.take_flash().as_deref(), Some("hello"));
    // Flash is still valid because expiry is 4s in the future.
    assert_eq!(state.take_flash().as_deref(), Some("hello"));
    // Expire manually and verify take_flash clears it.
    if let Some((_, exp)) = state.flash.as_mut() {
        *exp = std::time::Instant::now() - std::time::Duration::from_secs(1);
    }
    assert_eq!(state.take_flash(), None);
    assert!(state.flash.is_none());
}

#[test]
fn agent_cycle_keeps_mode_in_bounds_for_codex() {
    // Codex has only 3 modes; cycling to codex with a high mode_idx
    // should be safe because agent switch resets mode_idx to 0.
    let mut state = spawn_state_with_repo();
    state.spawn_input_next_field(); // mode moved away from 0? no, field=1 (agent)
    // First cycle past the claude mode list (5 entries) to exercise
    // wrapping, then jump to codex.
    state.spawn_input_next_field(); // field = 2 (mode)
    for _ in 0..worktree::CLAUDE_MODES.len() {
        state.spawn_input_cycle(1);
    }
    // Now go back to agent field and pick codex.
    state.spawn_input_prev_field(); // field = 1
    state.spawn_input_cycle(1); // agent → codex
    match &state.popup {
        PopupState::SpawnInput {
            agent_idx,
            mode_idx,
            ..
        } => {
            assert_eq!(*agent_idx, 1);
            // Mode must have reset to 0 (< codex mode list length).
            assert!(*mode_idx < worktree::CODEX_MODES.len());
        }
        _ => panic!(),
    }
}

#[test]
fn open_remove_confirm_for_unknown_pane_sets_flash_and_keeps_popup_closed() {
    // Without a real tmux environment `display_message` returns an
    // empty string, so the "not spawned" branch should fire, set the
    // flash banner, and leave the popup state untouched.
    let mut state = make_state(vec![]);
    assert!(state.flash.is_none());
    state.open_remove_confirm_for_pane("%nonexistent".into());
    assert!(matches!(state.popup, PopupState::None));
    let flash = state.flash.as_ref().expect("flash must be set");
    assert!(
        flash.0.contains("not spawned"),
        "flash should mention the unspawned pane: {:?}",
        flash.0
    );
}

#[test]
fn handle_mouse_click_routes_spawn_remove_targets_to_open_remove_confirm() {
    // Stuff a synthetic × click target into layout.spawn_remove_targets
    // and verify the click handler routes to
    // `open_remove_confirm_for_pane`. Without a tmux env the call
    // flashes "not spawned", which still proves the routing worked
    // (otherwise flash would stay None).
    use tmux_agent_sidebar::state::SpawnRemoveTarget;
    let mut state = make_state(vec![]);
    state.layout.spawn_remove_targets = vec![SpawnRemoveTarget {
        rect: ratatui::layout::Rect::new(4, 5, 3, 1),
        pane_id: "%42".into(),
    }];
    state.handle_mouse_click(5, 5);
    let flash = state.flash.as_ref().expect("click should have fired");
    assert!(
        flash.0.contains("not spawned"),
        "click on × target should call open_remove_confirm_for_pane: {:?}",
        flash.0
    );
}

// ─── Filter row: render ↔ hit-test agreement ─────────────────────────

#[test]
fn rendered_filter_row_and_hit_test_agree_at_the_20_column_minimum() {
    // Ten panes of each of the four counted statuses put every chip
    // count at two digits, so at the 20 column minimum the chip run no
    // longer fits beside the pop-out control. The renderer drops the
    // error chip whole, and the hit test must agree with what was
    // drawn: every drawn chip's start column selects exactly that
    // chip, the dropped chip's columns do nothing, and the control
    // column toggles instead of filtering.
    let mut panes = Vec::new();
    let mut id = 0;
    for status in [
        PaneStatus::Running,
        PaneStatus::Waiting,
        PaneStatus::Idle,
        PaneStatus::Error,
    ] {
        for _ in 0..10 {
            let mut pane = make_pane(AgentType::Claude, status.clone());
            pane.pane_id = format!("%{id}");
            id += 1;
            panes.push(pane);
        }
    }
    let mut state = make_state(vec![]);
    // A pane id no tmux server can ever have, so the control click's
    // pop-out action probes a pane that does not exist and — an unknown
    // window size meaning "do not act" — stays silent instead of touching
    // a live pane or flashing a bogus refusal.
    state.tmux_pane = "%4294967295".into();
    state.repo_groups = vec![make_repo_group("project", panes)];
    state.rebuild_row_targets();
    state.global.status_filter = StatusFilter::All;

    let output = render_to_string(&mut state, 20, 12);
    let row = output.lines().next().unwrap();

    // The frame snapshot pins the whole render: the two-digit chip run,
    // the dropped error chip (no ✕ anywhere on the row), and the
    // pop-out control on the last column of the 20-column row.
    insta::assert_snapshot!(output, @"
     ≡40 ●10 ◐10 ○10   ⤢
    ╭ Activity│Tasks│… ╮
    │  No activity yet │
    ╰──────────────────╯
    ");

    // The hit test runs on the width the frame actually rendered with.
    assert_eq!(state.layout.filter_bar_width, 20);

    let past = std::time::Instant::now() - std::time::Duration::from_millis(200);
    // The sidebar is the active pane: a filter-bar click only counts then,
    // so that the click which focuses the sidebar cannot select a chip.
    state.sidebar_pane.pane_active = true;
    let click = |state: &mut AppState, col: u16| {
        state.timers.last_filter_click = past;
        state.handle_mouse_click(0, col);
    };

    // Every drawn chip's start column selects exactly that chip.
    for (col, expected) in [
        (1, StatusFilter::All),
        (5, StatusFilter::Busy),
        (9, StatusFilter::Waiting),
        (13, StatusFilter::Idle),
    ] {
        click(&mut state, col);
        assert_eq!(
            state.global.status_filter, expected,
            "click on column {col} must select the drawn chip: {row}"
        );
    }

    // The dropped chip's columns and the gap do nothing.
    for col in [16, 17, 18] {
        click(&mut state, col);
        assert_eq!(
            state.global.status_filter,
            StatusFilter::Idle,
            "click on undrawn column {col} must not change the filter: {row}"
        );
    }

    // The control column toggles the pop-out instead of filtering.
    click(&mut state, 19);
    assert_eq!(
        state.global.status_filter,
        StatusFilter::Idle,
        "a click on the control must never change the filter"
    );
    // The probe for the bogus pane failed, and the pop-out reports the
    // unreadable window size instead of acting — the flash is the proof
    // that the click reached the pop-out (the refusal flash itself is
    // covered by the popout unit tests and the injected toggle test).
    assert!(
        state
            .flash
            .as_ref()
            .is_some_and(|(msg, _)| msg.starts_with("Pop-out unavailable:")),
        "a click on the control must reach the pop-out action"
    );
}

#[test]
fn floating_panel_clicks_hit_the_clamped_bottom_panel_geometry() {
    // While the sidebar floats, the Activity/Git panel is clamped to a
    // third of the panel height (20 → 12 of a 38-row panel). The mouse
    // handlers must hit-test the clamped geometry the frame actually
    // drew: the drawn tab row switches the tab, a drawn agent row in the
    // band the unclamped height wrongly claimed selects that agent, and
    // the wheel over those rows scrolls the agents list.
    let mut panes = Vec::new();
    for i in 0..20 {
        let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
        pane.pane_id = format!("%{i}");
        panes.push(pane);
    }
    let mut state = make_state(vec![]);
    state.repo_groups = vec![make_repo_group("project", panes)];
    state.sidebar_pane.floating = true;
    state.bottom_panel_height = 20;

    const TERM_WIDTH: u16 = 40;
    const TERM_HEIGHT: u16 = 38;
    let output = render_to_string(&mut state, TERM_WIDTH, TERM_HEIGHT);

    // The frame snapshot pins the drawn geometry: the clamped 12-row
    // bottom panel with its tab row, and the agent list running right up
    // to the panel's border.
    insta::assert_snapshot!(output, @"
     ≡20 ●0 ◐0 ○20 ✕0                      ⤡
    ⓘ                                    — ▾
    project
      ○ claude
        Waiting for prompt…
    ┃ ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
      ○ claude
        Waiting for prompt…
    ╭ Activity│Tasks│Git ──────────────────╮
    │            No activity yet           │
    ╰──────────────────────────────────────╯
    ");

    // The frame published the height it drew: 38/3 clamps 20 → 12.
    assert_eq!(state.layout.bottom_panel_height, 12);
    let bottom_start = TERM_HEIGHT - state.layout.bottom_panel_height;
    assert_eq!(bottom_start, 26, "the tab row is drawn at row 26");

    // A click on the drawn tab row (row 26) switches the tab. With the
    // unclamped height the boundary sat at row 18 and this click was
    // swallowed.
    state.bottom_tab = BottomTab::GitStatus;
    assert!(
        state.handle_terminal_click(bottom_start, 3, TERM_HEIGHT),
        "the drawn tab row must be clickable"
    );
    assert_eq!(state.bottom_tab, BottomTab::Activity);

    // A click on a drawn agent row inside the band 18..26 — above the
    // clamped panel, below where the unclamped height put the boundary —
    // selects that agent.
    let line_index = (19 - 2) as usize;
    assert_eq!(
        state.layout.line_to_row[line_index],
        Some(8),
        "row 19 must be a drawn agent row (agent 8)"
    );
    assert!(
        !state.handle_terminal_click(19, 5, TERM_HEIGHT),
        "row 19 is above the clamped panel, so it is not the tab row"
    );
    assert_eq!(
        state.global.selected_pane_row, 8,
        "the click on the drawn agent row must select agent 8"
    );

    // The wheel over the same band scrolls the agents list, and the wheel
    // over the drawn panel scrolls the bottom tab.
    state.scrolls.panes = ScrollState {
        offset: 0,
        total_lines: 40,
        visible_height: 20,
    };
    state.activity.scroll = ScrollState {
        offset: 0,
        total_lines: 30,
        visible_height: 10,
    };
    state.handle_mouse_scroll(24, TERM_HEIGHT, state.layout.bottom_panel_height, 3);
    assert_eq!(
        state.scrolls.panes.offset, 3,
        "row 24 is a drawn agent row: the wheel scrolls the agents list"
    );
    state.handle_mouse_scroll(28, TERM_HEIGHT, state.layout.bottom_panel_height, 3);
    assert_eq!(
        state.activity.scroll.offset, 3,
        "row 28 is inside the drawn panel: the wheel scrolls the panel"
    );
    assert_eq!(
        state.scrolls.panes.offset, 3,
        "the panel wheel must not touch the agents list"
    );
}
