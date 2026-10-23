use super::{AppState, BottomTab};

impl AppState {
    pub fn next_bottom_tab(&mut self) {
        let next = self.global.bottom_tab.next();
        self.global.set_bottom_tab(next);
    }

    /// Handle mouse click on the bottom panel tab header.
    /// Tab title layout: "╭ Activity│Tasks│Git ──────╮" — col is relative
    /// to the terminal. The `╭ ` prefix occupies cols 0-1, so the title
    /// text starts at col 2: "Activity" spans cols 2..9, "│" at col 10,
    /// "Tasks" spans cols 11..15, "│" at col 16, "Git" spans cols 17..19.
    pub fn handle_bottom_tab_click(&mut self, col: u16) {
        // Offset by 1 for the left border character
        let x = col.saturating_sub(1) as usize;
        // "Activity" = 8 chars (1..8), "│" = 1 char (9), "Tasks" = 5
        // chars (10..14), "│" = 1 char (15), "Git" = 3 chars (16..18)
        let selected = if x < 9 {
            Some(BottomTab::Activity)
        } else if (10..15).contains(&x) {
            Some(BottomTab::Tasks)
        } else if (16..19).contains(&x) {
            Some(BottomTab::GitStatus)
        } else {
            // Outside every title hitbox the click must not write the
            // shared value at all — there is no tab change to save.
            None
        };
        let Some(tab) = selected else { return };
        self.global.set_bottom_tab(tab);
    }

    pub fn scroll_bottom(&mut self, delta: isize) {
        match self.global.bottom_tab {
            BottomTab::Activity => self.activity.scroll.scroll(delta),
            BottomTab::Tasks => self.tasks.scroll.scroll(delta),
            BottomTab::GitStatus => self.scrolls.git.scroll(delta),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::{PaneGitInfo, RepoGroup};
    use crate::state::ScrollState;
    use crate::tmux::{AgentType, PaneInfo, PaneStatus, PermissionMode, WorktreeMetadata};

    fn test_pane(id: &str) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            pane_active: true,
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

    fn agent_group(pane_id: &str) -> RepoGroup {
        RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![(test_pane(pane_id), PaneGitInfo::default())],
        }
    }

    fn state_with_groups(repo_groups: Vec<RepoGroup>, focused_pane_id: Option<&str>) -> AppState {
        let mut state = AppState::new("%99".into());
        state.repo_groups = repo_groups;
        state.focus_state.focused_pane_id = focused_pane_id.map(str::to_string);
        state
    }

    // ─── focusing and lifecycle never move the shared tab ───────
    //
    // The tab moves only on an explicit selection (cycle key, tab-title
    // click, peer adoption): the tick pipeline carries no tab logic, so
    // the spec scenarios under "focusing a different agent does not
    // change the bottom tab" pin this at the same plane the deleted
    // auto-switch used to occupy, minus the pipeline call itself: the
    // helper below runs only the activity/task reads the tick shares,
    // and there is no tab code left for it to execute.
    fn run_tab_reads(state: &mut AppState) {
        state.refresh_activity_log();
        state.refresh_pane_tasks();
        state.refresh_task_progress();
    }

    #[test]
    fn focusing_a_shell_pane_keeps_the_tab() {
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Tasks;

        // The user focuses a pane with no agent running and the next
        // tick runs.
        state.focus_state.focused_pane_id = Some("%5".into());
        run_tab_reads(&mut state);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "focusing a shell pane must keep the selected tab"
        );
    }

    #[test]
    fn focusing_another_agent_keeps_the_tab() {
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Tasks;

        // A second agent pane appears and the user focuses it.
        let mut group = agent_group("%1");
        group.panes.push((test_pane("%2"), PaneGitInfo::default()));
        state.repo_groups = vec![group];
        state.focus_state.focused_pane_id = Some("%2".into());
        run_tab_reads(&mut state);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "focusing a different agent pane must keep the selected tab"
        );
    }

    #[test]
    fn an_agent_starting_in_the_focused_pane_keeps_the_tab() {
        let mut state = state_with_groups(vec![], Some("%1"));
        state.global.bottom_tab = BottomTab::GitStatus;

        // An agent starts in the pane the user is already focused on —
        // no focus change happens, so the tick must not touch the tab.
        state.repo_groups = vec![agent_group("%1")];
        run_tab_reads(&mut state);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "an agent starting in the focused pane must keep the selected tab"
        );
    }

    #[test]
    fn a_pane_relaunch_does_not_change_the_tab() {
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Tasks;

        // The agent's pane closes...
        state.repo_groups = vec![];
        run_tab_reads(&mut state);
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "closing the agent must not move the shared tab"
        );

        // ...and relaunches in the same pane. With no per-pane tab
        // memory there is nothing to reset: the shared tab is untouched.
        state.repo_groups = vec![agent_group("%1")];
        run_tab_reads(&mut state);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "a relaunched pane must not move the shared tab"
        );
    }

    // ─── three-way cycle + click hitboxes ───────────────────────

    #[test]
    fn click_on_each_title_selects_its_tab() {
        // Title layout "╭ Activity│Tasks│Git …─╮": x = clicked col − 1.
        // One click into each of the three hitbox ranges.
        let mut state = AppState::new("%99".into());

        state.handle_bottom_tab_click(3); // "Activity"
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);

        state.handle_bottom_tab_click(13); // "Tasks"
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);

        state.handle_bottom_tab_click(18); // "Git"
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
    }

    #[test]
    fn click_between_or_after_the_titles_changes_nothing() {
        // A click on a separator column or beyond the title text must
        // not switch tabs — only the drawn titles are clickable.
        for col in [10u16, 16, 20, 26] {
            let mut state = AppState::new("%99".into());
            state.global.bottom_tab = BottomTab::Tasks;
            state.handle_bottom_tab_click(col);
            assert_eq!(
                state.global.bottom_tab,
                BottomTab::Tasks,
                "col {col} sits outside a title hitbox"
            );
        }
    }

    #[test]
    fn scroll_bottom_tasks_arm_keeps_the_other_offsets() {
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Tasks;
        state.tasks.scroll = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 3,
        };
        state.activity.scroll.offset = 4;
        state.scrolls.git.offset = 5;

        state.scroll_bottom(1);
        assert_eq!(state.tasks.scroll.offset, 1);
        assert_eq!(state.activity.scroll.offset, 4);
        assert_eq!(state.scrolls.git.offset, 5);

        state.scroll_bottom(-2);
        assert_eq!(state.tasks.scroll.offset, 0);
    }
}
