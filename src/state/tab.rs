use super::{AppState, BottomTab, Focus};
use crate::tmux::AgentType;

/// The two tab panes of a split strip. Pane A carries the Tools group
/// (Activity, Git), pane B the Tasks group (Tasks, Thought). A one-pane
/// strip has no split panes: its tab helpers stay on the shared
/// selection (`shown_bottom_tabs` / `displayed_bottom_tab`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPane {
    A,
    B,
}

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

    /// The focused pane's identity for a tab bar with room to label
    /// itself: its pane id and, when the pane carries one, its own
    /// name. `None` when no focused pane resolves. The id is the part
    /// a cramped bar keeps whole, the name the part it truncates, so
    /// the two come back apart rather than pre-joined.
    pub fn focused_pane_identity(&self) -> Option<(&str, Option<&str>)> {
        let pane = self.pane_by_id(self.focus_state.focused_pane_id.as_deref()?)?;
        Some((pane.pane_id.as_str(), pane.pane_name.as_deref()))
    }

    /// The tabs the rule shows for the focused pane: Tools, then
    /// Thought while the focused pane runs pi, then Tasks and Git.
    /// The cycle steps this list and the rule's labels are built from
    /// it, so a hidden tab has no cycle step and no drawn label to
    /// click.
    pub fn shown_bottom_tabs(&self) -> Vec<BottomTab> {
        let mut tabs = vec![BottomTab::Activity];
        if self.focused_pane_is_pi() {
            tabs.push(BottomTab::Thought);
        }
        tabs.push(BottomTab::Tasks);
        tabs.push(BottomTab::GitStatus);
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

    /// The tabs the split pane's bar shows: pane A's fixed Tools group
    /// (Tools, then Git), pane B's fixed Tasks group (Tasks, then
    /// Thought while the focused pane runs pi). The cycle steps this
    /// list and the split bar's labels are built from it, so a hidden
    /// tab has no cycle step and no drawn glyph to click.
    pub fn shown_split_tabs(&self, pane: SplitPane) -> Vec<BottomTab> {
        match pane {
            SplitPane::A => vec![BottomTab::Activity, BottomTab::GitStatus],
            SplitPane::B => {
                let mut tabs = vec![BottomTab::Tasks];
                if self.focused_pane_is_pi() {
                    tabs.push(BottomTab::Thought);
                }
                tabs
            }
        }
    }

    /// The tab the split pane draws: pane A's stored selection, or
    /// pane B's stored selection except that Thought displays as Tasks
    /// when the focused pane does not run pi. Only an explicit
    /// selection writes the stored value, so a focus change never
    /// touches `@sidebar_split_tab_b` — a peer window that still
    /// focuses a pi pane keeps its Thought tab.
    pub fn displayed_split_tab(&self, pane: SplitPane) -> BottomTab {
        match pane {
            SplitPane::A => self.global.split_tab_a,
            SplitPane::B => match self.global.split_tab_b {
                BottomTab::Thought if !self.focused_pane_is_pi() => BottomTab::Tasks,
                tab => tab,
            },
        }
    }

    /// Cycle the given split pane's tab through its own shown group
    /// and write only that pane's option. The cycle steps through the
    /// shown tabs from the displayed tab, the way the one-pane cycle
    /// does — from a stored Thought hidden behind the Tasks fallback
    /// the next selection is Tasks, which replaces the stored value.
    pub fn next_split_tab(&mut self, pane: SplitPane) {
        let shown = self.shown_split_tabs(pane);
        let current = self.displayed_split_tab(pane);
        let next = shown
            .iter()
            .position(|tab| *tab == current)
            .map_or(shown[0], |idx| shown[(idx + 1) % shown.len()]);
        match pane {
            SplitPane::A => self.global.set_split_tab_a(next),
            SplitPane::B => self.global.set_split_tab_b(next),
        }
    }

    /// Handle a click on a split pane's tab bar. `col` is the terminal
    /// column. The hitboxes are the label columns the last frame
    /// actually drew (`FrameLayout::bottom_tab_targets`), filtered to
    /// the clicked pane's own group: the groups are disjoint, so a
    /// glyph's tab names its pane, and a column under the other pane's
    /// glyph — or under no glyph at all — selects nothing.
    pub fn handle_split_tab_click(&mut self, pane: SplitPane, col: u16) {
        let group = self.shown_split_tabs(pane);
        let Some(tab) = self
            .layout
            .bottom_tab_targets
            .iter()
            .filter(|target| group.contains(&target.tab))
            .find(|target| col >= target.start && col < target.end)
            .map(|target| target.tab)
        else {
            return;
        };
        match pane {
            SplitPane::A => self.global.set_split_tab_a(tab),
            SplitPane::B => self.global.set_split_tab_b(tab),
        }
    }

    /// Handle a click on the bottom panel's rule (its first drawn row).
    /// `col` is the terminal column. The hitboxes are the label columns
    /// the last frame actually drew (`FrameLayout::bottom_tab_targets`),
    /// so the `─ ` prefix, the dash fill, and a label the rule
    /// truncated away are all dead — a click outside a drawn label must
    /// not write the shared value at all (decision 3).
    pub fn handle_bottom_tab_click(&mut self, col: u16) {
        let Some(tab) = self
            .layout
            .bottom_tab_targets
            .iter()
            .find(|target| col >= target.start && col < target.end)
            .map(|target| target.tab)
        else {
            return;
        };
        self.global.set_bottom_tab(tab);
    }

    pub fn scroll_bottom(&mut self, delta: isize) {
        self.scroll_tab(self.displayed_bottom_tab(), delta);
    }

    /// Scroll the given split pane's displayed tab. The groups are
    /// disjoint, so each tab's scroll state belongs to one pane alone:
    /// pane A's scroll never touches pane B's tab and the other way
    /// round.
    pub fn scroll_split_tab(&mut self, pane: SplitPane, delta: isize) {
        self.scroll_tab(self.displayed_split_tab(pane), delta);
    }

    /// The per-tab scroll arm both scroll routers share: one scroll
    /// state per tab, with the Thought follow-the-end rule.
    fn scroll_tab(&mut self, tab: BottomTab, delta: isize) {
        match tab {
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

    /// Whether the drawn strip is a split one: pane B's panel has
    /// columns the last frame published. A one-pane strip — or a
    /// column, where no strip is drawn at all — reports false.
    pub fn split_strip_drawn(&self) -> bool {
        self.layout
            .strip
            .is_some_and(|strip| strip.tabs_b.is_some())
    }

    /// The tab the drawn feed panel shows: on a split strip the feed
    /// follows pane B's displayed tab — the Thought tab lives in pane
    /// B's group — and on a one-pane strip or a column the shared
    /// displayed tab. The renderer's `draw_split_strip` passes each
    /// pane its own tab, so this is the one answer the drawn frame and
    /// the feed refresher share.
    pub fn drawn_feed_tab(&self) -> BottomTab {
        if self.split_strip_drawn() {
            self.displayed_split_tab(SplitPane::B)
        } else {
            self.displayed_bottom_tab()
        }
    }

    /// Demote a `TabPaneB` focus the drawn strip cannot serve: when
    /// the frame drew no pane B (the strip narrowed below the split
    /// threshold), the one tab panel takes the focus back (design D7).
    /// `draw_strip` calls this right after it records the drawn strip,
    /// before the fill reads the focus, and the strip key map's
    /// key-time guard calls the same method, so both paths share one
    /// decision.
    pub fn demote_tab_pane_b(&mut self) {
        if self.focus_state.focus == Focus::TabPaneB && !self.split_strip_drawn() {
            self.focus_state.focus = Focus::ActivityLog;
        }
    }

    /// Whether the background git poller should run. In a split strip
    /// the flag follows the tab pane A displays (design D8) — pane B's
    /// selection never moves it; in a one-pane strip or a column it is
    /// the displayed shared tab, as before. A split strip is a drawn
    /// one, so the flag reads the geometry the last frame published —
    /// the same rects the click and scroll routers hit-test.
    pub fn git_poll_active(&self) -> bool {
        if self.split_strip_drawn() {
            self.displayed_split_tab(SplitPane::A).is_git_status()
        } else {
            self.displayed_bottom_tab().is_git_status()
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
                BottomTab::Thought,
                BottomTab::Tasks,
                BottomTab::GitStatus
            ]
        );
    }

    #[test]
    fn the_tab_bar_identity_is_the_pane_id_then_its_name() {
        // A named pane labels the strip's tab bar with the id first, so
        // a cramping bar always keeps the identity and cuts the name.
        let mut named = test_pane("%1");
        named.pane_name = Some("my-task".into());
        let state = state_with_groups(vec![group_with(vec![named])], Some("%1"));
        assert_eq!(state.focused_pane_identity(), Some(("%1", Some("my-task"))));

        // An unnamed pane labels itself with the id alone.
        let state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        assert_eq!(state.focused_pane_identity(), Some(("%1", None)));

        // No focused pane — or one that no longer resolves — labels
        // nothing.
        let state = state_with_groups(vec![agent_group("%1")], Some("%5"));
        assert_eq!(state.focused_pane_identity(), None);
        let state = state_with_groups(vec![agent_group("%1")], None);
        assert_eq!(state.focused_pane_identity(), None);
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
        // Spec "Keyboard cycles four tabs for a pi pane": Thought sits
        // second, between Tools and Tasks, and Git wraps back to
        // Tools.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));

        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
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
        // Rule layout at 40 columns with a 9-cell icon run: 15 pad
        // columns, then three glyphs on cols 15, 19 and 23 between the
        // " │ " separators. One click into each of the three drawn
        // hitbox ranges, including the first glyph's only column.
        let mut state = AppState::new("%99".into());
        crate::ui::bottom::draw_tab_targets(&mut state, 40);

        state.handle_bottom_tab_click(15); // the Tools glyph
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);

        state.handle_bottom_tab_click(19); // the Tasks glyph
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);

        state.handle_bottom_tab_click(23); // the Git glyph
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
    }

    #[test]
    fn click_between_or_after_the_titles_changes_nothing() {
        // A click on a separator column or beyond the title text must
        // not switch tabs — only the drawn glyphs are clickable. At 40
        // columns the glyph run spans cols 15-23 with the │ of each
        // " │ " separator on cols 17 and 21, fill from 24 on.
        for col in [17u16, 21, 25, 30] {
            let mut state = AppState::new("%99".into());
            crate::ui::bottom::draw_tab_targets(&mut state, 40);
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
        // The centered run starts at column 15 of 40: a click on the
        // left pad, on a col-0 event, changes nothing.
        let mut state = AppState::new("%99".into());
        crate::ui::bottom::draw_tab_targets(&mut state, 40);
        state.global.bottom_tab = BottomTab::Tasks;

        for col in [0u16, 1, 14] {
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
        // 40-column rule reads 13 pad cols then a 13-cell run, so the
        // Thought glyph sits alone on col 17 between its separators.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));
        crate::ui::bottom::draw_tab_targets(&mut state, 40);

        state.handle_bottom_tab_click(17); // the Thought glyph
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);
    }

    #[test]
    fn a_click_where_a_hidden_thought_label_would_be_changes_nothing() {
        // Spec "a click where a hidden label would be": for a non-pi
        // pane the Thought glyph is not drawn — the three-glyph run
        // spans cols 15-23, so col 17, where the pi rule would draw
        // Thought, is the │ of the first separator, and col 24 is
        // fill right of Git: neither the tab nor the split may change
        // there.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        crate::ui::bottom::draw_tab_targets(&mut state, 40);
        state.global.bottom_tab = BottomTab::Tasks;
        state.global.bottom_percent = 30;

        for col in [17u16, 24] {
            state.handle_bottom_tab_click(col);
            assert_eq!(
                state.global.bottom_tab,
                BottomTab::Tasks,
                "col {col} holds no label for a non-pi pane"
            );
        }
        assert_eq!(
            state.global.bottom_percent, 30,
            "a click outside every label must not resize the split"
        );
    }

    #[test]
    fn a_click_on_the_fill_past_a_truncated_label_changes_nothing() {
        // The hitboxes are the drawn columns: on a 6-column pi-pane
        // rule the 13-cell run overflows — Activity draws on col 0,
        // the first separator on cols 1-3, Thought on col 4, and the
        // second separator truncates to one pad column while Tasks and
        // Git collapse to empty ranges at col 6, where the full-width
        // layout would still have claimed them. Clicks on separators,
        // the empty ranges and the fill select nothing.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));
        crate::ui::bottom::draw_tab_targets(&mut state, 6);
        state.global.bottom_tab = BottomTab::Activity;

        state.handle_bottom_tab_click(4); // the Thought glyph, still drawn
        assert_eq!(state.global.bottom_tab, BottomTab::Thought);

        state.global.bottom_tab = BottomTab::Activity;
        for col in [5u16, 6, 8] {
            state.handle_bottom_tab_click(col);
            assert_eq!(
                state.global.bottom_tab,
                BottomTab::Activity,
                "col {col} is fill or a truncated-off glyph, not a hitbox"
            );
        }
    }

    // ─── the split panes' shown tabs and displayed tabs ───────────

    #[test]
    fn pane_b_shows_thought_only_for_a_pi_focus() {
        // Spec "the tabs for a pi pane" / "the tabs for a Claude pane":
        // pane A always offers its fixed Tools group; pane B's group
        // gains Thought only while the focused pane runs pi.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        assert_eq!(
            state.shown_split_tabs(SplitPane::A),
            vec![BottomTab::Activity, BottomTab::GitStatus]
        );
        assert_eq!(state.shown_split_tabs(SplitPane::B), vec![BottomTab::Tasks]);

        state.repo_groups = vec![group_with(vec![test_pane_with_agent("%1", AgentType::Pi)])];
        assert_eq!(
            state.shown_split_tabs(SplitPane::B),
            vec![BottomTab::Tasks, BottomTab::Thought]
        );
    }

    #[test]
    fn a_stored_thought_in_pane_b_displays_as_tasks_for_a_claude_focus() {
        // Spec "the focus moves to a Claude pane": the tab pane B draws
        // is the Tasks fallback, while the stored value keeps holding
        // `thought` — the fallback displays without a write.
        let mut state = state_with_groups(vec![agent_group("%1")], Some("%1"));
        state.global.split_tab_b = BottomTab::Thought;

        assert_eq!(state.displayed_split_tab(SplitPane::B), BottomTab::Tasks);
        assert_eq!(
            state.global.split_tab_b,
            BottomTab::Thought,
            "the fallback displays without moving the stored tab"
        );
    }

    #[test]
    fn a_focus_change_back_to_a_pi_pane_redisplays_pane_bs_thought() {
        // Spec "the focus returns to a pi pane": the display is derived
        // from the focus, so pane B shows the stored Thought again the
        // moment a pi pane is focused — no selection needed.
        let claude = test_pane("%1");
        let pi = test_pane_with_agent("%2", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![claude, pi])], Some("%1"));
        state.global.split_tab_b = BottomTab::Thought;
        assert_eq!(state.displayed_split_tab(SplitPane::B), BottomTab::Tasks);

        state.focus_state.focused_pane_id = Some("%2".into());
        assert_eq!(state.displayed_split_tab(SplitPane::B), BottomTab::Thought);
        assert_eq!(
            state.global.split_tab_b,
            BottomTab::Thought,
            "the focus change must not write @sidebar_split_tab_b"
        );
    }

    // ─── the per-pane cycle and click ─────────────────────────────

    #[test]
    fn pane_bs_cycle_leaves_pane_a_and_the_shared_tab() {
        // Spec "the cycle key in pane B": pane B's cycle steps its own
        // group and writes only its own option — pane A's stored tab
        // and the one-pane `@sidebar_bottom_tab` never move.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));
        state.global.split_tab_a = BottomTab::GitStatus;
        state.global.split_tab_b = BottomTab::Tasks;
        state.global.bottom_tab = BottomTab::Activity;

        state.next_split_tab(SplitPane::B);

        assert_eq!(state.global.split_tab_b, BottomTab::Thought);
        assert_eq!(state.global.split_tab_a, BottomTab::GitStatus);
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
    }

    #[test]
    fn a_pane_a_click_leaves_pane_b_unchanged() {
        // Spec "a selection in pane A": a click on pane A's bar selects
        // in pane A's option alone. The hitboxes are the drawn columns
        // a split frame publishes, pane A's glyphs and pane B's side by
        // side in the one `bottom_tab_targets` list; the group filter
        // keeps a click on a pane B column from moving pane A too. The
        // focused pane runs pi so pane B's group shows Thought.
        let pi = test_pane_with_agent("%1", AgentType::Pi);
        let mut state = state_with_groups(vec![group_with(vec![pi])], Some("%1"));
        state.layout.bottom_tab_targets = vec![
            crate::state::BottomTabTarget {
                start: 10,
                end: 14,
                tab: BottomTab::Activity,
            },
            crate::state::BottomTabTarget {
                start: 20,
                end: 24,
                tab: BottomTab::GitStatus,
            },
            crate::state::BottomTabTarget {
                start: 40,
                end: 44,
                tab: BottomTab::Tasks,
            },
            crate::state::BottomTabTarget {
                start: 50,
                end: 54,
                tab: BottomTab::Thought,
            },
        ];
        state.global.split_tab_a = BottomTab::Activity;
        state.global.split_tab_b = BottomTab::Tasks;

        state.handle_split_tab_click(SplitPane::A, 20); // pane A's Git glyph
        assert_eq!(state.global.split_tab_a, BottomTab::GitStatus);
        assert_eq!(
            state.global.split_tab_b,
            BottomTab::Tasks,
            "a pane A click must not move pane B's tab"
        );

        state.handle_split_tab_click(SplitPane::B, 50); // pane B's Thought glyph
        assert_eq!(state.global.split_tab_b, BottomTab::Thought);
        assert_eq!(
            state.global.split_tab_a,
            BottomTab::GitStatus,
            "a pane B click must not move pane A's tab"
        );
    }

    #[test]
    fn a_click_on_the_other_panes_group_selects_nothing() {
        // The groups are disjoint: a click routed to pane B that lands
        // on a pane A glyph — a col-20 Git hitbox — writes neither
        // option, and a click outside every glyph writes nothing either.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_tab_targets = vec![crate::state::BottomTabTarget {
            start: 20,
            end: 24,
            tab: BottomTab::GitStatus,
        }];
        state.global.split_tab_a = BottomTab::Activity;
        state.global.split_tab_b = BottomTab::Tasks;

        state.handle_split_tab_click(SplitPane::B, 20);
        state.handle_split_tab_click(SplitPane::B, 30);
        assert_eq!(state.global.split_tab_a, BottomTab::Activity);
        assert_eq!(state.global.split_tab_b, BottomTab::Tasks);
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
