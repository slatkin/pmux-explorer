//! Focus fill rendering: the base + active background fills painted over
//! the two sidebar sections while the sidebar pane is the active tmux
//! pane (`focus-indication` spec).
//!
//! Geometry used throughout: a 30×20 backend with the default 30% bottom
//! panel gives `split_rows(30, 20) == 6`, so rows 0–12 are the top
//! section and rows 13–19 the bottom panel.

#[allow(dead_code, unused_imports)]
mod test_helpers;

use pmux_explorer::group::{PaneGitInfo, RepoGroup};
use pmux_explorer::state::{AppState, Focus, PopupState};
use pmux_explorer::tmux::{AgentType, PaneStatus, SessionInfo, WindowInfo};
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use test_helpers::*;

const TOP_FILL: Color = Color::Indexed(236); // focus_active_bg default
const BASE_FILL: Color = Color::Indexed(234); // focus_bg default
const SELECTION: Color = Color::Indexed(239); // selection_bg default

/// Bottom panel row count for the standard 30×20 / 30% geometry.
const BOT_H: u16 = 6;

fn focused_panes_state() -> AppState {
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
    state
}

/// Same as [`focused_panes_state`] but the pane reports a git branch,
/// so the pane draws a branch row under its status row.
fn focused_panes_state_with_branch() -> AppState {
    let mut state = focused_panes_state();
    let pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    state.repo_groups = vec![RepoGroup {
        name: "project".into(),
        has_focus: true,
        panes: vec![(
            pane,
            PaneGitInfo {
                repo_root: Some("/home/user/project".into()),
                branch: Some("main".into()),
                is_worktree: false,
                worktree_name: None,
            },
        )],
    }];
    state.rebuild_row_targets();
    state
}

/// Every bg colour present in a row, ascending and deduplicated.
fn row_bgs(buf: &Buffer, y: u16) -> Vec<Color> {
    let width = buf.area.width;
    let mut bgs: Vec<Color> = ((0..width).map(|x| buf[(x, y)].bg)).collect();
    bgs.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    bgs.dedup();
    bgs
}

// ─── Focused + Panes: top active, bottom base ───────────────────────

