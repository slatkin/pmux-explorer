#[allow(dead_code, unused_imports)]
mod test_helpers;

use pmux_explorer::activity::ActivityEntry;
use pmux_explorer::state::{BottomTab, Focus};
use pmux_explorer::tmux::{AgentType, PaneStatus, SessionInfo, WindowInfo};
use test_helpers::*;

// ─── Styled Snapshot Tests for Selection and Focus ─────────────────

#[test]
fn snapshot_selected_focused_styled() {
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
    state.focus_state.sidebar_focused = true;
    state.global.selected_pane_row = 0;
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the selected row's ┃[fg:153,bg:239] marker
    // and the selection background spanning its content cells.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 10), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]W[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]g[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]…[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn snapshot_activity_focused_styled() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.activity.entries = vec![ActivityEntry {
        timestamp: "10:32".into(),
        tool: "Edit".into(),
        label: "src/main.rs".into(),
    }];

    // Styled snapshot locks in the focused group header accent (fg:153) and
    // the active-panel border color.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 14), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    1[fg:#2ac3de,bg:#24283b]0[fg:#2ac3de,bg:#24283b]:[fg:#2ac3de,bg:#24283b]3[fg:#2ac3de,bg:#24283b]2[fg:#2ac3de,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]E[fg:180,bg:#24283b]d[fg:180,bg:#24283b]i[fg:180,bg:#24283b]t[fg:180,bg:#24283b]
     [fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]m[fg:#a9b1d6,bg:#24283b]a[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b].[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn snapshot_activity_unfocused_styled() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    state.focus_state.focus = Focus::Panes; // not activity
    state.focus_state.sidebar_focused = true;
    state.activity.entries = vec![ActivityEntry {
        timestamp: "10:32".into(),
        tool: "Edit".into(),
        label: "src/main.rs".into(),
    }];

    // Styled snapshot locks in the unfocused bottom-panel rule
    // (border_inactive fg:240).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 14), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    1[fg:#2ac3de,bg:#1a1b26]0[fg:#2ac3de,bg:#1a1b26]:[fg:#2ac3de,bg:#1a1b26]3[fg:#2ac3de,bg:#1a1b26]2[fg:#2ac3de,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]E[fg:180,bg:#1a1b26]d[fg:180,bg:#1a1b26]i[fg:180,bg:#1a1b26]t[fg:180,bg:#1a1b26]
     [fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26]r[fg:#a9b1d6,bg:#1a1b26]c[fg:#a9b1d6,bg:#1a1b26]/[fg:#a9b1d6,bg:#1a1b26]m[fg:#a9b1d6,bg:#1a1b26]a[fg:#a9b1d6,bg:#1a1b26]i[fg:#a9b1d6,bg:#1a1b26]n[fg:#a9b1d6,bg:#1a1b26].[fg:#a9b1d6,bg:#1a1b26]r[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn bottom_tab_activity_uses_accent_when_selected() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.global.bottom_tab = BottomTab::Activity;

    // Styled snapshot locks in `A` using accent (fg:153) and `G` remaining
    // muted (fg:252) on the bottom-panel tab title row.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 14), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]N[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]y[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn bottom_tab_git_uses_accent_when_selected() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.global.bottom_tab = BottomTab::GitStatus;

    // Styled snapshot locks in `G` using accent (fg:153) and `A` remaining
    // muted (fg:252) on the bottom-panel tab title row.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 14), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868]T[fg:#a9b1d6,bg:#414868]o[fg:#a9b1d6,bg:#414868]o[fg:#a9b1d6,bg:#414868]l[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#7aa2f7,bg:#414868]i[fg:#7aa2f7,bg:#414868]t[fg:#7aa2f7,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]W[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]k[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b]g[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]a[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

// ─── Selection Background Border Tests ───────────────────────────────

#[test]
fn selection_marker_uses_accent_color_with_selection_bg() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    state.focus_state.sidebar_focused = true;
    state.focus_state.focus = Focus::Panes;
    state.global.selected_pane_row = 0;

    // Styled snapshot locks in:
    //   1. the selected row begins with `┃[fg:153,bg:239]` (accent + selection bg)
    //   2. the selected row never contains the old frame `│`
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 24), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn selection_bg_covers_inner_padding() {
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
    state.focus_state.sidebar_focused = true;
    state.focus_state.focus = Focus::Panes;
    state.global.selected_pane_row = 0;

    // Styled snapshot locks in the selection background extending across the
    // inner padding immediately after the `┃` marker (` [bg:239]`).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 24), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]W[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]g[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]…[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn selection_bg_shows_while_unfocused() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
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
    // The selection shows at all times — sidebar focus no longer gates
    // it — so the selected row carries the selection background even
    // while the sidebar is not the active tmux pane.
    state.focus_state.sidebar_focused = false;
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 24), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]o[fg:#7aa2f7,bg:#414868]l[fg:#7aa2f7,bg:#414868]s[fg:#7aa2f7,bg:#414868]│[fg:240,bg:#414868]T[fg:#a9b1d6,bg:#414868]a[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]k[fg:#a9b1d6,bg:#414868]s[fg:#a9b1d6,bg:#414868]│[fg:240,bg:#414868]G[fg:#a9b1d6,bg:#414868]i[fg:#a9b1d6,bg:#414868]t[fg:#a9b1d6,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

// ─── Custom Theme Tests ─────────────────────────────────────────────

#[test]
fn snapshot_custom_theme_colors() {
    use pmux_explorer::ui::colors::ColorTheme;
    use ratatui::style::Color;

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

    // Override theme with custom colors
    state.theme = ColorTheme {
        accent: Color::Indexed(196),       // red accent
        agent_claude: Color::Indexed(226), // yellow agent
        status_idle: Color::Indexed(46),   // green idle
        port: Color::Indexed(39),          // cyan port
        ..ColorTheme::default()
    };
    // Unfocus sidebar so selected row doesn't use REVERSED (which hides colors)
    state.focus_state.sidebar_focused = false;
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the custom theme colors (accent fg:196,
    // agent_claude fg:226, status_idle fg:46).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 10), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:196,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:46,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:196,bg:#24283b]r[fg:196,bg:#24283b]o[fg:196,bg:#24283b]j[fg:196,bg:#24283b]e[fg:196,bg:#24283b]c[fg:196,bg:#24283b]t[fg:196,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:196,bg:#414868] [bg:#414868]󰒲[fg:46,bg:#414868] [fg:226,bg:#414868][fg:226,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]W[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]g[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]…[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_theme_default_matches_shell_colors() {
    use pmux_explorer::ui::colors::ColorTheme;
    use ratatui::style::Color;

    let theme = ColorTheme::default();

    // Verify defaults match shell version's pmux-explorer.conf
    assert_eq!(theme.accent, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.border_inactive, Color::Indexed(240));
    assert_eq!(theme.status_running, Color::Rgb(0x9e, 0xce, 0x6a));
    assert_eq!(theme.status_waiting, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.status_idle, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.status_error, Color::Rgb(0xf7, 0x76, 0x8e));
    assert_eq!(theme.agent_claude, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.agent_codex, Color::Rgb(0xbb, 0x9a, 0xf7));
    assert_eq!(theme.text_active, Color::Rgb(0xc0, 0xca, 0xf5));
    assert_eq!(theme.text_muted, Color::Rgb(0xa9, 0xb1, 0xd6));
    assert_eq!(theme.session_header, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.wait_reason, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.wait_reason_action, Color::Rgb(0xf7, 0x76, 0x8e));
}
