use ratatui::style::Color;

use crate::shell_highlight::Class;
use crate::tmux::{self, AgentType, PaneStatus};

/// Runtime color theme, loaded from tmux @sidebar_color_* variables on startup.
/// Overrides may be xterm-256 indexes or six-digit RGB hex values.
/// Falls back to defaults if tmux variables are not set.
#[derive(Debug, Clone)]
pub struct ColorTheme {
    /// Accent color shared by every "active / focused" affordance:
    /// the `┃` marker on the active pane, the focused repo header, the
    /// bottom panel border when Activity/Git is focused, and the repo
    /// popup border.
    pub accent: Color,
    pub border_inactive: Color,
    pub status_all: Color,
    pub status_running: Color,
    pub status_waiting: Color,
    pub status_idle: Color,
    pub status_error: Color,
    pub status_unknown: Color,
    pub filter_inactive: Color,
    /// Filter bar row background. Painted at all times, so it sits over
    /// the focus fill while the sidebar pane is the active one.
    /// Default Tokyo Night line highlight (`#414868`).
    pub filter_bar_bg: Color,
    pub agent_claude: Color,
    pub agent_codex: Color,
    pub agent_opencode: Color,
    pub agent_pi: Color,
    pub pet_body: Color,
    pub pet_eye: Color,
    pub text_active: Color,
    pub text_muted: Color,
    pub text_inactive: Color,
    pub session_header: Color,
    pub port: Color,
    pub wait_reason: Color,
    /// Wait-reason text for a reason that demands the user. Default
    /// Tokyo Night red (`#f7768e`) so it does not blend into the yellow
    /// agent title on the row above.
    pub wait_reason_action: Color,
    /// The dim phase of an action-required reason's blink — same hue
    /// family as `wait_reason_action`, darker, so the pulse stays red.
    pub wait_reason_action_dim: Color,
    pub selection_bg: Color,
    /// Base focus background over the whole sidebar while its tmux pane
    /// is the active one. Default Tokyo Night background (`#1a1b26`)
    /// sits below selection_bg (`#414868`) so the cursor still pops.
    pub focus_bg: Color,
    /// Active focus background over the keyboard-owning section.
    /// Default Tokyo Night storm background (`#24283b`) stays distinct
    /// from focus_bg on a dark terminal.
    pub focus_active_bg: Color,
    pub branch: Color,
    pub badge_danger: Color,
    pub badge_auto: Color,
    pub badge_plan: Color,
    pub task_progress: Color,
    /// Tasks tab color for `pending` items (`◻`).
    pub task_pending: Color,
    /// Tasks tab color for `completed` items (`✔`).
    pub task_completed: Color,
    pub subagent: Color,
    pub commit_hash: Color,
    pub diff_added: Color,
    pub diff_deleted: Color,
    pub file_change: Color,
    pub pr_link: Color,
    pub section_title: Color,
    pub activity_timestamp: Color,
    /// Thought tab color of the `HH:mm:ss` line above the thinking text.
    pub thought_timestamp: Color,
    /// Tools tab color of a shell command word (`Bash`/`Monitor` labels).
    /// Default Tokyo Night blue (`#7aa2f7`).
    pub cmd_command: Color,
    /// Tools tab color of a flag word (`--release`). Default Tokyo Night
    /// cyan (`#7dcfff`).
    pub cmd_flag: Color,
    /// Tools tab color of quoted text. Default Tokyo Night yellow
    /// (`#e0af68`).
    pub cmd_string: Color,
    /// Tools tab color of `$variables` and leading `NAME=value`
    /// assignments. Default Tokyo Night purple (`#bb9af7`).
    pub cmd_variable: Color,
    /// Tools tab color of separators and redirections. Default Tokyo
    /// Night cyan2 (`#89ddff`).
    pub cmd_operator: Color,
    /// Tools tab color of a `#` comment. Default Tokyo Night comment
    /// grey-blue (`#565f89`).
    pub cmd_comment: Color,
    pub response_arrow: Color,
    /// Border colour tmux draws around the sidebar pane while it is the
    /// active pane of its window. Default is everforest dark-medium
    /// orange (`#E69875`): loud against dark backgrounds without
    /// reading as an error.
    pub focus_border: Color,
    /// The ` │ ` delimiter between the bottom-panel tab glyphs on the
    /// rule. Default Tokyo Night dim foreground (`#9aa5ce`), quiet
    /// against the filter-bar band next to the bright tab glyphs.
    pub tab_separator: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),
            border_inactive: Color::Indexed(240),
            status_all: Color::Rgb(0x7a, 0xa2, 0xf7),
            status_running: Color::Rgb(0x9e, 0xce, 0x6a),
            status_waiting: Color::Rgb(0xe0, 0xaf, 0x68),
            status_idle: Color::Rgb(0x7d, 0xcf, 0xff),
            status_error: Color::Rgb(0xf7, 0x76, 0x8e),
            status_unknown: Color::Rgb(0x56, 0x5f, 0x89),
            filter_inactive: Color::Rgb(0x9a, 0xa5, 0xce),
            filter_bar_bg: Color::Rgb(0x41, 0x48, 0x68),
            agent_claude: Color::Rgb(0xff, 0x9e, 0x64),
            agent_codex: Color::Rgb(0xbb, 0x9a, 0xf7),
            agent_opencode: Color::Rgb(0x7d, 0xcf, 0xff),
            agent_pi: Color::Rgb(0xe0, 0xaf, 0x68),
            pet_body: Color::Rgb(0xff, 0x9e, 0x64),
            pet_eye: Color::Rgb(0x9e, 0xce, 0x6a),
            text_active: Color::Rgb(0xc0, 0xca, 0xf5),
            text_muted: Color::Rgb(0xa9, 0xb1, 0xd6),
            text_inactive: Color::Rgb(0x56, 0x5f, 0x89),
            session_header: Color::Rgb(0x7d, 0xcf, 0xff),
            port: Color::Rgb(0x9a, 0xa5, 0xce),
            wait_reason: Color::Rgb(0xe0, 0xaf, 0x68),
            wait_reason_action: Color::Rgb(0xf7, 0x76, 0x8e),
            wait_reason_action_dim: Color::Rgb(0x8a, 0x3f, 0x4e),
            selection_bg: Color::Rgb(0x41, 0x48, 0x68),
            focus_bg: Color::Rgb(0x1a, 0x1b, 0x26),
            focus_active_bg: Color::Rgb(0x24, 0x28, 0x3b),
            branch: Color::Rgb(0x73, 0xda, 0xcb),
            badge_danger: Color::Rgb(0xf7, 0x76, 0x8e),
            badge_auto: Color::Rgb(0xe0, 0xaf, 0x68),
            badge_plan: Color::Rgb(0x7d, 0xcf, 0xff),
            task_progress: Color::Rgb(0xff, 0x9e, 0x64),
            task_pending: Color::Rgb(0x7a, 0xa2, 0xf7),
            task_completed: Color::Rgb(0x9e, 0xce, 0x6a),
            subagent: Color::Rgb(0x73, 0xda, 0xcb),
            commit_hash: Color::Rgb(0xe0, 0xaf, 0x68),
            diff_added: Color::Rgb(0x9e, 0xce, 0x6a),
            diff_deleted: Color::Rgb(0xf7, 0x76, 0x8e),
            file_change: Color::Rgb(0xe0, 0xaf, 0x68),
            pr_link: Color::Rgb(0x7d, 0xcf, 0xff),
            section_title: Color::Rgb(0x2a, 0xc3, 0xde),
            activity_timestamp: Color::Rgb(0x2a, 0xc3, 0xde),
            thought_timestamp: Color::Rgb(0xe0, 0xaf, 0x68),
            cmd_command: Color::Rgb(0x7a, 0xa2, 0xf7),
            cmd_flag: Color::Rgb(0x7d, 0xcf, 0xff),
            cmd_string: Color::Rgb(0xe0, 0xaf, 0x68),
            cmd_variable: Color::Rgb(0xbb, 0x9a, 0xf7),
            cmd_operator: Color::Rgb(0x89, 0xdd, 0xff),
            cmd_comment: Color::Rgb(0x56, 0x5f, 0x89),
            response_arrow: Color::Rgb(0x7d, 0xcf, 0xff),
            focus_border: Color::Rgb(0xE6, 0x98, 0x75),
            tab_separator: Color::Rgb(0x9a, 0xa5, 0xce),
        }
    }
}

