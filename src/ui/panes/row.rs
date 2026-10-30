use ratatui::{style::Style, text::Line};

use crate::tmux::PaneStatus;
use crate::ui::colors::ColorTheme;
use crate::ui::icons::StatusIcons;

mod body;
mod branch;
mod ctx;
mod status;

use body::{
    background_hint_row, idle_hint_row, prompt_rows, subagent_rows, task_progress_row,
    wait_reason_row,
};
use branch::branch_ports_row;
use ctx::{RowCtx, SELECTION_MARKER};
#[cfg(test)]
use status::running_icon_for;
use status::status_row;

pub(super) use branch::sidebar_remove_marker_col;

/// Blank columns added before the content of a nested subagent's rows.
pub(super) const NEST_INDENT: usize = 2;

/// The column-layout entry point — and the test-facing wrapper the
/// row tests call: production callers go through
/// [`render_pane_lines_with_repo_tag`] so the strip's inline repo tag
/// has a way in, and this one always passes `None`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn render_pane_lines_with_ports(
    pane: &crate::tmux::PaneInfo,
    git_info: &crate::group::PaneGitInfo,
    ports: Option<&[u16]>,
    task_progress: Option<&crate::activity::TaskProgress>,
    selected: bool,
    active: bool,
    width: usize,
    icons: &StatusIcons,
    theme: &ColorTheme,
    spinner_frame: usize,
    now: u64,
) -> Vec<Line<'static>> {
    render_pane_lines(
        pane,
        git_info,
        ports,
        task_progress,
        selected,
        active,
        width,
        icons,
        theme,
        spinner_frame,
        now,
        false,
        false,
    )
}

