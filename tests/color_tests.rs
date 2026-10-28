#[allow(dead_code, unused_imports)]
mod test_helpers;

use pmux_explorer::activity::{ActivityEntry, TaskProgress, TaskStatus};
use pmux_explorer::state::{BottomTab, Focus};
use pmux_explorer::tmux::{AgentType, PaneStatus, PermissionMode, SessionInfo, WindowInfo};
use pmux_explorer::ui::colors::ColorTheme;
use ratatui::style::Color;
use test_helpers::*;

// ─── ColorTheme Default Values ──────────────────────────────────────

#[test]
fn test_all_color_theme_defaults() {
    let theme = ColorTheme::default();

    // Core UI colors
    assert_eq!(theme.accent, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.border_inactive, Color::Indexed(240));
    assert_eq!(theme.selection_bg, Color::Rgb(0x41, 0x48, 0x68));
    assert_eq!(theme.focus_bg, Color::Rgb(0x1a, 0x1b, 0x26));
    assert_eq!(theme.focus_active_bg, Color::Rgb(0x24, 0x28, 0x3b));

    // Status colors
    assert_eq!(theme.status_all, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.status_running, Color::Rgb(0x9e, 0xce, 0x6a));
    assert_eq!(theme.status_waiting, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.status_idle, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.status_error, Color::Rgb(0xf7, 0x76, 0x8e));
    assert_eq!(theme.status_unknown, Color::Rgb(0x56, 0x5f, 0x89));

    // Agent colors
    assert_eq!(theme.agent_claude, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.agent_codex, Color::Rgb(0xbb, 0x9a, 0xf7));
    assert_eq!(theme.pet_body, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.pet_eye, Color::Rgb(0x9e, 0xce, 0x6a));

    // Text colors
    assert_eq!(theme.text_active, Color::Rgb(0xc0, 0xca, 0xf5));
    assert_eq!(theme.text_muted, Color::Rgb(0xa9, 0xb1, 0xd6));
    assert_eq!(theme.text_inactive, Color::Rgb(0x56, 0x5f, 0x89));

    // Header/UI element colors
    assert_eq!(theme.session_header, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.port, Color::Rgb(0x9a, 0xa5, 0xce));
    assert_eq!(theme.wait_reason, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.wait_reason_action, Color::Rgb(0xf7, 0x76, 0x8e));
    assert_eq!(theme.branch, Color::Rgb(0x73, 0xda, 0xcb));

    // New theme fields
    assert_eq!(theme.badge_danger, Color::Rgb(0xf7, 0x76, 0x8e));
    assert_eq!(theme.badge_auto, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.task_progress, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.subagent, Color::Rgb(0x73, 0xda, 0xcb));
    assert_eq!(theme.commit_hash, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.diff_added, Color::Rgb(0x9e, 0xce, 0x6a));
    assert_eq!(theme.diff_deleted, Color::Rgb(0xf7, 0x76, 0x8e));
    assert_eq!(theme.file_change, Color::Rgb(0xe0, 0xaf, 0x68));
    assert_eq!(theme.pr_link, Color::Rgb(0x7d, 0xcf, 0xff));
    assert_eq!(theme.focus_border, Color::Rgb(0xE6, 0x98, 0x75));
}

// ─── status_color() for all PaneStatus variants ─────────────────────

#[test]
fn test_status_color_all_variants() {
    let theme = ColorTheme::default();

    assert_eq!(
        theme.status_color(&PaneStatus::Running, false),
        Color::Rgb(0x9e, 0xce, 0x6a)
    );
    assert_eq!(
        theme.status_color(&PaneStatus::Waiting, false),
        Color::Rgb(0xe0, 0xaf, 0x68)
    );
    assert_eq!(
        theme.status_color(&PaneStatus::Idle, false),
        Color::Rgb(0x7d, 0xcf, 0xff)
    );
    assert_eq!(
        theme.status_color(&PaneStatus::Error, false),
        Color::Rgb(0xf7, 0x76, 0x8e)
    );
    assert_eq!(
        theme.status_color(&PaneStatus::Unknown, false),
        Color::Rgb(0x56, 0x5f, 0x89)
    );
}

