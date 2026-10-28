#[allow(dead_code, unused_imports)]
mod test_helpers;

use pmux_explorer::state::Focus;
use pmux_explorer::tmux::{AgentType, PaneStatus, SessionInfo, WindowInfo};
use pmux_explorer::ui::colors::ColorTheme;
use pmux_explorer::ui::icons::StatusIcons;
use test_helpers::*;

// ─── Agents: auto-scroll behavior Tests ─────────────────────────────

#[test]
fn test_agents_auto_scroll_keeps_selected_visible() {
    // Create enough agents to overflow a small viewport
    let mut panes = Vec::new();
    for i in 0..10 {
        let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
        pane.pane_id = format!("%{}", i);
        panes.push(pane);
    }

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: panes.clone(),
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", panes)];
    state.focus_state.sidebar_focused = true;
    state.focus_state.focus = Focus::Panes;
    state.rebuild_row_targets();

    // Render with a small height. With the 2-row header, the first pane
    // still stays visible without needing to scroll.
    let _ = render_to_string(&mut state, 28, 26);
    assert_eq!(state.scrolls.panes.offset, 0, "initially at top");

    // Select last agent and re-render
    state.global.selected_pane_row = 9;
    let _ = render_to_string(&mut state, 28, 26);
    assert!(
        state.scrolls.panes.offset > 0,
        "should scroll down to show selected agent"
    );
}

#[test]
fn test_panes_scroll_offset_tracks_total_and_visible() {
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

    let _ = render_to_string(&mut state, 28, 26);
    // After rendering, panes_scroll.total_lines and panes_scroll.visible_height should be set
    assert!(
        state.scrolls.panes.total_lines > 0,
        "total lines should be populated"
    );
    assert!(
        state.scrolls.panes.visible_height > 0,
        "visible height should be populated"
    );
}

// ─── Agents: Codex agent color ──────────────────────────────────────

#[test]
fn snapshot_codex_agent_styled() {
    let theme = ColorTheme::default();
    assert_eq!(
        theme.agent_color(&AgentType::Codex),
        ratatui::style::Color::Rgb(0xbb, 0x9a, 0xf7)
    );
}

// ─── Agents: running icon variants via render ───────────────────────

#[test]
fn test_running_icon_blink_off() {
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
    state.focus_state.sidebar_focused = false;
    state.spinner_frame = 0;

    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 1  1 󰥔 0 󰒲 0  0      ⤢
    ⓘ                        — ▾
    project
    ┃  
      Tools│Tasks│Git
            No tools yet
    ");
}

#[test]
fn test_running_spinner_frame_advances() {
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
    state.focus_state.sidebar_focused = false;
    state.spinner_frame = 3;

    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 1  1 󰥔 0 󰒲 0  0      ⤢
    ⓘ                        — ▾
    project
    ┃  
      Tools│Tasks│Git
            No tools yet
    ");
}

#[test]
fn test_waiting_icon() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Waiting);
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
    state.focus_state.sidebar_focused = false;

    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 1  0 󰥔 1 󰒲 0  0      ⤢
    ⓘ                        — ▾
    project
    ┃ 󰥔 
      Tools│Tasks│Git
            No tools yet
    ");
}

#[test]
fn test_error_icon() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Error);
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
    state.focus_state.sidebar_focused = false;

    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 1  0 󰥔 0 󰒲 0  1      ⤢
    ⓘ                        — ▾
    project
    ┃  
      Tools│Tasks│Git
            No tools yet
    ");
}

#[test]
fn test_unknown_status_icon() {
    let icons = StatusIcons::default();
    assert_eq!(icons.status_icon(&PaneStatus::Unknown), "·");
}

// ─── Agents: auto-scroll keeps selected pane visible ───────────────

#[test]
fn test_agents_auto_scroll_shows_last_selected_pane() {
    // When the last agent in a group is selected, the auto-scroll
    // should bring it into view (the selection marker must be visible).
    // Nine panes make a 19-line list inside the 16-row list viewport
    // (30% of the 26-row frame leaves 18 agent rows), so the selection
    // really needs a scroll.
    let mut panes = Vec::new();
    for i in 0..9 {
        let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
        pane.pane_id = format!("%{}", i);
        panes.push(pane);
    }

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: panes.clone(),
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", panes)];
    state.focus_state.sidebar_focused = true;
    state.focus_state.focus = Focus::Panes;
    state.rebuild_row_targets();

    // Select the last agent
    state.global.selected_pane_row = 8;
    let _ = render_to_string(&mut state, 28, 26);

    // Auto-scroll should have moved forward to keep the last-selected pane visible.
    assert!(
        state.scrolls.panes.offset > 0,
        "selecting the last agent should scroll the list"
    );
}

#[test]
fn test_agents_auto_scroll_up_shows_group_header() {
    // After scrolling down, selecting the first agent should scroll
    // back up enough to show the group header. At 25 frame rows the
    // 30% split leaves the list viewport 2 lines short of the 17-line
    // total (15 vs 17), so the up-scroll branch `first < offset` can
    // fire; at 26 it is 1 short and the branch misses by equality.
    let mut panes = Vec::new();
    for i in 0..8 {
        let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
        pane.pane_id = format!("%{}", i);
        panes.push(pane);
    }

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "project".into(),
            window_active: true,
            auto_rename: false,
            panes: panes.clone(),
        }],
    }]);
    state.repo_groups = vec![make_repo_group("project", panes)];
    state.focus_state.sidebar_focused = true;
    state.focus_state.focus = Focus::Panes;
    state.rebuild_row_targets();

    // Scroll to bottom
    state.global.selected_pane_row = 7;
    let _ = render_to_string(&mut state, 28, 25);
    assert!(state.scrolls.panes.offset > 0, "should have scrolled down");

    // Now select first agent and re-render
    state.global.selected_pane_row = 0;
    // The snapshot locks in that the `project` repo header is visible after
    // scrolling back up to the first agent.
    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 8  0 󰥔 0 󰒲 8  0      ⤢
    ⓘ                        — ▾
    project
      󰒲 
        Waiting for prompt…
    ┃ 󰒲 
        Waiting for prompt…
      󰒲 
        Waiting for prompt…
      󰒲 
        Waiting for prompt…
      󰒲 
        Waiting for prompt…
      󰒲 
        Waiting for prompt…
      󰒲 
        Waiting for prompt…
      Tools│Tasks│Git
            No tools yet
    ");
}

