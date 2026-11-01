use super::AppState;
use crate::tmux;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Focus {
    Filter,
    #[default]
    Panes,
    ActivityLog,
}

#[derive(Debug, Clone)]
pub struct FocusState {
    pub sidebar_focused: bool,
    pub focus: Focus,
    pub focused_pane_id: Option<String>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            sidebar_focused: false,
            focus: Focus::Panes,
            focused_pane_id: None,
        }
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn pane_by_id(&self, pane_id: &str) -> Option<&tmux::PaneInfo> {
        for group in &self.repo_groups {
            for (pane, _) in &group.panes {
                if pane.pane_id == pane_id {
                    return Some(pane);
                }
            }
        }
        None
    }

    pub fn selected_pane(&self) -> Option<&tmux::PaneInfo> {
        let target = self
            .layout
            .pane_row_targets
            .get(self.global.selected_pane_row)?;
        self.pane_by_id(&target.pane_id)
    }

    pub fn find_focused_pane(&mut self) {
        // Query tmux directly for the active pane, not through `repo_groups`
        // which only contains agent panes. This allows activity/git info to
        // be displayed even when the focused pane has no agent running.
        // When the sidebar has focus, pick_active_pane names the window's
        // last active pane, so a window switched to with the sidebar active
        // still marks its agent. With no such pane it returns None — preserve
        // the previously focused pane so bottom panel data stays stable —
        // unless that pane lives in another window, where it would mark
        // an agent the user is not looking at.
        let panes = tmux::query_active_window_panes();
        if let Some((id, _)) = tmux::pick_active_pane(&self.tmux_pane, &panes) {
            self.adopt_focused_pane(id);
        } else if self
            .focus_state
            .focused_pane_id
            .as_deref()
            .is_some_and(|prev| tmux::is_stale_focus(prev, &panes))
        {
            self.focus_state.focused_pane_id = None;
        }
    }

    /// The cursor and the tmux focus are one agent, so a focus change from
    /// outside the sidebar — a click on an agent pane, a tmux navigation
    /// key — takes the cursor with it: the marked row is the pane the user
    /// just entered, and one click is enough. The row itself follows from
    /// `cursor_pane_id` the next time the targets are rebuilt.
    ///
    /// This only ever runs for a pane tmux reports as focused; while the
    /// sidebar holds the keyboard [`tmux::find_active_pane`] reports
    /// the pane that held it before, which is no focus change, so browsing
    /// with `j`/`k` keeps the cursor it moved.
    fn adopt_focused_pane(&mut self, pane_id: String) {
        if self.focus_state.focused_pane_id.as_deref() != Some(pane_id.as_str()) {
            self.global.cursor_pane_id = Some(pane_id.clone());
        }
        self.focus_state.focused_pane_id = Some(pane_id);
    }

    /// Put the cursor on `row` and name the pane it lands on, so the row
    /// and the shared cursor identity move together. Every cursor move
    /// goes through here — a direct row write would leave the two
    /// behind each other, and the next rebuild would resolve the row
    /// back onto the stale pane.
    pub fn select_row(&mut self, row: usize) {
        self.global.selected_pane_row = row;
        self.global.cursor_pane_id = self
            .layout
            .pane_row_targets
            .get(row)
            .map(|target| target.pane_id.clone());
    }

    /// Move agent selection. Returns true if moved, false if at boundary.
    pub fn move_pane_selection(&mut self, delta: isize) -> bool {
        if self.layout.pane_row_targets.is_empty() {
            return false;
        }
        let len = self.layout.pane_row_targets.len() as isize;
        let next = self.global.selected_pane_row as isize + delta;
        if next >= 0 && next < len {
            self.select_row(next as usize);
            true
        } else {
            false
        }
    }

    pub fn activate_selected_pane(&mut self) {
        if let Some(target_pane_id) = self
            .layout
            .pane_row_targets
            .get(self.global.selected_pane_row)
            .map(|target| target.pane_id.clone())
        {
            // Update the sidebar immediately so the active marker and
            // repo header highlight move without waiting for the next
            // periodic tmux refresh — but only once tmux took the jump.
            if tmux::select_pane(&target_pane_id) {
                self.focus_state.focused_pane_id = Some(target_pane_id.clone());
            }

            // The jump may land on a window that has no sidebar of its own
            // (e.g. with @sidebar_auto_create off). Summon one there so the
            // sidebar "follows" the jump instead of disappearing. create-only
            // is a no-op when that window already has a sidebar.
            let window_id = tmux::display_message(&target_pane_id, "#{window_id}");
            if !window_id.is_empty() {
                let pane_path = tmux::display_message(&target_pane_id, "#{pane_current_path}");
                let pane_path = if pane_path.is_empty() {
                    "~".to_string()
                } else {
                    pane_path
                };
                crate::cli::toggle::cmd_toggle(&[
                    "--create-only".to_string(),
                    window_id,
                    pane_path,
                ]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_default_is_panes() {
        assert_eq!(Focus::default(), Focus::Panes);
    }

    #[test]
    fn focus_state_new_has_expected_initial_values() {
        let state = FocusState::new();
        assert!(!state.sidebar_focused);
        assert_eq!(state.focus, Focus::Panes);
        assert!(state.focused_pane_id.is_none());
    }

    #[test]
    fn focus_state_default_delegates_to_new() {
        let state = FocusState::default();
        assert!(!state.sidebar_focused);
        assert_eq!(state.focus, Focus::Panes);
        assert!(state.focused_pane_id.is_none());
    }

    // ─── AppState focus accessors ────────────────────────────────────

    use crate::group::{PaneGitInfo, RepoGroup};
    use crate::state::layout::RowTarget;
    use crate::tmux::{AgentType, PaneInfo, PaneStatus, PermissionMode, WorktreeMetadata};

    fn test_pane(id: &str) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            pane_active: false,
            status: PaneStatus::Running,
            attention: false,
            agent: AgentType::Claude,
            path: "/tmp".into(),
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
    fn pane_by_id_searches_all_groups() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "alpha".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())],
            },
            RepoGroup {
                name: "beta".into(),
                has_focus: false,
                panes: vec![(test_pane("%42"), PaneGitInfo::default())],
            },
        ];

        assert!(state.pane_by_id("%1").is_some());
        assert!(state.pane_by_id("%42").is_some());
        assert!(state.pane_by_id("%does-not-exist").is_none());
    }

    #[test]
    fn selected_pane_tracks_pane_row_targets() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![RepoGroup {
            name: "alpha".into(),
            has_focus: true,
            panes: vec![
                (test_pane("%1"), PaneGitInfo::default()),
                (test_pane("%2"), PaneGitInfo::default()),
            ],
        }];
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
        ];

        state.global.selected_pane_row = 0;
        assert_eq!(
            state.selected_pane().map(|p| p.pane_id.as_str()),
            Some("%1")
        );
        state.global.selected_pane_row = 1;
        assert_eq!(
            state.selected_pane().map(|p| p.pane_id.as_str()),
            Some("%2")
        );
        // Out-of-range cursor → None (not a panic).
        state.global.selected_pane_row = 99;
        assert!(state.selected_pane().is_none());
    }

    #[test]
    fn adopt_focused_pane_moves_the_cursor_onto_the_focused_pane() {
        let mut state = AppState::new("%99".into());
        state.global.cursor_pane_id = Some("%1".into());

        state.adopt_focused_pane("%3".into());

        assert_eq!(state.focus_state.focused_pane_id.as_deref(), Some("%3"));
        assert_eq!(
            state.global.cursor_pane_id.as_deref(),
            Some("%3"),
            "the cursor and the tmux focus are one agent"
        );
    }

    #[test]
    fn adopt_focused_pane_keeps_its_cursor_for_the_same_pane() {
        let mut state = AppState::new("%99".into());
        state.focus_state.focused_pane_id = Some("%2".into());
        state.global.cursor_pane_id = Some("%2".into());

        state.adopt_focused_pane("%2".into());

        assert_eq!(state.global.cursor_pane_id.as_deref(), Some("%2"));
    }

    #[test]
    fn move_pane_selection_stays_within_bounds() {
        let mut state = AppState::new("%99".into());
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
        ];
        state.global.selected_pane_row = 0;
        assert!(!state.move_pane_selection(-1));
        assert!(state.move_pane_selection(1));
        assert_eq!(state.global.selected_pane_row, 1);
        assert!(!state.move_pane_selection(5));
    }
}