#[test]
fn test_status_color_attention_overrides_all() {
    let theme = ColorTheme::default();

    // attention=true should always return status_waiting regardless of status
    for status in &[
        PaneStatus::Running,
        PaneStatus::Waiting,
        PaneStatus::Idle,
        PaneStatus::Error,
        PaneStatus::Unknown,
    ] {
        assert_eq!(
            theme.status_color(status, true),
            theme.status_waiting,
            "attention=true should override {:?} to waiting color",
            status
        );
    }
}

// ─── agent_color() for all AgentType variants ───────────────────────

#[test]
fn test_agent_color_all_variants() {
    let theme = ColorTheme::default();

    assert_eq!(
        theme.agent_color(&AgentType::Claude),
        Color::Rgb(0xff, 0x9e, 0x64)
    );
    assert_eq!(
        theme.agent_color(&AgentType::Codex),
        Color::Rgb(0xbb, 0x9a, 0xf7)
    );
}

// ─── PermissionMode badge colors ────────────────────────────────────

#[test]
fn test_permission_mode_bypass_all_renders_danger_color() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Running);
    pane.permission_mode = PermissionMode::BypassPermissions;

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

    // Styled snapshot locks in the BypassAll `!` badge rendered with
    // badge_danger (fg:167).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 26), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [fg:#f7768e,bg:#414868]![fg:#f7768e,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn test_permission_mode_full_auto_renders_auto_color() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Running);
    pane.permission_mode = PermissionMode::Auto;

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

    // Styled snapshot locks in the Auto `auto` badge rendered with
    // badge_auto (fg:221).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 26), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [fg:#e0af68,bg:#414868]a[fg:#e0af68,bg:#414868]u[fg:#e0af68,bg:#414868]t[fg:#e0af68,bg:#414868]o[fg:#e0af68,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn test_permission_mode_normal_no_badge() {
    let pane = make_pane(AgentType::Claude, PaneStatus::Running);
    // permission_mode is Normal by default in make_pane

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

    // Snapshot locks in that the agent row shows no badge in Normal mode.
    insta::assert_snapshot!(render_to_string(&mut state, 28, 25), @"
     󱓯 1  1 󰥔 0 󰒲 0  0    ⇄ ⤢
    ⓘ                        — ▾
    project
    ┃  
              │  │ 󰊢
            No tools yet
    ");
}

// ─── Activity tool_color_index all branches ─────────────────────────

#[test]
fn test_tool_color_all_branches() {
    let cases = vec![
        ("Edit", 180),
        ("Write", 180),
        ("Bash", 114),
        ("Read", 110),
        ("Glob", 110),
        ("Grep", 110),
        ("Agent", 181),
        ("UnknownTool", 244),
        ("", 244),
    ];
    for (tool, expected) in cases {
        let entry = ActivityEntry {
            timestamp: "10:00".into(),
            tool: tool.into(),
            label: "test".into(),
        };
        assert_eq!(
            entry.tool_color_index(),
            expected,
            "tool_color_index for '{}'",
            tool
        );
    }
}

// ─── Git status summary colors ──────────────────────────────────────

