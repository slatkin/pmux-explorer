use super::{AppState, BottomTab};
use crate::tmux::AgentType;

impl AppState {
    /// Whether the focused pane runs pi. The focused pane is looked up
    /// by id across the groups, the way the activity and task reads do;
    /// no focused pane (or a vanished one) is not pi.
    fn focused_pane_is_pi(&self) -> bool {
        self.focus_state
            .focused_pane_id
            .as_deref()
            .and_then(|id| self.pane_by_id(id))
            .is_some_and(|pane| pane.agent == AgentType::Pi)
    }

    /// The tabs the rule shows for the focused pane: Activity, Tasks
    /// and Git always, plus Thought while the focused pane runs pi.
    /// The cycle and the click hitboxes are both derived from this
    /// list, so a hidden tab has no cycle step and no hitbox.
    pub fn shown_bottom_tabs(&self) -> Vec<BottomTab> {
        let mut tabs = vec![BottomTab::Activity, BottomTab::Tasks, BottomTab::GitStatus];
        if self.focused_pane_is_pi() {
            tabs.push(BottomTab::Thought);
        }
        tabs
    }

    /// The tab the bottom panel draws: the stored shared selection,
    /// except that Thought displays as Activity when the focused pane
    /// does not run pi. Only an explicit selection writes the stored
    /// value, so a focus change never touches `@sidebar_bottom_tab` —
    /// a peer window that still focuses a pi pane keeps its Thought
    /// tab.
    pub fn displayed_bottom_tab(&self) -> BottomTab {
        match self.global.bottom_tab {
            BottomTab::Thought if !self.focused_pane_is_pi() => BottomTab::Activity,
            tab => tab,
        }
    }

    pub fn next_bottom_tab(&mut self) {
        // The cycle steps through the shown tabs from the displayed
        // tab, so from a stored Thought hidden behind the Activity
        // fallback the next selection is Tasks.
        let shown = self.shown_bottom_tabs();
        let current = self.displayed_bottom_tab();
        let next = shown
            .iter()
            .position(|tab| *tab == current)
            .map_or(shown[0], |idx| shown[(idx + 1) % shown.len()]);
        self.global.set_bottom_tab(next);
    }

    /// Handle a click on the bottom panel's rule (its first drawn row).
    /// Rule layout: "─ Activity│Tasks│Git ──────" — col is relative to
    /// the terminal. The `─ ` prefix occupies cols 0-1, so the first
    /// label starts at col 2; one `│` separator column follows each
    /// label. The hitboxes are computed from the shown tabs and the
    /// same label strings `build_tab_title` draws, so a label hidden
    /// because its tab is not shown has no hitbox and the columns can
    /// never drift from the drawing.
    pub fn handle_bottom_tab_click(&mut self, col: u16) {
        // Only the drawn labels are clickable. The `─ ` prefix cols 0-1
        // and the dash fill right of the last label change nothing — a
        // click outside a label must not write the shared value at all
        // (decision 3).
        let x = col.saturating_sub(1) as usize;
        let mut cursor = 1usize; // the first label starts right after "─ "
        let selected = self.shown_bottom_tabs().into_iter().find(|tab| {
            let width = tab.label().chars().count();
            let hit = cursor..cursor + width;
            cursor += width + 1; // label, then the separator column
            hit.contains(&x)
        });
        let Some(tab) = selected else { return };
        self.global.set_bottom_tab(tab);
    }