impl ColorTheme {
    /// Load colors from tmux @sidebar_color_* variables, falling back to defaults.
    /// Fetches all global options in a single tmux call to avoid N subprocess forks.
    pub fn from_tmux() -> Self {
        let all_opts = tmux::get_all_global_options();
        Self::from_options(&all_opts)
    }

    /// Apply a map of global tmux options onto the default theme. Used
    /// by `from_tmux` and by tests that pin the option-to-field wiring.
    pub fn from_options(all_opts: &std::collections::HashMap<String, String>) -> Self {
        let mut theme = Self::default();

        let read = |var: &str, fallback: Color| -> Color {
            all_opts
                .get(var)
                .and_then(|s| parse_tmux_color(s))
                .unwrap_or(fallback)
        };

        theme.accent = read(tmux::SIDEBAR_COLOR_ACCENT, theme.accent);
        theme.border_inactive = read(tmux::SIDEBAR_COLOR_BORDER, theme.border_inactive);
        theme.status_all = read(tmux::SIDEBAR_COLOR_ALL, theme.status_all);
        theme.status_running = read(tmux::SIDEBAR_COLOR_RUNNING, theme.status_running);
        theme.status_waiting = read(tmux::SIDEBAR_COLOR_WAITING, theme.status_waiting);
        theme.status_idle = read(tmux::SIDEBAR_COLOR_IDLE, theme.status_idle);
        theme.status_error = read(tmux::SIDEBAR_COLOR_ERROR, theme.status_error);
        theme.filter_inactive = read(tmux::SIDEBAR_COLOR_FILTER_INACTIVE, theme.filter_inactive);
        theme.filter_bar_bg = read(tmux::SIDEBAR_COLOR_FILTER_BAR_BG, theme.filter_bar_bg);
        theme.agent_claude = read(tmux::SIDEBAR_COLOR_AGENT_CLAUDE, theme.agent_claude);
        theme.agent_codex = read(tmux::SIDEBAR_COLOR_AGENT_CODEX, theme.agent_codex);
        theme.agent_opencode = read(tmux::SIDEBAR_COLOR_AGENT_OPENCODE, theme.agent_opencode);
        theme.agent_pi = read(tmux::SIDEBAR_COLOR_AGENT_PI, theme.agent_pi);
        theme.pet_body = read(tmux::SIDEBAR_COLOR_PET_BODY, theme.pet_body);
        theme.pet_eye = read(tmux::SIDEBAR_COLOR_PET_EYE, theme.pet_eye);
        theme.text_active = read(tmux::SIDEBAR_COLOR_TEXT_ACTIVE, theme.text_active);
        theme.text_muted = read(tmux::SIDEBAR_COLOR_TEXT_MUTED, theme.text_muted);
        theme.text_inactive = read(tmux::SIDEBAR_COLOR_TEXT_INACTIVE, theme.text_inactive);
        theme.session_header = read(tmux::SIDEBAR_COLOR_SESSION, theme.session_header);
        theme.port = read(tmux::SIDEBAR_COLOR_PORT, theme.port);
        theme.wait_reason = read(tmux::SIDEBAR_COLOR_WAIT_REASON, theme.wait_reason);
        theme.wait_reason_action = read(
            tmux::SIDEBAR_COLOR_WAIT_REASON_ACTION,
            theme.wait_reason_action,
        );
        theme.wait_reason_action_dim = read(
            tmux::SIDEBAR_COLOR_WAIT_REASON_ACTION_DIM,
            theme.wait_reason_action_dim,
        );
        theme.selection_bg = read(tmux::SIDEBAR_COLOR_SELECTION, theme.selection_bg);
        theme.focus_bg = read(tmux::SIDEBAR_COLOR_FOCUS_BG, theme.focus_bg);
        theme.focus_active_bg = read(tmux::SIDEBAR_COLOR_FOCUS_ACTIVE_BG, theme.focus_active_bg);
        theme.branch = read(tmux::SIDEBAR_COLOR_BRANCH, theme.branch);
        theme.task_progress = read(tmux::SIDEBAR_COLOR_TASK_PROGRESS, theme.task_progress);
        theme.task_pending = read(tmux::SIDEBAR_COLOR_TASK_PENDING, theme.task_pending);
        theme.task_completed = read(tmux::SIDEBAR_COLOR_TASK_COMPLETED, theme.task_completed);
        theme.subagent = read(tmux::SIDEBAR_COLOR_SUBAGENT, theme.subagent);
        theme.commit_hash = read(tmux::SIDEBAR_COLOR_COMMIT_HASH, theme.commit_hash);
        theme.diff_added = read(tmux::SIDEBAR_COLOR_DIFF_ADDED, theme.diff_added);
        theme.diff_deleted = read(tmux::SIDEBAR_COLOR_DIFF_DELETED, theme.diff_deleted);
        theme.file_change = read(tmux::SIDEBAR_COLOR_FILE_CHANGE, theme.file_change);
        theme.pr_link = read(tmux::SIDEBAR_COLOR_PR_LINK, theme.pr_link);
        theme.section_title = read(tmux::SIDEBAR_COLOR_SECTION_TITLE, theme.section_title);
        theme.activity_timestamp = read(
            tmux::SIDEBAR_COLOR_ACTIVITY_TIMESTAMP,
            theme.activity_timestamp,
        );
        theme.thought_timestamp = read(
            tmux::SIDEBAR_COLOR_THOUGHT_TIMESTAMP,
            theme.thought_timestamp,
        );
        theme.response_arrow = read(tmux::SIDEBAR_COLOR_RESPONSE_ARROW, theme.response_arrow);
        theme.cmd_command = read(tmux::SIDEBAR_COLOR_CMD_COMMAND, theme.cmd_command);
        theme.cmd_flag = read(tmux::SIDEBAR_COLOR_CMD_FLAG, theme.cmd_flag);
        theme.cmd_string = read(tmux::SIDEBAR_COLOR_CMD_STRING, theme.cmd_string);
        theme.cmd_variable = read(tmux::SIDEBAR_COLOR_CMD_VARIABLE, theme.cmd_variable);
        theme.cmd_operator = read(tmux::SIDEBAR_COLOR_CMD_OPERATOR, theme.cmd_operator);
        theme.cmd_comment = read(tmux::SIDEBAR_COLOR_CMD_COMMENT, theme.cmd_comment);
        theme.focus_border = read(tmux::SIDEBAR_COLOR_FOCUS_BORDER, theme.focus_border);
        theme.tab_separator = read(tmux::SIDEBAR_COLOR_TAB_SEPARATOR, theme.tab_separator);

        theme
    }