#[test]
fn test_git_summary_modified_uses_badge_auto_color() {
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

    state.global.bottom_tab = BottomTab::GitStatus;
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.git.branch = "main".into();
    state.git.unstaged_files = vec![pmux_explorer::git::GitFileEntry {
        status: 'M',
        name: "src/lib.rs".into(),
        additions: 5,
        deletions: 2,
        path: String::new(),
    }];

    // Styled snapshot locks in the Modified file badge color
    // (badge_auto fg:221).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#7aa2f7,bg:#414868,bold] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    m[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]1[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]f[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b]
    ─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]
    U[fg:#2ac3de,bg:#24283b]n[fg:#2ac3de,bg:#24283b]s[fg:#2ac3de,bg:#24283b]t[fg:#2ac3de,bg:#24283b]a[fg:#2ac3de,bg:#24283b]g[fg:#2ac3de,bg:#24283b]e[fg:#2ac3de,bg:#24283b]d[fg:#2ac3de,bg:#24283b] [fg:#2ac3de,bg:#24283b]([fg:#2ac3de,bg:#24283b]1[fg:#2ac3de,bg:#24283b])[fg:#2ac3de,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    M[fg:#e0af68,bg:#24283b] [bg:#24283b]s[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]b[fg:#a9b1d6,bg:#24283b].[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]+[fg:#9ece6a,bg:#24283b]5[fg:#9ece6a,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]-[fg:#f7768e,bg:#24283b]2[fg:#f7768e,bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

// ─── Render Tests: verify correct colors in styled output ───────────

#[test]
fn test_task_progress_line_uses_task_progress_color() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Running);
    pane.pane_id = "%1".into();

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

    // Set task progress for pane %1
    let progress = TaskProgress {
        tasks: vec![
            ("Task A".into(), TaskStatus::Completed),
            ("Task B".into(), TaskStatus::InProgress),
            ("Task C".into(), TaskStatus::Pending),
        ],
    };
    state.set_pane_task_progress("%1", Some(progress));

    // Styled snapshot locks in both the task_progress color (fg:223) and
    // the progress glyphs (✔/◼/◻) with the "1/3" count.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 40, 40), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [fg:#ff9e64,bg:#24283b] [fg:#ff9e64,bg:#24283b]✔[fg:#ff9e64,bg:#24283b]◼[fg:#ff9e64,bg:#24283b]◻[fg:#ff9e64,bg:#24283b] [fg:#ff9e64,bg:#24283b]1[fg:#ff9e64,bg:#24283b]/[fg:#ff9e64,bg:#24283b]3[fg:#ff9e64,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
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
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn test_subagent_line_uses_subagent_color() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Running);
    pane.subagents = vec!["Explore #1".into()];

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

    // Styled snapshot locks in the subagent line color (fg:73) plus the
    // rendered "Explore #1" label.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 40, 27), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]└[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]E[fg:#73dacb,bg:#24283b]x[fg:#73dacb,bg:#24283b]p[fg:#73dacb,bg:#24283b]l[fg:#73dacb,bg:#24283b]o[fg:#73dacb,bg:#24283b]r[fg:#73dacb,bg:#24283b]e[fg:#73dacb,bg:#24283b] [fg:#73dacb,bg:#24283b]#[fg:#73dacb,bg:#24283b]1[fg:#73dacb,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn test_response_arrow_uses_response_arrow_color() {
    let mut pane = make_pane(AgentType::Claude, PaneStatus::Idle);
    pane.pane_active = false;
    pane.prompt = "Task completed successfully".into();
    pane.prompt_is_response = true;

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

    // Styled snapshot locks in:
    //   • response_arrow color (fg:81) + bold on the response glyph
    //   • text_active color (fg:255) on the focused response text
    insta::assert_snapshot!(render_to_styled_string(&mut state, 40, 27), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b]󰭹[fg:#7dcfff,bg:#24283b,bold] [fg:#7dcfff,bg:#24283b,bold]T[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]k[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]c[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]m[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]l[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]d[fg:#c0caf5,bg:#24283b] [fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]u[fg:#c0caf5,bg:#24283b]c[fg:#c0caf5,bg:#24283b]c[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]u[fg:#c0caf5,bg:#24283b]l[fg:#c0caf5,bg:#24283b]l[fg:#c0caf5,bg:#24283b]y[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

// test_commit_hash_uses_commit_hash_color was removed because
// git_last_commit and commit hash rendering no longer exist.

#[test]
fn test_pr_link_uses_pr_link_color() {
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

    state.global.bottom_tab = BottomTab::GitStatus;
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.git.branch = "feature/test".into();
    state.git.pr_number = Some("99".into());
    state.git.remote_url = "https://github.com/user/repo".into();

    // Styled snapshot locks in the PR link: pr_link color (fg:117) plus
    // the underline modifier on the `#99` glyphs.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 40), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#7aa2f7,bg:#414868,bold] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    f[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]u[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]/[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]t[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]#[fg:#7dcfff,bg:#24283b,underline]9[fg:#7dcfff,bg:#24283b,underline]9[fg:#7dcfff,bg:#24283b,underline]
    ─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]W[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]k[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b]g[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]a[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_diff_stat_added_uses_diff_added_color() {
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

    state.global.bottom_tab = BottomTab::GitStatus;
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.git.branch = "main".into();
    state.git.diff_stat = Some((42, 10));

    // Styled snapshot locks in the `+42` additions rendered with
    // diff_added color (fg:114).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 40), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#7aa2f7,bg:#414868,bold] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    m[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    +[fg:#9ece6a,bg:#24283b]4[fg:#9ece6a,bg:#24283b]2[fg:#9ece6a,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]-[fg:#f7768e,bg:#24283b]1[fg:#f7768e,bg:#24283b]0[fg:#f7768e,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]0[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]f[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b]
    ─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]W[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]k[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b]g[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]a[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_diff_stat_deleted_uses_diff_deleted_color() {
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

    state.global.bottom_tab = BottomTab::GitStatus;
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.git.branch = "main".into();
    state.git.diff_stat = Some((0, 25));

    // Styled snapshot locks in the `-25` deletions rendered with
    // diff_deleted color (fg:174).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#7aa2f7,bg:#414868,bold] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    m[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    +[fg:#9ece6a,bg:#24283b]0[fg:#9ece6a,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]-[fg:#f7768e,bg:#24283b]2[fg:#f7768e,bg:#24283b]5[fg:#f7768e,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]0[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]f[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b]
    ─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]W[fg:#a9b1d6,bg:#24283b]o[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]k[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b]g[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]t[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]c[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]a[fg:#a9b1d6,bg:#24283b]n[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_file_change_stat_uses_file_change_color() {
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

    state.global.bottom_tab = BottomTab::GitStatus;
    state.focus_state.focus = Focus::ActivityLog;
    state.focus_state.sidebar_focused = true;
    state.git.branch = "main".into();
    state.git.unstaged_files = vec![pmux_explorer::git::GitFileEntry {
        status: 'M',
        name: "lib.rs".into(),
        additions: 40,
        deletions: 10,
        path: String::new(),
    }];

    // Styled snapshot locks in the `M lib.rs` row rendered with
    // badge_auto (fg:221) on the status glyph.
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]—[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26]▾[fg:#a9b1d6,bg:#1a1b26]
    p[fg:#7aa2f7,bg:#1a1b26]r[fg:#7aa2f7,bg:#1a1b26]o[fg:#7aa2f7,bg:#1a1b26]j[fg:#7aa2f7,bg:#1a1b26]e[fg:#7aa2f7,bg:#1a1b26]c[fg:#7aa2f7,bg:#1a1b26]t[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ┃[fg:#7aa2f7,bg:#1a1b26] [bg:#1a1b26][fg:82,bg:#1a1b26] [fg:#ff9e64,bg:#1a1b26][fg:#ff9e64,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#7aa2f7,bg:#414868,bold] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    m[fg:#c0caf5,bg:#24283b]a[fg:#c0caf5,bg:#24283b]i[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]1[fg:#a9b1d6,bg:#24283b] [fg:#a9b1d6,bg:#24283b]f[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]l[fg:#a9b1d6,bg:#24283b]e[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b]
    ─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]─[fg:240,bg:#24283b]
    U[fg:#2ac3de,bg:#24283b]n[fg:#2ac3de,bg:#24283b]s[fg:#2ac3de,bg:#24283b]t[fg:#2ac3de,bg:#24283b]a[fg:#2ac3de,bg:#24283b]g[fg:#2ac3de,bg:#24283b]e[fg:#2ac3de,bg:#24283b]d[fg:#2ac3de,bg:#24283b] [fg:#2ac3de,bg:#24283b]([fg:#2ac3de,bg:#24283b]1[fg:#2ac3de,bg:#24283b])[fg:#2ac3de,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    M[fg:#e0af68,bg:#24283b] [bg:#24283b]l[fg:#a9b1d6,bg:#24283b]i[fg:#a9b1d6,bg:#24283b]b[fg:#a9b1d6,bg:#24283b].[fg:#a9b1d6,bg:#24283b]r[fg:#a9b1d6,bg:#24283b]s[fg:#a9b1d6,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]+[fg:#9ece6a,bg:#24283b]4[fg:#9ece6a,bg:#24283b]0[fg:#9ece6a,bg:#24283b]/[fg:#a9b1d6,bg:#24283b]-[fg:#f7768e,bg:#24283b]1[fg:#f7768e,bg:#24283b]0[fg:#f7768e,bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

// ─── Custom theme overrides for new fields ──────────────────────────

#[test]
fn test_custom_theme_new_fields_override() {
    let theme = ColorTheme {
        badge_danger: Color::Indexed(196),
        badge_auto: Color::Indexed(226),
        task_progress: Color::Indexed(200),
        subagent: Color::Indexed(100),
        commit_hash: Color::Indexed(150),
        diff_added: Color::Indexed(46),
        diff_deleted: Color::Indexed(160),
        file_change: Color::Indexed(208),
        pr_link: Color::Indexed(33),
        port: Color::Indexed(82),
        ..ColorTheme::default()
    };

    assert_eq!(theme.badge_danger, Color::Indexed(196));
    assert_eq!(theme.badge_auto, Color::Indexed(226));
    assert_eq!(theme.task_progress, Color::Indexed(200));
    assert_eq!(theme.subagent, Color::Indexed(100));
    assert_eq!(theme.commit_hash, Color::Indexed(150));
    assert_eq!(theme.diff_added, Color::Indexed(46));
    assert_eq!(theme.diff_deleted, Color::Indexed(160));
    assert_eq!(theme.file_change, Color::Indexed(208));
    assert_eq!(theme.pr_link, Color::Indexed(33));
    assert_eq!(theme.port, Color::Indexed(82));

    // Original fields should still be default
    assert_eq!(theme.accent, Color::Rgb(0x7a, 0xa2, 0xf7));
    assert_eq!(theme.agent_claude, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.pet_body, Color::Rgb(0xff, 0x9e, 0x64));
    assert_eq!(theme.pet_eye, Color::Rgb(0x9e, 0xce, 0x6a));
}

// ─── Branch color in styled output ──────────────────────────────────

#[test]
fn test_branch_color_in_agent_panel() {
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
    state.repo_groups = vec![pmux_explorer::group::RepoGroup {
        name: "project".into(),
        has_focus: true,
        panes: vec![(
            pane,
            pmux_explorer::group::PaneGitInfo {
                repo_root: Some("/home/user/project".into()),
                branch: Some("feature/cool-feature".into()),
                is_worktree: false,
                worktree_name: None,
            },
        )],
    }];
    state.rebuild_row_targets();
    state.focus_state.sidebar_focused = false;
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the branch name rendered with branch color
    // (fg:109).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 40, 26), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]+[fg:#7aa2f7,bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868] [fg:#73dacb,bg:#414868] [fg:#73dacb,bg:#414868][fg:#73dacb,bg:#414868] [fg:#73dacb,bg:#414868]f[fg:#73dacb,bg:#414868]e[fg:#73dacb,bg:#414868]a[fg:#73dacb,bg:#414868]t[fg:#73dacb,bg:#414868]u[fg:#73dacb,bg:#414868]r[fg:#73dacb,bg:#414868]e[fg:#73dacb,bg:#414868]/[fg:#73dacb,bg:#414868]c[fg:#73dacb,bg:#414868]o[fg:#73dacb,bg:#414868]o[fg:#73dacb,bg:#414868]l[fg:#73dacb,bg:#414868]-[fg:#73dacb,bg:#414868]f[fg:#73dacb,bg:#414868]e[fg:#73dacb,bg:#414868]a[fg:#73dacb,bg:#414868]t[fg:#73dacb,bg:#414868]u[fg:#73dacb,bg:#414868]r[fg:#73dacb,bg:#414868]e[fg:#73dacb,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

// ─── Selection background color ─────────────────────────────────────

#[test]
fn test_selection_bg_color_applied() {
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

    // Styled snapshot locks in the selected agent row's selection
    // background (bg:239).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
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
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

// ─── Accent (focused) vs border_inactive colors ─────────────────────

#[test]
fn test_accent_vs_border_inactive_colors() {
    let mut pane1 = make_pane(AgentType::Claude, PaneStatus::Running);
    pane1.pane_id = "%1".into();
    let mut pane2 = make_pane(AgentType::Codex, PaneStatus::Idle);
    pane2.pane_id = "%2".into();

    let mut state = make_state(vec![SessionInfo {
        session_name: "main".into(),
        windows: vec![WindowInfo {
            window_id: "@0".into(),
            window_name: "fish".into(),
            window_active: true,
            auto_rename: true,
            panes: vec![pane1.clone(), pane2.clone()],
        }],
    }]);
    state.repo_groups = vec![
        pmux_explorer::group::RepoGroup {
            name: "focused-repo".into(),
            has_focus: true,
            panes: vec![(pane1, pmux_explorer::group::PaneGitInfo::default())],
        },
        pmux_explorer::group::RepoGroup {
            name: "unfocused-repo".into(),
            has_focus: false,
            panes: vec![(pane2, pmux_explorer::group::PaneGitInfo::default())],
        },
    ];
    state.focus_state.focused_pane_id = Some("%1".into());
    state.rebuild_row_targets();

    // Styled snapshot locks in:
    //   • focused group header rendered with accent (fg:153)
    //   • unfocused group header rendered with border_inactive (fg:240)
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 30), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]2[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    f[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]u[fg:#7aa2f7,bg:#24283b]s[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]d[fg:#7aa2f7,bg:#24283b]-[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]p[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:82,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    u[fg:#c0caf5,bg:#24283b]n[fg:#c0caf5,bg:#24283b]f[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b]c[fg:#c0caf5,bg:#24283b]u[fg:#c0caf5,bg:#24283b]s[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]d[fg:#c0caf5,bg:#24283b]-[fg:#c0caf5,bg:#24283b]r[fg:#c0caf5,bg:#24283b]e[fg:#c0caf5,bg:#24283b]p[fg:#c0caf5,bg:#24283b]o[fg:#c0caf5,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b]󰒲[fg:#7dcfff,bg:#24283b] [fg:#bb9af7,bg:#24283b]c[fg:#bb9af7,bg:#24283b]o[fg:#bb9af7,bg:#24283b]d[fg:#bb9af7,bg:#24283b]e[fg:#bb9af7,bg:#24283b]x[fg:#bb9af7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [fg:#565f89,bg:#24283b] [fg:#565f89,bg:#24283b]W[fg:#565f89,bg:#24283b]a[fg:#565f89,bg:#24283b]i[fg:#565f89,bg:#24283b]t[fg:#565f89,bg:#24283b]i[fg:#565f89,bg:#24283b]n[fg:#565f89,bg:#24283b]g[fg:#565f89,bg:#24283b] [fg:#565f89,bg:#24283b]f[fg:#565f89,bg:#24283b]o[fg:#565f89,bg:#24283b]r[fg:#565f89,bg:#24283b] [fg:#565f89,bg:#24283b]p[fg:#565f89,bg:#24283b]r[fg:#565f89,bg:#24283b]o[fg:#565f89,bg:#24283b]m[fg:#565f89,bg:#24283b]p[fg:#565f89,bg:#24283b]t[fg:#565f89,bg:#24283b]…[fg:#565f89,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
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
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

// ─── Status color rendering for each PaneStatus ─────────────────────

#[test]
fn test_running_status_color_in_output() {
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
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the running spinner using SPINNER_PULSE[0]
    // color (fg:82).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9ece6a,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_waiting_status_color_in_output() {
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
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the waiting status using status_waiting
    // color (fg:221).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#e0af68,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868]󰥔[fg:#e0af68,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_error_status_color_in_output() {
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
    state.global.bottom_percent = 0;

    // Styled snapshot locks in the error status using status_error
    // color (fg:167).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868][fg:#f7768e,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
    ⓘ[fg:#e0af68,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]—[fg:#a9b1d6,bg:#24283b] [bg:#24283b]▾[fg:#a9b1d6,bg:#24283b]
    p[fg:#7aa2f7,bg:#24283b]r[fg:#7aa2f7,bg:#24283b]o[fg:#7aa2f7,bg:#24283b]j[fg:#7aa2f7,bg:#24283b]e[fg:#7aa2f7,bg:#24283b]c[fg:#7aa2f7,bg:#24283b]t[fg:#7aa2f7,bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ┃[fg:#7aa2f7,bg:#414868] [bg:#414868][fg:#f7768e,bg:#414868] [fg:#ff9e64,bg:#414868][fg:#ff9e64,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
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
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
     [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b] [bg:#24283b]
    ");
}

#[test]
fn test_idle_status_color_in_output() {
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
    state.focus_state.sidebar_focused = false;

    // Styled snapshot locks in the idle status using status_idle
    // color (fg:110).
    insta::assert_snapshot!(render_to_styled_string(&mut state, 28, 25), @"
     [bg:#414868]󱓯[fg:#7aa2f7,bg:#414868] [bg:#414868]1[fg:#7aa2f7,bg:#414868,bold] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰥔[fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868]󰒲[fg:#7dcfff,bg:#414868] [bg:#414868]1[fg:#c0caf5,bg:#414868] [bg:#414868][fg:#9aa5ce,bg:#414868] [bg:#414868]0[fg:#9aa5ce,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]⇄[fg:#9ece6a,bg:#414868] [bg:#414868]⤢[fg:#9ece6a,bg:#414868] [bg:#414868]
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
     [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868][fg:#7aa2f7,bg:#414868,bold] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868][fg:#c0caf5,bg:#414868] [fg:#9aa5ce,bg:#414868]│[fg:#9aa5ce,bg:#414868] [fg:#9aa5ce,bg:#414868]󰊢[fg:#c0caf5,bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868] [bg:#414868]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]N[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]o[fg:#a9b1d6,bg:#1a1b26]l[fg:#a9b1d6,bg:#1a1b26]s[fg:#a9b1d6,bg:#1a1b26] [fg:#a9b1d6,bg:#1a1b26]y[fg:#a9b1d6,bg:#1a1b26]e[fg:#a9b1d6,bg:#1a1b26]t[fg:#a9b1d6,bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
     [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26] [bg:#1a1b26]
    ");
}

#[test]
fn test_unknown_status_color_in_output() {
    let theme = pmux_explorer::ui::colors::ColorTheme::default();
    assert_eq!(
        theme.status_color(&PaneStatus::Unknown, false),
        ratatui::style::Color::Rgb(0x56, 0x5f, 0x89)
    );
}

// ─── Shell-command token colors (@sidebar_color_cmd_*) ──────────────

#[test]
fn test_cmd_color_options_override_their_fields() {
    use pmux_explorer::tmux::{
        SIDEBAR_COLOR_CMD_COMMAND, SIDEBAR_COLOR_CMD_COMMENT, SIDEBAR_COLOR_CMD_FLAG,
        SIDEBAR_COLOR_CMD_OPERATOR, SIDEBAR_COLOR_CMD_STRING, SIDEBAR_COLOR_CMD_VARIABLE,
    };

    let mut options = std::collections::HashMap::new();
    options.insert(SIDEBAR_COLOR_CMD_COMMAND.to_string(), "#112233".to_string());
    options.insert(SIDEBAR_COLOR_CMD_FLAG.to_string(), "10".to_string());
    options.insert(SIDEBAR_COLOR_CMD_STRING.to_string(), "aabbcc".to_string());
    options.insert(
        SIDEBAR_COLOR_CMD_VARIABLE.to_string(),
        "#445566".to_string(),
    );
    options.insert(SIDEBAR_COLOR_CMD_OPERATOR.to_string(), "77".to_string());
    options.insert(SIDEBAR_COLOR_CMD_COMMENT.to_string(), "#8899aa".to_string());

    let theme = ColorTheme::from_options(&options);

    assert_eq!(theme.cmd_command, Color::Rgb(0x11, 0x22, 0x33));
    assert_eq!(theme.cmd_flag, Color::Indexed(10));
    assert_eq!(theme.cmd_string, Color::Rgb(0xaa, 0xbb, 0xcc));
    assert_eq!(theme.cmd_variable, Color::Rgb(0x44, 0x55, 0x66));
    assert_eq!(theme.cmd_operator, Color::Indexed(77));
    assert_eq!(theme.cmd_comment, Color::Rgb(0x88, 0x99, 0xaa));
}

#[test]
fn test_invalid_cmd_color_options_keep_the_default() {
    use pmux_explorer::tmux::{
        SIDEBAR_COLOR_CMD_COMMAND, SIDEBAR_COLOR_CMD_COMMENT, SIDEBAR_COLOR_CMD_FLAG,
        SIDEBAR_COLOR_CMD_OPERATOR, SIDEBAR_COLOR_CMD_STRING, SIDEBAR_COLOR_CMD_VARIABLE,
    };

    let mut options = std::collections::HashMap::new();
    options.insert(
        SIDEBAR_COLOR_CMD_COMMAND.to_string(),
        "not-a-color".to_string(),
    );
    options.insert(SIDEBAR_COLOR_CMD_FLAG.to_string(), "#12".to_string());
    options.insert(SIDEBAR_COLOR_CMD_STRING.to_string(), "256".to_string());
    options.insert(SIDEBAR_COLOR_CMD_VARIABLE.to_string(), "".to_string());
    options.insert(
        SIDEBAR_COLOR_CMD_OPERATOR.to_string(),
        "#gg0011".to_string(),
    );
    options.insert(SIDEBAR_COLOR_CMD_COMMENT.to_string(), "-1".to_string());

    let theme = ColorTheme::from_options(&options);
    let default = ColorTheme::default();

    assert_eq!(theme.cmd_command, default.cmd_command);
    assert_eq!(theme.cmd_flag, default.cmd_flag);
    assert_eq!(theme.cmd_string, default.cmd_string);
    assert_eq!(theme.cmd_variable, default.cmd_variable);
    assert_eq!(theme.cmd_operator, default.cmd_operator);
    assert_eq!(theme.cmd_comment, default.cmd_comment);
}