    pub fn scroll_bottom(&mut self, delta: isize) {
        match self.displayed_bottom_tab() {
            BottomTab::Activity => self.activity.scroll.scroll(delta),
            BottomTab::Tasks => self.tasks.scroll.scroll(delta),
            BottomTab::GitStatus => self.scrolls.git.scroll(delta),
            BottomTab::Thought => {
                self.scrolls.thought.scroll(delta);
                // Follow-the-end (design D6): a scroll up suspends the
                // follow, a scroll that reaches the bottom re-arms it.
                // While the flag stands the draw itself moves the offset
                // to the last page.
                if delta < 0 {
                    self.thought.follow = false;
                } else if self.scrolls.thought.at_end() {
                    self.thought.follow = true;
                }
            }
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

    fn test_pane_with_agent(id: &str, agent: AgentType) -> PaneInfo {
        let mut pane = test_pane(id);
        pane.agent = agent;
        pane
    }

    fn agent_group(pane_id: &str) -> RepoGroup {
        RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![(test_pane(pane_id), PaneGitInfo::default())],
        }
    }

    fn group_with(panes: Vec<PaneInfo>) -> RepoGroup {
        RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: panes
                .into_iter()
                .map(|pane| (pane, PaneGitInfo::default()))
                .collect(),
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

    // ─── the shown tabs and the displayed tab ─────────────────────

    #[test]
    fn the_thought_tab_is_shown_only_while_the_focused_pane_runs_pi() {
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        assert_eq!(
            state.shown_bottom_tabs(),
            vec![BottomTab::Activity, BottomTab::Tasks, BottomTab::GitStatus]
        );

        state.repo_groups = vec![group_with(vec![test_pane_with_agent("%1", AgentType::Pi)])];
        assert_eq!(
            state.shown_bottom_tabs(),
            vec![
                BottomTab::Activity,
                BottomTab::Tasks,
                BottomTab::GitStatus,
                BottomTab::Thought
            ]
        );
    }

    #[test]
    fn a_stored_thought_displays_as_activity_for_a_non_pi_pane() {
        // Spec "focus moves to a Claude pane": the tab the panel draws
        // is the Activity fallback, while the stored shared value keeps
        // holding `thought`.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Thought;

        assert_eq!(state.displayed_bottom_tab(), BottomTab::Activity);
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Thought,
            "the fallback displays without moving the stored tab"
        );
    }

    #[test]
    fn a_stored_thought_displays_as_thought_for_a_pi_pane() {
        // Spec "focus returns to a pi pane" and "another window keeps
        // Thought": the display is derived from the focus, so a window
        // whose focused pane runs pi shows the stored Thought.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));
        state.global.bottom_tab = BottomTab::Thought;

        assert_eq!(state.displayed_bottom_tab(), BottomTab::Thought);
    }

    #[test]
    fn a_focus_change_displays_the_fallback_and_writes_nothing() {
        // Spec "focus moves to a Claude pane": the tick that follows the
        // focus change carries no tab logic, so the display flips to the
        // fallback while `@sidebar_bottom_tab` is never written — the
        // stored value stays `thought`, and a peer window focusing a pi
        // pane keeps its Thought tab.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi, test_pane("%2")])], Some("%1"));
        state.global.bottom_tab = BottomTab::Thought;
        assert_eq!(state.displayed_bottom_tab(), BottomTab::Thought);

        state.focus_state.focused_pane_id = Some("%2".into());
        run_tab_reads(&mut state);

        assert_eq!(state.displayed_bottom_tab(), BottomTab::Activity);
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Thought,
            "a focus change must not write @sidebar_bottom_tab"
        );

        // The stored Thought displays again the moment a pi pane is
        // focused — no selection needed.
        state.focus_state.focused_pane_id = Some("%1".into());
        assert_eq!(state.displayed_bottom_tab(), BottomTab::Thought);
    }

    // ─── the cycle steps through the shown tabs ───────────────────

    #[test]
    fn the_cycle_covers_four_tabs_for_a_pi_pane() {
        // Spec "Keyboard cycles four tabs for a pi pane": Thought takes
        // its place after Git and wraps back to Activity.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));

        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
    }

    #[test]
    fn the_cycle_from_the_thought_fallback_selects_tasks() {
        // Spec "the cycle key from the fallback": a stored Thought that
        // displays as Activity cycles on from the displayed tab.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Thought;

        state.next_bottom_tab();

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "Tasks follows the Activity fallback in the cycle"
        );
    }

    // ─── three-way cycle + click hitboxes ───────────────────────

    #[test]
    fn click_on_each_title_selects_its_tab() {
        // Rule layout "─ Activity│Tasks│Git …─": x = clicked col − 1.
        // One click into each of the three hitbox ranges, including
        // the first label's leftmost column.
        let mut state = AppState::new("%99".into());

        state.handle_bottom_tab_click(2); // left edge of "Activity"
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);

        state.handle_bottom_tab_click(3); // middle of "Activity"
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
    fn click_on_the_prefix_left_of_activity_changes_nothing() {
        // The `─ ` before the first label is rule, not label: a click
        // on the prefix column or a col-0 event changes nothing.
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Tasks;

        for col in [0u16, 1] {
            state.handle_bottom_tab_click(col);
            assert_eq!(
                state.global.bottom_tab,
                BottomTab::Tasks,
                "col {col} is left of the first label"
            );
        }
    }

    #[test]
    fn a_click_on_the_thought_label_selects_it_for_a_pi_pane() {
        // Spec "a click on the Thought label": with Thought shown the
        // rule reads "─ Activity│Tasks│Git│Thought …", so the Thought
        // label starts at col 21 (8+1+5+1+3+1+1 for the prefix).
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));

        state.handle_bottom_tab_click(21); // left edge of "Thought"
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);

        state.handle_bottom_tab_click(24); // middle of "Thought"
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);
    }

    #[test]
    fn a_click_where_a_hidden_thought_label_would_be_changes_nothing() {
        // Spec "a click where a hidden label would be": for a non-pi
        // pane the columns right of `Git` are dash fill, not a label —
        // neither the tab nor the split may change.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.bottom_tab = BottomTab::Tasks;
        state.global.bottom_percent = 30;

        for col in [21u16, 24] {
            state.handle_bottom_tab_click(col);
            assert_eq!(
                state.global.bottom_tab,
                BottomTab::Tasks,
                "col {col} holds no Thought label for a non-pi pane"
            );
        }
        assert_eq!(
            state.global.bottom_percent, 30,
            "a click outside every label must not resize the split"
        );
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