    pub fn status_color(&self, status: &PaneStatus, attention: bool) -> Color {
        if attention {
            return self.status_waiting;
        }
        match status {
            PaneStatus::Running => self.status_running,
            PaneStatus::Background => self.status_running,
            PaneStatus::Waiting => self.status_waiting,
            PaneStatus::Idle => self.status_idle,
            PaneStatus::Error => self.status_error,
            PaneStatus::Unknown => self.status_unknown,
        }
    }

    pub fn agent_color(&self, agent: &AgentType) -> Color {
        match agent {
            AgentType::Claude => self.agent_claude,
            AgentType::Codex => self.agent_codex,
            AgentType::OpenCode => self.agent_opencode,
            AgentType::Pi => self.agent_pi,
        }
    }

    /// The color of one shell-token class in the Tools tab. An argument
    /// keeps the muted text color the whole label used before, so the
    /// unhighlighted base stays the one the user already tunes.
    pub fn cmd_class_color(&self, class: Class) -> Color {
        match class {
            Class::Arg => self.text_muted,
            Class::Command => self.cmd_command,
            Class::Flag => self.cmd_flag,
            Class::Str => self.cmd_string,
            Class::Variable => self.cmd_variable,
            Class::Operator => self.cmd_operator,
            Class::Comment => self.cmd_comment,
        }
    }
}