// ─── Repo popup rendering ───────────────────────────────────────────

#[test]
fn repo_popup_renders_repo_names_when_open() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "frontend".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![
        make_repo_group("frontend", vec![pane.clone()]),
        make_repo_group("backend", vec![pane.clone()]),
    ];
    state.rebuild_row_targets();
    state.popup = pmux_explorer::state::PopupState::Repo {
        selected: 0,
        area: None,
    };

    // The snapshot locks in that the popup lists the `All` entry plus both
    // repo names when opened.
    insta::assert_snapshot!(render_to_string(&mut state, 40, 30), @"
     󱓯 2  0 󰥔 0 󰒲 2  0                  ⤢
    ⓘ                                    — ▾
    frontend                    ┌──────────┐
    ┃ 󰒲                        │ All      │
        Waiting for prompt…     │ frontend │
                                │ backend  │
    backend                     └──────────┘
    ┃ 󰒲 
        Waiting for prompt…
      Tools│Tasks│Git
                  No tools yet
    ");
    // The popup area is required for click hit-testing and is non-visual
    // state, so it stays as a direct assertion.
    assert!(
        state.repo_popup_area().is_some(),
        "render should populate repo popup area for hit-testing"
    );
}

#[test]
fn repo_popup_highlights_selected_entry_with_background() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@1".into(),
            window_name: "frontend".into(),
            window_active: true,
            auto_rename: false,
            panes: vec![pane.clone()],
        }],
    }]);
    state.repo_groups = vec![
        make_repo_group("frontend", vec![pane.clone()]),
        make_repo_group("backend", vec![pane.clone()]),
    ];
    state.rebuild_row_targets();
    state.focus_state.sidebar_focused = false; // surface raw colors instead of REVERSED
    state.popup = pmux_explorer::state::PopupState::Repo {
        selected: 2, // "backend" (0=All, 1=frontend, 2=backend)
        area: None,
    };

    // Styled snapshot locks in that the `backend` row carries the selection
    // background (bg:239) on each cell of the entry.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 40, 30), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]2[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]2[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#c0caf5,bg:#24283b] [bg:#24283b]▾[fg:#c0caf5,bg:#24283b]
    f[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]n[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]n[fg:#7aa2f7,bg:#24283b]d[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]┌[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]┐[fg:#7aa2f7]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]│[fg:#7aa2f7] [fg:#c0caf5]A[fg:#c0caf5]l[fg:#c0caf5]l[fg:#c0caf5] [fg:#c0caf5] [fg:#c0caf5] [fg:#c0caf5] [fg:#c0caf5] [fg:#c0caf5] [fg:#c0caf5]│[fg:#7aa2f7]
     [bg:#24283b] [bg:#24283b] [fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]W[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]g[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]…[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]│[fg:#7aa2f7] [fg:#a9b1d6]f[fg:#a9b1d6]r[fg:#a9b1d6]o[fg:#a9b1d6]n[fg:#a9b1d6]t[fg:#a9b1d6]e[fg:#a9b1d6]n[fg:#a9b1d6]d[fg:#a9b1d6] [fg:#a9b1d6]│[fg:#7aa2f7]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]│[fg:#7aa2f7] [fg:#c0caf5,bg:#414868]b[fg:#c0caf5,bg:#414868]a[fg:#c0caf5,bg:#414868]c[fg:#c0caf5,bg:#414868]k[fg:#c0caf5,bg:#414868]e[fg:#c0caf5,bg:#414868]n[fg:#c0caf5,bg:#414868]d[fg:#c0caf5,bg:#414868] [fg:#c0caf5,bg:#414868] [fg:#c0caf5,bg:#414868]│[fg:#7aa2f7]
    b[fg:#7aa2f7,bg:#24283b]a[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]k[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]n[fg:#7aa2f7,bg:#24283b]d[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]└[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]─[fg:#7aa2f7]┘[fg:#7aa2f7]
    ┃[fg:#7aa2f7,bg:#24283b] [bg:#24283b]󰒲[fg:#7dcfff,bg:#24283b] [fg:#ff9e64,bg:#24283b][fg:#ff9e64,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]W[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]g[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]…[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868]T[fg:#7aa2f7,bg:#414868,bold]o[fg:#7aa2f7,bg:#414868,bold]o[fg:#7aa2f7,bg:#414868,bold]l[fg:#7aa2f7,bg:#414868,bold]s[fg:#7aa2f7,bg:#414868,bold]│[fg:240,bg:#414868]T[fg:#c0caf5,bg:#414868]a[fg:#c0caf5,bg:#414868]s[fg:#c0caf5,bg:#414868]k[fg:#c0caf5,bg:#414868]s[fg:#c0caf5,bg:#414868]│[fg:240,bg:#414868]G[fg:#c0caf5,bg:#414868]i[fg:#c0caf5,bg:#414868]t[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}