#[test]
fn focus_fill_panes_top_active_bottom_base() {
    let mut state = focused_panes_state();
    // Default focus is Panes.
    assert_eq!(state.focus_state.focus, Focus::Panes);

    let buf = render_to_buffer(&mut state, 30, 20);

    // The filter bar row (row 0) carries the active fill.
    assert_eq!(buf[(0, 0)].bg, TOP_FILL);
    // The bottom panel's rule row and last row carry the base fill.
    assert_eq!(buf[(0, 20 - BOT_H)].bg, BASE_FILL);
    assert_eq!(buf[(0, 19)].bg, BASE_FILL);
    // No section carries the other section's fill.
    assert!(
        !(0..20 - BOT_H).any(|y| row_bgs(&buf, y).contains(&BASE_FILL)),
        "no top-section cell may carry the base fill"
    );
    assert!(
        !(20 - BOT_H..20).any(|y| row_bgs(&buf, y).contains(&TOP_FILL)),
        "no bottom-panel cell may carry the active fill"
    );

    // The fill covers blank space below the last pane row: the blank
    // row just above the bottom panel carries the active fill.
    assert_eq!(buf[(0, 20 - BOT_H - 1)].bg, TOP_FILL);

    insta::assert_snapshot!(render_to_styled_string(&mut state, 30, 20), @"
     [bg:236]≡[fg:111,bg:236]1[fg:153,bg:236] [bg:236]●[fg:114,bg:236]0[fg:245,bg:236] [bg:236]◐[fg:245,bg:236]0[fg:245,bg:236] [bg:236]○[fg:245,bg:236]1[fg:255,bg:236] [bg:236]✕[fg:245,bg:236]0[fg:245,bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]⤢[fg:114,bg:236] [bg:236]
    ⓘ[fg:221,bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]—[fg:252,bg:236] [bg:236]▾[fg:252,bg:236]
    p[fg:153,bg:236]r[fg:153,bg:236]o[fg:153,bg:236]j[fg:153,bg:236]e[fg:153,bg:236]c[fg:153,bg:236]t[fg:153,bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
    ┃[fg:153,bg:239] [bg:239]○[fg:110,bg:239] [fg:174,bg:239]c[fg:174,bg:239]l[fg:174,bg:239]a[fg:174,bg:239]u[fg:174,bg:239]d[fg:174,bg:239]e[fg:174,bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239] [bg:239]
     [bg:236] [bg:236] [fg:255,bg:236] [fg:255,bg:236]W[fg:255,bg:236]a[fg:255,bg:236]i[fg:255,bg:236]t[fg:255,bg:236]i[fg:255,bg:236]n[fg:255,bg:236]g[fg:255,bg:236] [fg:255,bg:236]f[fg:255,bg:236]o[fg:255,bg:236]r[fg:255,bg:236] [fg:255,bg:236]p[fg:255,bg:236]r[fg:255,bg:236]o[fg:255,bg:236]m[fg:255,bg:236]p[fg:255,bg:236]t[fg:255,bg:236]…[fg:255,bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
    ─[fg:240,bg:234] [fg:240,bg:234]T[fg:153,bg:234]o[fg:153,bg:234]o[fg:153,bg:234]l[fg:153,bg:234]s[fg:153,bg:234]│[fg:240,bg:234]T[fg:252,bg:234]a[fg:252,bg:234]s[fg:252,bg:234]k[fg:252,bg:234]s[fg:252,bg:234]│[fg:240,bg:234]G[fg:252,bg:234]i[fg:252,bg:234]t[fg:252,bg:234] [fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]─[fg:240,bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]N[fg:252,bg:234]o[fg:252,bg:234] [fg:252,bg:234]t[fg:252,bg:234]o[fg:252,bg:234]o[fg:252,bg:234]l[fg:252,bg:234]s[fg:252,bg:234] [fg:252,bg:234]y[fg:252,bg:234]e[fg:252,bg:234]t[fg:252,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
    ");
}

// ─── Focused + ActivityLog: bottom active, top base ─────────────────

#[test]
fn focus_fill_activity_log_bottom_active_top_base() {
    let mut state = focused_panes_state();
    state.focus_state.focus = Focus::ActivityLog;

    let buf = render_to_buffer(&mut state, 30, 20);

    // The bottom panel's rule row carries the active fill; the top
    // section's filter bar row carries the base fill.
    assert_eq!(buf[(0, 20 - BOT_H)].bg, TOP_FILL);
    assert_eq!(buf[(0, 0)].bg, BASE_FILL);
    // No section carries the other section's fill.
    assert!(
        !(0..20 - BOT_H).any(|y| row_bgs(&buf, y).contains(&TOP_FILL)),
        "no top-section cell may carry the active fill"
    );
    assert!(
        !(20 - BOT_H..20).any(|y| row_bgs(&buf, y).contains(&BASE_FILL)),
        "no bottom-panel cell may carry the base fill"
    );

    insta::assert_snapshot!(render_to_styled_string(&mut state, 30, 20), @"
     [bg:234]≡[fg:111,bg:234]1[fg:153,bg:234] [bg:234]●[fg:114,bg:234]0[fg:245,bg:234] [bg:234]◐[fg:245,bg:234]0[fg:245,bg:234] [bg:234]○[fg:245,bg:234]1[fg:255,bg:234] [bg:234]✕[fg:245,bg:234]0[fg:245,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]⤢[fg:114,bg:234] [bg:234]
    ⓘ[fg:221,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]—[fg:252,bg:234] [bg:234]▾[fg:252,bg:234]
    p[fg:153,bg:234]r[fg:153,bg:234]o[fg:153,bg:234]j[fg:153,bg:234]e[fg:153,bg:234]c[fg:153,bg:234]t[fg:153,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
    ┃[fg:153,bg:234] [bg:234]○[fg:110,bg:234] [fg:174,bg:234]c[fg:174,bg:234]l[fg:174,bg:234]a[fg:174,bg:234]u[fg:174,bg:234]d[fg:174,bg:234]e[fg:174,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [fg:255,bg:234] [fg:255,bg:234]W[fg:255,bg:234]a[fg:255,bg:234]i[fg:255,bg:234]t[fg:255,bg:234]i[fg:255,bg:234]n[fg:255,bg:234]g[fg:255,bg:234] [fg:255,bg:234]f[fg:255,bg:234]o[fg:255,bg:234]r[fg:255,bg:234] [fg:255,bg:234]p[fg:255,bg:234]r[fg:255,bg:234]o[fg:255,bg:234]m[fg:255,bg:234]p[fg:255,bg:234]t[fg:255,bg:234]…[fg:255,bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
     [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234] [bg:234]
    ─[fg:153,bg:236] [fg:153,bg:236]T[fg:153,bg:236]o[fg:153,bg:236]o[fg:153,bg:236]l[fg:153,bg:236]s[fg:153,bg:236]│[fg:240,bg:236]T[fg:252,bg:236]a[fg:252,bg:236]s[fg:252,bg:236]k[fg:252,bg:236]s[fg:252,bg:236]│[fg:240,bg:236]G[fg:252,bg:236]i[fg:252,bg:236]t[fg:252,bg:236] [fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]─[fg:153,bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]N[fg:252,bg:236]o[fg:252,bg:236] [fg:252,bg:236]t[fg:252,bg:236]o[fg:252,bg:236]o[fg:252,bg:236]l[fg:252,bg:236]s[fg:252,bg:236] [fg:252,bg:236]y[fg:252,bg:236]e[fg:252,bg:236]t[fg:252,bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
     [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236] [bg:236]
    ");
}

// ─── Unfocused: no fill anywhere ─────────────────────────────────────

#[test]
fn focus_fill_unfocused_paints_nothing() {
    let mut state = focused_panes_state();
    state.focus_state.sidebar_focused = false;

    let buf = render_to_buffer(&mut state, 30, 20);

    assert!(
        !(0..20).any(|y| {
            row_bgs(&buf, y)
                .iter()
                .any(|bg| matches!(bg, Color::Indexed(234) | Color::Indexed(236)))
        }),
        "no cell may carry a focus fill when the sidebar is unfocused"
    );

    insta::assert_snapshot!(render_to_string(&mut state, 30, 20), @"
     ≡1 ●0 ◐0 ○1 ✕0             ⤢
    ⓘ                          — ▾
    project
    ┃ ○ claude
        Waiting for prompt…
    ─ Tools│Tasks│Git ────────────
             No tools yet
    ");
}

// ─── Hidden bottom panel: top is active regardless of focus ─────────

#[test]
fn focus_fill_bottom_hidden_top_gets_active_fill() {
    let mut state = focused_panes_state();
    // ActivityLog focus with the panel hidden: the top is still the
    // only place input can land, so it takes the active fill (D2).
    state.focus_state.focus = Focus::ActivityLog;
    state.global.bottom_percent = 0;

    let buf = render_to_buffer(&mut state, 30, 20);

    assert_eq!(buf[(0, 0)].bg, TOP_FILL);
    assert_eq!(buf[(0, 19)].bg, TOP_FILL);
    assert!(
        !(0..20).any(|y| row_bgs(&buf, y).contains(&BASE_FILL)),
        "a hidden bottom panel must leave no base fill on the frame"
    );

    insta::assert_snapshot!(render_to_string(&mut state, 30, 20), @"
     ≡1 ●0 ◐0 ○1 ✕0             ⤢
    ⓘ                          — ▾
    project
    ┃ ○ claude
        Waiting for prompt…
    ");
}

// ─── Selection overrides the fill; popup/flash punch through ────────

#[test]
fn focus_fill_selected_row_keeps_selection_bg() {
    let mut state = focused_panes_state_with_branch();

    let buf = render_to_buffer(&mut state, 30, 20);

    // Row 3 is the status row, row 4 the branch row: both marker rows
    // of the selected pane keep selection_bg over the active fill. The
    // repo header row above them shows the fill.
    assert_eq!(buf[(0, 3)].bg, SELECTION);
    assert_eq!(buf[(0, 4)].bg, SELECTION);
    assert_eq!(buf[(0, 2)].bg, TOP_FILL);
}

#[test]
fn focus_fill_popup_and_flash_show_no_fill() {
    let mut state = focused_panes_state();
    state.popup = PopupState::RemoveConfirm {
        pane_id: "%42".into(),
        branch: "add-login".into(),
        error: None,
        area: None,
    };
    state.set_flash("spawn: test flash");

    let buf = render_to_buffer(&mut state, 30, 20);

    let popup_rect = match &state.popup {
        PopupState::RemoveConfirm { area, .. } => area.expect("popup records its rect"),
        _ => unreachable!(),
    };
    for y in popup_rect.y..popup_rect.y + popup_rect.height {
        for x in popup_rect.x..popup_rect.x + popup_rect.width {
            let bg = buf[(x, y)].bg;
            assert!(
                !matches!(bg, Color::Indexed(234) | Color::Indexed(236)),
                "popup cell ({x},{y}) must not carry a focus fill, got {bg:?}"
            );
        }
    }

    // The flash banner renders on the last row of the top section with
    // Clear, so its cells show the terminal default, not the fill.
    let flash_y = 20 - BOT_H - 1;
    for x in 0..30 {
        let bg = buf[(x, flash_y)].bg;
        assert!(
            !matches!(bg, Color::Indexed(234) | Color::Indexed(236)),
            "flash cell ({x},{flash_y}) must not carry a focus fill, got {bg:?}"
        );
    }
}

// ─── Pet band carries the top fill ───────────────────────────────────

#[test]
fn focus_fill_pet_band_carries_top_fill() {
    let mut state = focused_panes_state();
    state.pet_enabled = true;

    let buf = render_to_buffer(&mut state, 30, 20);

    // The pet band is the bottom PET_SCENE_HEIGHT rows of the top
    // section. Blank cells there must carry the top fill; chair cells
    // keep their own explicit bg (Indexed 94), which patches over it.
    let band_top = 20 - BOT_H - 5;
    let mut blank_filled = 0;
    for y in band_top..20 - BOT_H {
        for x in 0..30 {
            let cell = &buf[(x, y)];
            if cell.symbol() == " " {
                assert_eq!(
                    cell.bg, TOP_FILL,
                    "blank pet-band cell ({x},{y}) must carry the top fill"
                );
                blank_filled += 1;
            }
        }
    }
    assert!(blank_filled > 0, "the pet band must have blank cells");

    insta::assert_snapshot!(render_to_string(&mut state, 30, 20), @"
     ≡1 ●0 ◐0 ○1 ✕0             ⤢
    ⓘ                          — ▾
    project
    ┃ ○ claude
        Waiting for prompt…
      ▄ ▄
     ▄▀▀▀▄                   ████
      ▀ ▀                 ██ █  █
    ─ Tools│Tasks│Git ────────────
             No tools yet
    ");
}