/// Render a theme colour as the `fg=` item of a tmux style string, for
/// options tmux parses itself (border styles). Indexed colours use
/// tmux's `colour<n>` spelling; RGB colours use `#RRGGBB`. Any other
/// colour variant falls back to `default`, which leaves tmux's normal
/// styling in place.
pub fn tmux_fg_style(color: Color) -> String {
    match color {
        Color::Indexed(n) => format!("fg=colour{n}"),
        Color::Rgb(r, g, b) => format!("fg=#{r:02x}{g:02x}{b:02x}"),
        _ => "fg=default".to_string(),
    }
}

fn parse_tmux_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Ok(index) = value.parse::<u8>() {
        return Some(Color::Indexed(index));
    }

    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let rgb = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::Rgb(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn status_color_attention_overrides() {
        let theme = ColorTheme::default();
        // attention=true should always return status_waiting regardless of status
        assert_eq!(
            theme.status_color(&PaneStatus::Idle, true),
            theme.status_waiting
        );
        assert_eq!(
            theme.status_color(&PaneStatus::Running, true),
            theme.status_waiting
        );
        assert_eq!(
            theme.status_color(&PaneStatus::Error, true),
            theme.status_waiting
        );
    }

    #[test]
    fn status_color_normal() {
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
    fn agent_color_all() {
        let theme = ColorTheme::default();
        assert_eq!(
            theme.agent_color(&AgentType::Claude),
            Color::Rgb(0xff, 0x9e, 0x64)
        );
        assert_eq!(
            theme.agent_color(&AgentType::Codex),
            Color::Rgb(0xbb, 0x9a, 0xf7)
        );
        assert_eq!(
            theme.agent_color(&AgentType::OpenCode),
            Color::Rgb(0x7d, 0xcf, 0xff)
        );
        assert_eq!(
            theme.agent_color(&AgentType::Pi),
            Color::Rgb(0xe0, 0xaf, 0x68)
        );
    }

    #[test]
    fn pet_color_defaults_match_current_palette() {
        let theme = ColorTheme::default();
        assert_eq!(theme.pet_body, Color::Rgb(0xff, 0x9e, 0x64));
        assert_eq!(theme.pet_eye, Color::Rgb(0x9e, 0xce, 0x6a));
    }

    #[test]
    fn from_options_accepts_hex_and_indexed_colors() {
        let mut options = std::collections::HashMap::new();
        options.insert(
            tmux::SIDEBAR_COLOR_ACCENT.to_string(),
            "#1a2b3c".to_string(),
        );
        options.insert(
            tmux::SIDEBAR_COLOR_AGENT_CODEX.to_string(),
            "d0e7ff".to_string(),
        );
        options.insert(tmux::SIDEBAR_COLOR_BORDER.to_string(), "42".to_string());

        let theme = ColorTheme::from_options(&options);

        assert_eq!(theme.accent, Color::Rgb(0x1a, 0x2b, 0x3c));
        assert_eq!(theme.agent_codex, Color::Rgb(0xd0, 0xe7, 0xff));
        assert_eq!(theme.border_inactive, Color::Indexed(42));
    }

    #[test]
    fn tmux_fg_style_covers_indexed_and_rgb() {
        assert_eq!(tmux_fg_style(Color::Indexed(42)), "fg=colour42");
        assert_eq!(tmux_fg_style(Color::Rgb(0xE6, 0x98, 0x75)), "fg=#e69875");
    }

    #[test]
    fn from_options_accepts_focus_fill_overrides() {
        let mut options = std::collections::HashMap::new();
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_BG.to_string(),
            "#0a0b0c".to_string(),
        );
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_ACTIVE_BG.to_string(),
            "50".to_string(),
        );
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_BORDER.to_string(),
            "42".to_string(),
        );

        let theme = ColorTheme::from_options(&options);

        assert_eq!(theme.focus_bg, Color::Rgb(0x0a, 0x0b, 0x0c));
        assert_eq!(theme.focus_active_bg, Color::Indexed(50));
        assert_eq!(theme.focus_border, Color::Indexed(42));
    }

    #[test]
    fn from_options_falls_back_for_invalid_focus_fills() {
        let mut options = std::collections::HashMap::new();
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_BG.to_string(),
            "not-a-color".to_string(),
        );
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_ACTIVE_BG.to_string(),
            "256".to_string(),
        );
        options.insert(
            tmux::SIDEBAR_COLOR_FOCUS_BORDER.to_string(),
            "not-a-color".to_string(),
        );

        let theme = ColorTheme::from_options(&options);
        let default_theme = ColorTheme::default();

        assert_eq!(theme.focus_bg, default_theme.focus_bg);
        assert_eq!(theme.focus_active_bg, default_theme.focus_active_bg);
        assert_eq!(theme.focus_border, default_theme.focus_border);
    }

    #[test]
    fn from_options_falls_back_for_invalid_colors() {
        let mut options = std::collections::HashMap::new();
        options.insert(tmux::SIDEBAR_COLOR_ACCENT.to_string(), "#12".to_string());
        options.insert(
            tmux::SIDEBAR_COLOR_AGENT_CLAUDE.to_string(),
            "not-a-color".to_string(),
        );
        options.insert(tmux::SIDEBAR_COLOR_BORDER.to_string(), "256".to_string());

        let theme = ColorTheme::from_options(&options);
        let default_theme = ColorTheme::default();

        assert_eq!(theme.accent, default_theme.accent);
        assert_eq!(theme.agent_claude, default_theme.agent_claude);
        assert_eq!(theme.border_inactive, default_theme.border_inactive);
    }
}