/// [`render_pane_lines_with_ports`] plus the strip's branch lift:
/// `inline_branch` hands a non-empty branch label to the status row,
/// so a strip's agent list needs no dedicated branch line.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_pane_lines_inline_branch(
    pane: &crate::tmux::PaneInfo,
    git_info: &crate::group::PaneGitInfo,
    ports: Option<&[u16]>,
    task_progress: Option<&crate::activity::TaskProgress>,
    selected: bool,
    active: bool,
    width: usize,
    icons: &StatusIcons,
    theme: &ColorTheme,
    spinner_frame: usize,
    now: u64,
    inline_branch: bool,
    nested: bool,
) -> Vec<Line<'static>> {
    render_pane_lines(
        pane,
        git_info,
        ports,
        task_progress,
        selected,
        active,
        width,
        icons,
        theme,
        spinner_frame,
        now,
        inline_branch,
        nested,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_pane_lines(
    pane: &crate::tmux::PaneInfo,
    git_info: &crate::group::PaneGitInfo,
    ports: Option<&[u16]>,
    task_progress: Option<&crate::activity::TaskProgress>,
    selected: bool,
    active: bool,
    width: usize,
    icons: &StatusIcons,
    theme: &ColorTheme,
    spinner_frame: usize,
    now: u64,
    inline_branch: bool,
    nested: bool,
) -> Vec<Line<'static>> {
    // A strip lifts the branch label onto the status row; the branch
    // row below then only renders for ports or the remove marker.
    let lifted_branch = inline_branch
        .then(|| crate::ui::text::branch_label(git_info))
        .filter(|label| !label.is_empty());
    // The same strip-vs-column split caps the prompt preview: a strip
    // keeps the last-conversation marker to one truncated line, a
    // column keeps the three-line wrap.
    let prompt_max_lines = if inline_branch { 1 } else { 3 };
    // A nested subagent indents every row it draws by the same amount, so
    // the width left for its content shrinks with the indent.
    let indent = if nested { NEST_INDENT } else { 0 };
    let inner_width = width.saturating_sub(2 + indent);
    let bg = if selected {
        Some(theme.selection_bg)
    } else {
        None
    };
    let apply_bg = |style: Style| match bg {
        Some(c) => style.bg(c),
        None => style,
    };
    // The left marker `┃` highlights the pane that is currently focused in
    // tmux (`active`). To keep the active accent compact, it only appears on
    // the status row and the branch/ports row (when present) — never on
    // deeper details like task progress or prompt wrapping. The sidebar
    // cursor position (`selected`) still paints the full pane with the
    // selection background.
    let marker_ctx = RowCtx {
        marker_char: if active { SELECTION_MARKER } else { " " },
        marker_style: if active {
            apply_bg(Style::default().fg(theme.accent))
        } else {
            apply_bg(Style::default())
        },
        inner_width,
        theme,
        bg,
        active,
        indent,
        inline_branch: lifted_branch,
        prompt_max_lines,
    };
    let plain_ctx = RowCtx {
        marker_char: " ",
        marker_style: Style::default(),
        inner_width,
        theme,
        bg: None,
        active,
        indent,
        inline_branch: None,
        prompt_max_lines,
    };

    let mut out: Vec<Line<'static>> = Vec::with_capacity(8);
    out.push(status_row(pane, &marker_ctx, icons, spinner_frame, now));
    if let Some(line) = branch_ports_row(git_info, ports, pane.sidebar_spawned, &marker_ctx) {
        out.push(line);
    }
    let ctx = &plain_ctx;
    if let Some(line) = task_progress_row(task_progress, ctx) {
        out.push(line);
    }
    out.extend(subagent_rows(&pane.subagents, ctx));
    if let Some(line) = wait_reason_row(&pane.wait_reason, &pane.status, ctx, spinner_frame) {
        out.push(line);
    }
    if let Some(cmd) = pane.bg_shell_cmd.as_deref() {
        out.push(background_hint_row(ctx, cmd));
    }
    // The parent link is positional: a subagent is drawn indented under
    // its parent by the row collector. The prompt/response preview still
    // renders, per the pi-agent-support spec.
    if !pane.prompt.is_empty() {
        out.extend(prompt_rows(pane, ctx));
    } else if matches!(pane.status, PaneStatus::Idle) {
        out.push(idle_hint_row(ctx));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::body::RESPONSE_ARROW_PREFIX;
    use super::*;
    use crate::group::PaneGitInfo;
    use crate::pmux_task::PmuxTaskKind;
    use crate::tmux::{
        AgentType, PaneInfo, PermissionMode, SubagentInfo, SubagentParent, WorktreeMetadata,
    };
    use crate::ui::icons::StatusIcons;
    use crate::ui::text::display_width;
    use ratatui::style::{Color, Modifier};

    fn pane(permission_mode: PermissionMode, status: PaneStatus, prompt: &str) -> PaneInfo {
        pane_with_response(permission_mode, status, prompt, false)
    }

    fn pane_with_response(
        permission_mode: PermissionMode,
        status: PaneStatus,
        prompt: &str,
        is_response: bool,
    ) -> PaneInfo {
        PaneInfo {
            pane_id: "%1".into(),
            pane_active: false,
            status,
            attention: false,
            agent: AgentType::Codex,
            path: "/tmp/project".into(),
            current_command: String::new(),
            prompt: prompt.into(),
            prompt_is_response: is_response,
            started_at: None,
            wait_reason: String::new(),
            permission_mode,
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

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn test_ctx<'a>(theme: &'a ColorTheme, inner_width: usize, active: bool) -> RowCtx<'a> {
        RowCtx {
            marker_char: " ",
            marker_style: Style::default(),
            indent: 0,
            inner_width,
            theme,
            bg: None,
            active,
            inline_branch: None,
            prompt_max_lines: 3,
        }
    }

    /// Render one pane with the nesting flag the sidebar passes for a
    /// subagent that sits under its parent.
    fn render_nested(pane: &PaneInfo, width: usize, nested: bool) -> Vec<Line<'static>> {
        let theme = ColorTheme::default();
        render_pane_lines(
            pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            width,
            &StatusIcons::default(),
            &theme,
            0,
            0,
            false,
            nested,
        )
    }

    #[test]
    fn render_pane_lines_shows_permission_badge() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Auto, PaneStatus::Running, "");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        let status = line_text(&lines[0]);
        assert!(status.contains(" codex auto"));
    }

    #[test]
    fn render_pane_lines_shows_defer_badge() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Defer, PaneStatus::Running, "");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        let status = line_text(&lines[0]);
        assert!(
            status.contains(" codex defer"),
            "defer permission mode should render its badge, got: {status}"
        );
    }

    #[test]
    fn render_pane_lines_truncates_long_pane_name_to_keep_elapsed_visible() {
        // Regression: a pane name can be arbitrarily long and would push
        // the elapsed counter off-screen if we did not truncate it first.
        // The width budget reserves room for the status icon, the agent
        // label, and the elapsed label.
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Running, "");
        p.pane_name = Some("this-is-a-ridiculously-long-pane-name-that-will-not-fit".into());
        // started_at must be > 0 for elapsed_label to render.
        // started_at=1, now=66 → elapsed=65s → "1m5s".
        p.started_at = Some(1);
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            30,
            &StatusIcons::default(),
            &theme,
            0,
            66,
        );

        let status = line_text(&lines[0]);
        // The elapsed counter stays fully visible, the agent label leads
        // the row untruncated, and the pane name is cut with an ellipsis.
        insta::assert_snapshot!(status, @"   codex this-is-a-ridi… 1m5s");
        // Each rendered cell should fit inside the 30-column width.
        let visible_width = display_width(&status);
        assert!(
            visible_width <= 30,
            "status row width {visible_width} must not exceed inner_width 30: {status}"
        );
    }

    #[test]
    fn render_pane_lines_shows_branch_and_ports_on_same_row() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Default, PaneStatus::Running, "");
        let ports = vec![3000, 5173];
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo {
                repo_root: Some("/tmp/project".into()),
                branch: Some("feature/sidebar".into()),
                is_worktree: false,
                worktree_name: None,
            },
            Some(&ports),
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let branch_port_line = line_text(&lines[1]);
        assert!(branch_port_line.contains("feature/sidebar"));
        assert!(branch_port_line.contains(":3000, 5173"));
        assert!(branch_port_line.find("feature/sidebar") < branch_port_line.find(":3000, 5173"));
    }

    #[test]
    fn render_pane_lines_truncates_long_branch_when_ports_present() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Default, PaneStatus::Running, "");
        let ports = vec![3000];
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo {
                repo_root: Some("/tmp/project".into()),
                branch: Some("feature/sidebar/really-long-branch-name-that-should-truncate".into()),
                is_worktree: false,
                worktree_name: None,
            },
            Some(&ports),
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let branch_port_line = line_text(&lines[1]);
        assert!(
            branch_port_line.contains('…'),
            "long branch should be truncated"
        );
        assert!(branch_port_line.contains(":3000"));
        assert!(
            branch_port_line.find('…') < branch_port_line.find(":3000"),
            "branch truncation should remain left of the port text"
        );
    }

    #[test]
    fn render_pane_lines_uses_injected_now_for_elapsed() {
        let theme = ColorTheme::default();
        let mut pane = pane(PermissionMode::Default, PaneStatus::Running, "");
        pane.started_at = Some(1_000_000 - 125);
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            1_000_000,
        );

        let status = line_text(&lines[0]);
        assert!(status.contains("2m5s"));
    }

    #[test]
    fn running_icon_for_all_statuses() {
        let icons = StatusIcons::default();
        assert_eq!(
            running_icon_for(&PaneStatus::Idle, 0, &icons),
            ("\u{f04b2}", None)
        );
        assert_eq!(
            running_icon_for(&PaneStatus::Waiting, 0, &icons),
            ("\u{f0954}", None)
        );
        assert_eq!(
            running_icon_for(&PaneStatus::Error, 0, &icons),
            ("\u{f071}", None)
        );
        assert_eq!(
            running_icon_for(&PaneStatus::Unknown, 0, &icons),
            ("·", None)
        );
        assert_eq!(
            running_icon_for(&PaneStatus::Background, 0, &icons),
            ("◎", None)
        );

        let (icon, color) = running_icon_for(&PaneStatus::Running, 0, &icons);
        assert_eq!(icon, "\u{ef0c}");
        assert_eq!(color, Some(Color::Indexed(82)));
    }

    #[test]
    fn render_pane_lines_shows_idle_prompt_hint() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Default, PaneStatus::Idle, "");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert_eq!(lines.len(), 2);
        let hint = line_text(&lines[1]);
        assert!(hint.contains("Waiting for prompt"));
    }

    #[test]
    fn render_pane_lines_shows_bg_command_even_while_running() {
        // A live background shell must stay visible in the pane
        // regardless of the agent's current status — running bursts in
        // the middle of a background task should not make the shell
        // look like it vanished.
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Running, "");
        p.bg_shell_cmd = Some("npm run dev".into());
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        let hint = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(
            hint.contains("npm run dev"),
            "bg command must render during running state, got:\n{hint}"
        );
    }

    #[test]
    fn render_pane_lines_shows_bg_command_even_while_idle() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Idle, "");
        p.bg_shell_cmd = Some("cargo watch".into());
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        let joined = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(
            joined.contains("cargo watch"),
            "bg command must render during idle state, got:\n{joined}"
        );
    }

    #[test]
    fn render_pane_lines_shows_background_command_when_known() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Background, "");
        p.bg_shell_cmd = Some("cargo build --release".into());
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert_eq!(lines.len(), 2);
        let hint = line_text(&lines[1]);
        assert!(hint.contains("cargo build --release"), "got: {hint}");
        assert!(
            !hint.contains("Background shell running"),
            "fallback text must not appear when a command is known, got: {hint}"
        );
    }

    #[test]
    fn render_pane_lines_truncates_long_background_command() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Background, "");
        p.bg_shell_cmd = Some("cargo run --bin very-long-command-name --flag".into());
        // Narrow width forces the ellipsis path in `truncate_to_width`.
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            20,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        let hint = line_text(&lines[1]);
        assert!(hint.contains("cargo run"), "command missing in {hint}");
        assert!(hint.contains('\u{2026}'), "ellipsis missing in {hint}");
        assert!(
            display_width(&hint) <= 20,
            "row width {w} must not exceed inner_width 20: {hint}",
            w = display_width(&hint)
        );
    }

    #[test]
    fn render_pane_lines_wraps_prompt_when_present() {
        let theme = ColorTheme::default();
        let pane = pane(
            PermissionMode::BypassPermissions,
            PaneStatus::Idle,
            "hello world from codex",
        );
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            18,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let status = line_text(&lines[0]);
        assert!(status.contains(" codex !"));
        assert!(!line_text(&lines[1]).contains("Waiting for prompt"));
    }

    #[test]
    fn render_pane_lines_shows_single_subagent() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Running, "test");
        p.subagents = vec!["Explore".into()];
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 3);
        let sub_line = line_text(&lines[1]);
        assert!(sub_line.contains("└ "));
        assert!(sub_line.contains("Explore #1"));
    }

    #[test]
    fn render_pane_lines_shows_multiple_subagents_tree() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Running, "test");
        p.subagents = vec!["Explore #1".into(), "Plan".into(), "Explore #2".into()];
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 5);
        assert!(line_text(&lines[1]).contains("├ "));
        assert!(line_text(&lines[1]).contains("Explore #1"));
        assert!(line_text(&lines[2]).contains("├ "));
        assert!(line_text(&lines[2]).contains("Plan #2"));
        assert!(line_text(&lines[3]).contains("└ "));
        assert!(line_text(&lines[3]).contains("Explore #2"));
    }

    #[test]
    fn render_pane_lines_subagents_before_wait_reason() {
        let theme = ColorTheme::default();
        let mut p = pane(PermissionMode::Default, PaneStatus::Waiting, "");
        p.subagents = vec!["Explore".into()];
        p.wait_reason = "permission_prompt".into();
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 3);
        let sub_line = line_text(&lines[1]);
        assert!(sub_line.contains("Explore #1"));
        let reason_line = line_text(&lines[2]);
        assert!(reason_line.contains("permission required"));
    }

    #[test]
    fn render_pane_lines_response_shows_arrow() {
        let theme = ColorTheme::default();
        let p = pane_with_response(
            PermissionMode::Default,
            PaneStatus::Idle,
            "Task completed successfully",
            true,
        );
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let response_line = line_text(&lines[1]);
        assert!(response_line.contains(RESPONSE_ARROW_PREFIX));
        assert!(response_line.contains("Task completed successfully"));
    }

    #[test]
    fn render_pane_lines_response_uses_char_wrap() {
        let theme = ColorTheme::default();
        let p = pane_with_response(
            PermissionMode::Default,
            PaneStatus::Idle,
            "abcdef ghijk lmnop qrstu vwxyz",
            true,
        );
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            20,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let first = line_text(&lines[1]);
        assert!(first.contains(RESPONSE_ARROW_PREFIX));
        // char-wrap must not trim inter-word spaces like word-wrap does
        let second = line_text(&lines[2]);
        assert!(!second.starts_with("│  ghijk"));
    }

    #[test]
    fn render_pane_lines_normal_prompt_not_detected_as_response() {
        let theme = ColorTheme::default();
        let p = pane(PermissionMode::Default, PaneStatus::Running, "fix the bug");
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let prompt_line = line_text(&lines[1]);
        assert!(!prompt_line.contains(RESPONSE_ARROW_PREFIX.trim_end()));
        assert!(prompt_line.contains("fix the bug"));
    }

    #[test]
    fn render_pane_lines_shows_task_progress() {
        use crate::activity::{TaskProgress, TaskStatus};
        let theme = ColorTheme::default();
        let p = pane(PermissionMode::Default, PaneStatus::Running, "");
        let progress = TaskProgress {
            tasks: vec![
                ("Task A".into(), TaskStatus::Completed),
                ("Task B".into(), TaskStatus::InProgress),
                ("Task C".into(), TaskStatus::Pending),
            ],
        };
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            Some(&progress),
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(lines.len() >= 2);
        let task_line = line_text(&lines[1]);
        assert!(task_line.contains("✔◼◻"));
        assert!(task_line.contains("1/3"));
    }

    #[test]
    fn render_pane_lines_no_task_line_when_empty() {
        use crate::activity::TaskProgress;
        let theme = ColorTheme::default();
        let p = pane(PermissionMode::Default, PaneStatus::Idle, "");
        let progress = TaskProgress { tasks: vec![] };
        let lines = render_pane_lines_with_ports(
            &p,
            &PaneGitInfo::default(),
            None,
            Some(&progress),
            false,
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert_eq!(lines.len(), 2);
        let hint = line_text(&lines[1]);
        assert!(hint.contains("Waiting for prompt"));
    }

    #[test]
    fn branch_ports_row_renders_port_only_without_branch() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let ports = vec![3000];
        let line = branch_ports_row(&PaneGitInfo::default(), Some(&ports), false, &ctx)
            .expect("should render port line");
        assert!(line_text(&line).contains(":3000"));
    }

    #[test]
    fn branch_ports_row_renders_plus_marker_for_non_spawned_worktree() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feat/x".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let line = branch_ports_row(&git, None, false, &ctx).expect("branch row should render");
        let text = line_text(&line);
        assert!(text.contains("+ feat/x"), "plain + marker: {text}");
        assert!(!text.contains('×'), "non-spawned must not render ×");
    }

    #[test]
    fn branch_ports_row_pins_trailing_x_to_right_edge_for_sidebar_spawned_worktree() {
        let theme = ColorTheme::default();
        let inner_width = 40usize;
        let ctx = test_ctx(&theme, inner_width, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feat/x".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let line = branch_ports_row(&git, None, true, &ctx).expect("branch row should render");
        let text = line_text(&line);
        assert!(
            text.contains("+ feat/x"),
            "+ worktree marker must be preserved: {text}"
        );
        // The trailing `×` must sit at the very last row column
        // (= inner_width + 1), mirroring the repo header's
        // right-aligned `+` spawn button.
        assert_eq!(
            rendered_x_col(&text),
            inner_width + 1,
            "× should pin to the rightmost column"
        );
        // The `×` suffix must come AFTER the branch text, not before.
        let plus_idx = text.find("+ feat/x").unwrap();
        let x_idx = text.find('×').unwrap();
        assert!(
            plus_idx < x_idx,
            "`+ feat/x` must precede the trailing `×`, got: {text}"
        );
        // Branch text stays in the normal branch color.
        let body_span = line
            .spans
            .iter()
            .find(|s| s.content.contains("feat/x"))
            .expect("branch body span");
        assert_eq!(body_span.style.fg, Some(theme.branch));
        // The trailing `×` span is painted with status_error so the
        // glyph reads as a remove action.
        let marker_span = line
            .spans
            .iter()
            .find(|s| s.content == "×")
            .expect("× span");
        assert_eq!(marker_span.style.fg, Some(theme.status_error));
    }

    /// Display column (in terminal cells, not bytes) where the
    /// first `×` appears in the rendered row. `text.find` returns a
    /// byte index which skews for multibyte glyphs like `…`, so
    /// tests that truncate branches need to measure in display cells.
    fn rendered_x_col(text: &str) -> usize {
        let idx = text.find('×').expect("× should be present");
        display_width(&text[..idx])
    }

    #[test]
    fn branch_ports_row_truncates_long_branch_but_keeps_x_at_right_edge() {
        let theme = ColorTheme::default();
        // Narrow ctx forces the branch text to truncate, but the
        // `×` must still render at the right edge — the action
        // affordance cannot be the thing that gets clipped.
        let inner_width = 18usize;
        let ctx = test_ctx(&theme, inner_width, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feature/really-long-branch-name".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let line = branch_ports_row(&git, None, true, &ctx).expect("branch row should render");
        let text = line_text(&line);
        assert!(
            text.contains('×'),
            "× must remain visible even when branch truncates: {text}"
        );
        assert!(
            text.contains('…'),
            "branch text should show truncation ellipsis: {text}"
        );
        assert_eq!(
            rendered_x_col(&text),
            inner_width + 1,
            "× stays pinned to right edge under truncation"
        );
        // Total row width = marker(1) + space(1) + inner_width.
        let rendered_width = display_width(text.trim_end());
        assert!(
            rendered_width <= inner_width + 2,
            "row must fit within marker + inner width (={}), got {rendered_width}: {text}",
            inner_width + 2
        );
    }

    #[test]
    fn sidebar_remove_marker_col_matches_branch_ports_row_layout() {
        // The click-target math in panes.rs uses
        // `sidebar_remove_marker_col` to line up the hit region with
        // the rendered `×`. Verify the two agree by counting the `×`
        // position in the rendered text and comparing to the helper.
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feat/abc".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let line = branch_ports_row(&git, None, true, &ctx).expect("branch row should render");
        let text = line_text(&line);
        let computed = sidebar_remove_marker_col(&git, None, true, ctx.inner_width, ctx.indent)
            .expect("col should be Some");
        assert_eq!(
            computed as usize,
            rendered_x_col(&text),
            "computed × col must match rendered position"
        );
    }

    #[test]
    fn sidebar_remove_marker_col_does_not_depend_on_branch_length() {
        // Right-edge pinning means the col is determined by the row
        // width alone. A short and a long branch must produce the
        // same col.
        let short = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("x".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let long = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feature/really-long-branch-name".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let col_short = sidebar_remove_marker_col(&short, None, true, 40, 0);
        let col_long = sidebar_remove_marker_col(&long, None, true, 40, 0);
        assert_eq!(col_short, col_long);
        assert_eq!(col_short, Some(41));
    }

    #[test]
    fn sidebar_remove_marker_col_returns_none_for_non_spawned() {
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feat/x".into()),
            is_worktree: true,
            worktree_name: None,
        };
        assert_eq!(sidebar_remove_marker_col(&git, None, false, 40, 0), None);
    }

    #[test]
    fn sidebar_remove_marker_col_returns_none_for_non_worktree_branch() {
        // sidebar_spawned=true but the branch label has no `+` prefix
        // (is_worktree=false). No × should be rendered or registered.
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("main".into()),
            is_worktree: false,
            worktree_name: None,
        };
        assert_eq!(sidebar_remove_marker_col(&git, None, true, 40, 0), None);
    }

    #[test]
    fn branch_ports_row_keeps_x_at_right_edge_when_ports_are_present() {
        // Ports (`  :3000`) eat space on the right side of the row,
        // but the `×` must still pin to the very last column with
        // ports stacked just to its left.
        let theme = ColorTheme::default();
        let inner_width = 40usize;
        let ctx = test_ctx(&theme, inner_width, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("feat/abc".into()),
            is_worktree: true,
            worktree_name: None,
        };
        let ports = [3000u16];
        let line =
            branch_ports_row(&git, Some(&ports), true, &ctx).expect("branch row should render");
        let text = line_text(&line);
        assert_eq!(
            rendered_x_col(&text),
            inner_width + 1,
            "× must pin to right edge regardless of port presence"
        );
        let port_idx = text.find(":3000").expect(":3000 should be present");
        let x_idx = text.find('×').unwrap();
        assert!(
            port_idx < x_idx,
            "ports should sit to the LEFT of the × marker: {text}"
        );
    }

    #[test]
    fn branch_ports_row_keeps_plain_branch_when_sidebar_spawned_but_not_worktree() {
        // Edge case: sidebar_spawned=true but is_worktree=false.
        // `branch_label` does not emit the "+ " prefix, so
        // branch_ports_row must not try to swap anything and the
        // resulting row must stay plain.
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let git = PaneGitInfo {
            repo_root: Some("/r".into()),
            branch: Some("main".into()),
            is_worktree: false,
            worktree_name: None,
        };
        let line = branch_ports_row(&git, None, true, &ctx).expect("branch row should render");
        let text = line_text(&line);
        assert!(text.contains("main"));
        assert!(!text.contains('×'));
        assert!(!text.contains('+'));
    }

    #[test]
    fn wait_reason_row_uses_error_color_when_status_is_error() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let line = wait_reason_row("permission_prompt", &PaneStatus::Error, &ctx, 0)
            .expect("should render reason line");
        let text_span = line
            .spans
            .iter()
            .find(|s| s.content.contains("permission"))
            .expect("reason text should be present");
        assert_eq!(text_span.style.fg, Some(theme.status_error));
    }

    #[test]
    fn wait_reason_row_labels_a_pi_prompt_kind() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let line = wait_reason_row("custom", &PaneStatus::Waiting, &ctx, 0)
            .expect("should render reason line");
        // Snapshot the visible text; the row padding that follows it is
        // covered by `pad_to`.
        insta::assert_snapshot!(line_text(&line).trim_end(), @"    prompt");
    }

    #[test]
    fn wait_reason_row_paints_action_required_reasons_apart_from_the_agent_title() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let color_of = |reason: &str| -> Option<Color> {
            let line = wait_reason_row(reason, &PaneStatus::Waiting, &ctx, 0)
                .expect("should render reason line");
            // The marker and padding spans carry no foreground, so the first
            // one that does is the reason text.
            line.spans.iter().find_map(|span| span.style.fg)
        };

        for reason in ["custom", "select", "confirm", "input", "editor"] {
            assert_eq!(
                color_of(reason),
                Some(theme.wait_reason_action),
                "{reason} demands the user and should use the action colour"
            );
        }
        assert_eq!(
            color_of("auth_success"),
            Some(theme.wait_reason),
            "a soft reason keeps the ordinary wait colour"
        );
    }

    #[test]
    fn render_pane_lines_selected_applies_background_to_spans() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Auto, PaneStatus::Running, "do work");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            true, // selected
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        // Every inner (non-marker) span on the status line must carry the selection bg.
        // The left marker uses marker_style only.
        let status = &lines[0];
        let has_bg = status
            .spans
            .iter()
            .any(|s| s.style.bg == Some(theme.selection_bg));
        assert!(
            has_bg,
            "selected row should apply selection_bg to inner spans"
        );
    }

    #[test]
    fn render_pane_lines_selected_leaves_content_rows_unhighlighted() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Auto, PaneStatus::Running, "do work");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            true, // selected
            false,
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        assert!(
            lines
                .iter()
                .skip(1)
                .flat_map(|line| &line.spans)
                .all(|span| span.style.bg != Some(theme.selection_bg)),
            "content rows should not carry the selection background"
        );
    }

    #[test]
    fn render_pane_lines_active_shows_left_marker_on_status_row() {
        let theme = ColorTheme::default();
        let pane = pane(PermissionMode::Default, PaneStatus::Running, "");
        let lines = render_pane_lines_with_ports(
            &pane,
            &PaneGitInfo::default(),
            None,
            None,
            false,
            true, // active
            40,
            &StatusIcons::default(),
            &theme,
            0,
            0,
        );

        // The status row (line 0) must start with the SELECTION_MARKER in the
        // accent fg; no BOLD is applied to the title span.
        let marker_span = &lines[0].spans[0];
        assert_eq!(marker_span.content, SELECTION_MARKER);
        assert_eq!(marker_span.style.fg, Some(theme.accent));

        let title_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("codex"))
            .expect("title span should be present");
        assert!(
            !title_span.style.add_modifier.contains(Modifier::BOLD),
            "active pane title should not be BOLD"
        );
    }

    #[test]
    fn status_row_default_permission_mode_omits_badge() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let pane = pane(PermissionMode::Default, PaneStatus::Running, "");
        let line = status_row(&pane, &ctx, &StatusIcons::default(), 0, 0);
        let text = line_text(&line);
        // Default mode has an empty badge string — no extra badge token should appear.
        assert!(
            !text.contains(" auto") && !text.contains(" plan") && !text.contains(" !"),
            "default permission mode should not render a badge, got: {text}"
        );
    }

    #[test]
    fn prompt_rows_indents_continuation_lines() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 20, false);
        let mut p = pane(
            PermissionMode::Default,
            PaneStatus::Running,
            "aaaa bbbb cccc dddd eeee",
        );
        p.prompt_is_response = false;
        let lines = prompt_rows(&p, &ctx);
        assert!(
            lines.len() >= 2,
            "expected prompt to wrap across multiple lines"
        );
        for line in &lines {
            let text = line_text(line);
            // Each line starts with marker(1) + space(1) + indent(2) = "    " for non-selected.
            assert!(
                text.starts_with("    "),
                "each wrapped line should carry the left padding, got: {text}"
            );
        }
    }

    #[test]
    fn wait_reason_row_blinks_action_required_label() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let fg_at = |reason: &str, frame: usize| {
            wait_reason_row(reason, &PaneStatus::Waiting, &ctx, frame)
                .expect("action-required reason should render")
                .spans
                .iter()
                .find_map(|s| s.style.fg)
        };
        let blink_frame = crate::SPINNER_PULSE.len() / 2;

        // Red on the first half of the cycle, dimmed red on the second —
        // a visible pulse on the demand-for-input label only, never grey.
        assert_eq!(fg_at("custom", 0), Some(theme.wait_reason_action));
        assert_eq!(
            fg_at("custom", blink_frame),
            Some(theme.wait_reason_action_dim)
        );
        for reason in ["select", "confirm", "input", "editor"] {
            assert_eq!(
                fg_at(reason, blink_frame),
                Some(theme.wait_reason_action_dim)
            );
        }

        // Non-action reasons and the error-status row hold their color.
        assert_eq!(fg_at("auth_success", 0), Some(theme.wait_reason));
        assert_eq!(fg_at("auth_success", blink_frame), Some(theme.wait_reason));
        assert_eq!(
            wait_reason_row("permission_prompt", &PaneStatus::Error, &ctx, blink_frame)
                .expect("reason should render")
                .spans
                .iter()
                .find_map(|s| s.style.fg),
            Some(theme.status_error)
        );
    }

    // ─── pmux subagent badge and nesting ────────────────────────

    fn pi_pane() -> PaneInfo {
        let mut p = pane(PermissionMode::Default, PaneStatus::Running, "");
        p.agent = AgentType::Pi;
        p
    }

    fn pi_subagent(kind: PmuxTaskKind, parent: SubagentParent, task: Option<&str>) -> PaneInfo {
        let mut p = pi_pane();
        p.subagent = Some(SubagentInfo {
            kind,
            task: task.map(str::to_string),
            parent,
        });
        p
    }

    #[test]
    fn status_row_spawned_subagent_shows_badge() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let p = pi_subagent(
            PmuxTaskKind::Spawned,
            SubagentParent::Pane("%0".into()),
            None,
        );
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    ♟");
    }

    #[test]
    fn status_row_resumed_subagent_shows_badge() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let p = pi_subagent(
            PmuxTaskKind::Resumed,
            SubagentParent::Pane("%0".into()),
            None,
        );
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    ↻");
    }

    #[test]
    fn status_row_orphaned_subagent_shows_the_tag_in_the_error_color() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let p = pi_subagent(PmuxTaskKind::Spawned, SubagentParent::Orphaned, None);
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    ♟ ORPHANED");
        let tag = line
            .spans
            .iter()
            .find(|span| span.content.contains("ORPHANED"))
            .expect("the ORPHANED tag span");
        assert_eq!(
            tag.style.fg,
            Some(theme.status_error),
            "the tag uses the theme's error color"
        );
    }

    #[test]
    fn status_row_resolved_subagent_has_no_tag() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let p = pi_subagent(
            PmuxTaskKind::Resumed,
            SubagentParent::Pane("%0".into()),
            None,
        );
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    ↻");
        assert!(
            !line_text(&line).contains("ORPHANED"),
            "only an orphaned subagent carries the tag"
        );
    }

    #[test]
    fn status_row_long_pane_name_truncates_before_the_orphan_tag() {
        // The tag is fixed furniture on an orphaned row: the pane name
        // is cut first and the whole tag always survives.
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 30, false);
        let mut p = pi_subagent(PmuxTaskKind::Spawned, SubagentParent::Orphaned, None);
        p.pane_name = Some("a-very-long-pane-name-that-will-not-fit".into());
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        let text = line_text(&line);
        insta::assert_snapshot!(text, @"    ♟ ORPHANED a-very-long-pa…");
        assert!(
            text.contains("ORPHANED"),
            "the tag survives a tight row: {text:?}"
        );
        assert!(
            text.contains('\u{2026}'),
            "the pane name is what gets cut: {text:?}"
        );
    }

    #[test]
    fn status_row_plain_pi_pane_shows_no_badge() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let p = pi_pane();
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"   ");
    }

    #[test]
    fn status_row_lifted_branch_shows_after_pane_name() {
        // The strip's lifted branch: the label after the pane name in
        // the branch colour — the dedicated branch row is gone.
        let theme = ColorTheme::default();
        let mut ctx = test_ctx(&theme, 40, false);
        ctx.inline_branch = Some("feat/auth".into());
        let mut p = pi_pane();
        p.pane_name = Some("my-task".into());
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    my-task feat/auth");
    }

    #[test]
    fn status_row_lifted_branch_truncates_to_fit() {
        // A tight row cuts the branch with an ellipsis — the pane
        // name is the row's identity, the branch only its context.
        let theme = ColorTheme::default();
        let mut ctx = test_ctx(&theme, 20, false);
        ctx.inline_branch = Some("feat/auth".into());
        let mut p = pi_pane();
        p.pane_name = Some("my-task".into());
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    my-task feat/au…");
    }

    #[test]
    fn status_row_named_pane_shows_agent_label_then_pane_name() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let mut p = pi_pane();
        p.pane_name = Some("my-task".into());
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    my-task");
    }

    #[test]
    fn status_row_named_subagent_shows_badge_before_pane_name() {
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 40, false);
        let mut p = pi_subagent(PmuxTaskKind::Spawned, SubagentParent::Orphaned, None);
        p.pane_name = Some("my-task".into());
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 0);
        insta::assert_snapshot!(line_text(&line), @"    ♟ ORPHANED my-task");
    }

    #[test]
    fn status_row_long_pane_name_truncates_keeps_label_and_elapsed() {
        // A pane name longer than the space left after the status icon,
        // the agent label, and the elapsed label is cut with an ellipsis;
        // the agent label and the elapsed label stay fully visible.
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 30, false);
        let mut p = pi_pane();
        p.pane_name = Some("a-very-long-pane-name-that-will-not-fit".into());
        p.started_at = Some(1);
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 66);
        let text = line_text(&line);
        insta::assert_snapshot!(text, @"    a-very-long-pane-nam… 1m5s");
    }

    #[test]
    fn status_row_subagent_badge_counts_toward_width_budget() {
        // The badge and the ORPHANED tag sit before the pane name and eat
        // into the title budget the same way a permission badge does, so
        // the elapsed label keeps its place on the row.
        let theme = ColorTheme::default();
        let ctx = test_ctx(&theme, 30, false);
        let mut p = pi_subagent(PmuxTaskKind::Spawned, SubagentParent::Orphaned, None);
        p.pane_name = Some("a-very-long-session-name-that-will-not-fit".into());
        p.started_at = Some(1);
        let line = status_row(&p, &ctx, &StatusIcons::default(), 0, 66);
        let text = line_text(&line);
        insta::assert_snapshot!(
            text,
            @"    ♟ ORPHANED a-very-lo… 1m5s"
        );
        assert!(
            display_width(text.trim_end()) <= ctx.inner_width + 2,
            "row must stay within marker + inner_width 30: {text}"
        );
    }

    #[test]
    fn a_nested_pane_indents_every_row_at_width_40() {
        let mut p = pi_subagent(
            PmuxTaskKind::Spawned,
            SubagentParent::Pane("%0".into()),
            Some("unit1 idle collapse"),
        );
        p.prompt = "Task completed".into();
        p.prompt_is_response = true;
        let nested = render_nested(&p, 40, true);
        let flat = render_nested(&p, 40, false);
        let joined = nested
            .iter()
            .map(|l| line_text(l).trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(joined, @"
          ♟
        󰭹 Task completed
        ");
        assert_eq!(
            nested.len(),
            flat.len(),
            "the removed parent row leaves a nested pane with exactly the rows a top-level pane draws"
        );
        for (nested_line, flat_line) in nested.iter().zip(flat.iter()) {
            let nested_text = line_text(nested_line);
            let flat_text = line_text(flat_line);
            assert_eq!(
                display_width(nested_text.trim_end()),
                display_width(flat_text.trim_end()) + NEST_INDENT,
                "all content shifts right by the indent: {nested_text:?}"
            );
            assert!(
                display_width(&nested_text) <= 40,
                "a nested row stays inside the width: {nested_text:?}"
            );
        }
    }

    #[test]
    fn a_nested_pane_is_cut_inside_the_indent_at_a_narrow_width() {
        let mut p = pi_subagent(
            PmuxTaskKind::Resumed,
            SubagentParent::Pane("%0".into()),
            Some("unit2"),
        );
        p.prompt = "A response whose text is far longer than the narrow sidebar".into();
        p.prompt_is_response = true;
        let nested = render_nested(&p, 18, true);
        let joined = nested
            .iter()
            .map(|l| line_text(l).trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(joined, @"
          ↻
        󰭹 A response
          whose text
          is far…
        ");
        assert_eq!(
            nested.len(),
            render_nested(&p, 18, false).len(),
            "a narrow sidebar does not add or drop rows"
        );
        for line in &nested {
            let text = line_text(line);
            assert!(
                text.starts_with("    "),
                "the marker, space and indent stay on every row: {text:?}"
            );
            assert!(
                display_width(&text) <= 18,
                "a narrow row is cut, never overflows: {text:?}"
            );
        }
    }

    #[test]
    fn a_nested_subagent_response_preview_still_renders() {
        let mut p = pi_subagent(
            PmuxTaskKind::Spawned,
            SubagentParent::Pane("%0".into()),
            Some("unit1 idle collapse"),
        );
        p.prompt = "Task completed".into();
        p.prompt_is_response = true;
        let lines = render_pane_lines(
            &p,
            &PaneGitInfo {
                repo_root: Some("/tmp/project".into()),
                branch: Some("main".into()),
                is_worktree: false,
                worktree_name: None,
            },
            None,
            None,
            false,
            false,
            40,
            &StatusIcons::default(),
            &ColorTheme::default(),
            0,
            0,
            false,
            true,
        );
        let joined = lines
            .iter()
            .map(|l| line_text(l).trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(joined, @"
          ♟
           main
        󰭹 Task completed
        ");
        assert!(
            joined.contains("Task completed"),
            "a subagent's response preview must still render below the status row"
        );
    }
}
