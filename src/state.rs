use std::time::Instant;

use crate::cli::toggle::SidebarPosition;
use crate::tmux::SidebarPaneInfo;
use crate::ui::colors::ColorTheme;
use crate::ui::icons::StatusIcons;

mod activity;
mod filter;
mod focus;
mod global;
mod layout;
mod notices;
mod pane_runtime;
mod pet;
mod popout;
mod popup;
mod refresh;
mod scroll;
mod side_switch;
mod tab;
mod tasks;
mod thought;
mod timers;

pub use activity::ActivityState;
pub use filter::{RepoFilter, StatusCounts, StatusFilter};
pub use focus::{Focus, FocusState};
pub use global::GlobalState;
pub use layout::{
    BottomTabTarget, FrameLayout, HyperlinkOverlay, LayoutKind, RepoSpawnTarget, RowTarget,
    SpawnRemoveTarget, SplitDrag, StripLayout,
};
pub use layout::{cols_for_percent, filter_column_layout, strip_layout, strip_layout_for_tab_cols};
pub(crate) use layout::{filter_bar_layout, split_rows};
pub(crate) use notices::debug_forced_display;
pub use notices::{ClaudePluginNotice, NoticesCopyTarget, NoticesMissingHookGroup, NoticesState};
pub use pane_runtime::{PaneRuntimeMap, PaneRuntimeState};
pub use popout::{PanelGeometry, clamp_bottom_height, panel_geometry};
pub use popup::{PopupState, SpawnField};
#[cfg(test)]
pub(crate) use refresh::{TaskProgressDecision, classify_task_progress};
pub use scroll::{ScrollState, ScrollStates};
pub use tasks::TasksState;
pub use thought::ThoughtState;
pub use timers::RefreshTimers;

/// Bottom-panel tabs. The selected tab is shared state: it lives in
/// [`GlobalState`], is written with `set -g @sidebar_bottom_tab`, and
/// changes only through an explicit selection (the manual cycle, a
/// tab-title click) or through the shared value a peer instance
/// selected.
///
/// `Thought` is stored in the shared value like the others, but the
/// rule draws it only while the focused pane runs pi; a stored Thought
/// displays as Activity otherwise (see
/// [`AppState::displayed_bottom_tab`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Activity,
    Tasks,
    GitStatus,
    Thought,
}

impl BottomTab {
    /// Whether this tab drives the background git poller.
    pub fn is_git_status(self) -> bool {
        matches!(self, Self::GitStatus)
    }

    /// The label drawn on the rule and persisted in the
    /// `@sidebar_bottom_tab` tmux option (lower case, see `as_str`).
    /// The values contain no space, so they ride the space-split parse
    /// of the shared reading safely.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Tasks => "tasks",
            Self::GitStatus => "git",
            Self::Thought => "thought",
        }
    }

    /// Parse a tmux-option label into a `BottomTab`. The shared option
    /// carries only what a sidebar wrote, so anything unknown falls
    /// back to the `Activity` default instead of inventing a tab.
    pub fn from_label(s: &str) -> Self {
        match s {
            "tasks" => Self::Tasks,
            "git" => Self::GitStatus,
            "thought" => Self::Thought,
            _ => Self::Activity,
        }
    }

    /// The Nerd Font glyph the rule draws for this tab (Tools ,
    /// Tasks , Git 󰊢, Thought 󰟶). The click hitboxes are computed from
    /// the same strings, so a glyph and its hitbox cannot drift apart.
    /// The persisted option keeps the lowercase words of `as_str`;
    /// only the drawing changes.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Activity => "\u{f0ad}",
            Self::Tasks => "\u{f0ae}",
            Self::GitStatus => "\u{f02a2}",
            Self::Thought => "\u{f07f6}",
        }
    }
}

pub struct AppState {
    pub now: u64,
    pub repo_groups: Vec<crate::group::RepoGroup>,
    /// Sidebar focus + pane focus tracking (sidebar_focused, focus,
    /// focused_pane_id).
    pub focus_state: FocusState,
    /// Transient one-line status banner (message + expiry) for spawn /
    /// remove feedback. Cleared by `take_flash` once the deadline passes.
    pub flash: Option<(String, Instant)>,
    pub spinner_frame: usize,
    /// Frame-scoped render output (pane_row_targets, line_to_row,
    /// repo_button_col, hyperlink_overlays). Rewritten every frame by
    /// the UI layer; consumed by mouse/keyboard handlers before the
    /// next render.
    pub layout: FrameLayout,
    /// Live split drag: a press on the rule (a column) or the divider
    /// (a strip) until its release. While a moved drag is set the
    /// matching panel renders [`SplitDrag::size`] instead of the shared
    /// percent's geometry, and `apply_all` holds that axis's shared
    /// percent back so a tick cannot move the rule or the divider under
    /// the mouse (a column drag blocks `@sidebar_bottom_height`, a
    /// strip drag `@sidebar_tab_width`). Set by a press on the rule or
    /// the divider, consumed by the release (or the stale-drag end on a
    /// new press).
    pub split_drag: Option<SplitDrag>,
    pub activity: ActivityState,
    /// Focused pane's task store for the Tasks tab (refreshed by
    /// `refresh_pane_tasks` with the activity log's mtime check).
    pub tasks: TasksState,
    /// The Thought tab's feed reader state (refreshed by
    /// `refresh_thought_feed` only while the tab is drawn).
    pub thought: ThoughtState,
    pub tmux_pane: String,
    /// The sidebar's own pane state as tmux reported it on the last
    /// refresh: focus, size, window size and the floating flag. The
    /// docked-vs-floating mode is read from this, never remembered
    /// separately.
    pub sidebar_pane: SidebarPaneInfo,
    /// Docked size captured when the sidebar was popped out, restored
    /// by the pop-in (`resize-pane -x` for a column, `-y` for a strip):
    /// the width of a column, the height of a strip. `None` while
    /// docked and when the pane was already floating when the sidebar
    /// started (nothing to restore).
    pub docked_size: Option<u16>,
    /// The edge a popped-out strip was docked at, captured with the
    /// docked size on the way out — a floating pane is no longer at an
    /// edge, so the pop-in cannot read the edge off the pane. `None`
    /// for a column and when the pane was already floating when the
    /// sidebar started; the strip pop-in then falls back to the bare
    /// join.
    pub docked_edge: Option<SidebarPosition>,
    /// Scroll offsets for the agents list and git tab. The Activity
    /// tab's scroll lives in [`ActivityState::scroll`], the Tasks tab's
    /// in [`TasksState::scroll`].
    pub scrolls: ScrollStates,
    pub theme: ColorTheme,
    pub icons: StatusIcons,
    pub git: crate::git::GitData,
    pub pane_states: PaneRuntimeMap,
    /// Periodic-refresh clocks (port scan, filter
    /// debounce, port-scan first-run flag).
    pub timers: RefreshTimers,
    /// Current popup state. At most one popup is open at a time; the enum
    /// variant encodes both which popup is open and its per-popup data.
    pub popup: PopupState,
    /// All fields related to the ⓘ notices button and its popup — the button
    /// click region, cached hook/plugin diagnostics, per-agent copy targets,
    /// and the transient "copied" feedback label.
    pub notices: NoticesState,
    /// Pending OSC 52 clipboard payload. The main loop flushes this to
    /// stdout after the next frame so tmux (with `set-clipboard on`) can
    /// forward it to the upstream terminal's clipboard — covering the
    /// SSH case where `arboard` would only reach the remote machine.
    pub pending_osc52_copy: Option<String>,
    pub pet_state: crate::ui::pet::PetState,
    /// Pet animation X position (character offset from left of bottom panel).
    pub pet_x: u16,
    /// Pet animation frame index (0 = sitting, 1-4 = running/working).
    pub pet_frame: usize,
    /// Working animation tick used to slow the hand motion down a bit.
    pub pet_working_frame_tick: usize,
    /// Walking animation tick used to pace the optional bounce.
    pub pet_walk_tick: usize,
    /// Seed used to randomize walking bounce timing.
    pub pet_walk_seed: usize,
    /// Tick at which the next walk bounce should start.
    pub pet_walk_bounce_next_tick: usize,
    /// Tick until which the current walk bounce stays lifted.
    pub pet_walk_bounce_lift_until: usize,
    pub pet_bob_timer: usize,
    /// Idle animation schedule tick for jump motion within the bob cycle.
    pub pet_idle_jump_tick: usize,
    /// Idle animation schedule tick for blink motion within the bob cycle.
    pub pet_idle_blink_tick: usize,
    /// Idle animation schedule tick for the hand-raise motion within the bob cycle.
    pub pet_idle_wave_tick: usize,
    /// Whether the hand-raise motion is enabled for the current idle cycle.
    pub pet_idle_wave_enabled: bool,
    /// Seed used to reshuffle idle motion timing.
    pub pet_idle_seed: usize,
    /// Current working animation tick, used to pace paper motion.
    pub pet_working_paper_timer: usize,
    /// Tick at which the paper stack should next lift.
    pub pet_working_paper_next_lift_tick: usize,
    /// Tick until which the paper stack stays lifted after a trigger.
    pub pet_working_paper_lift_until: usize,
    /// Horizontal offset applied to the paper stack during a lift.
    pub pet_working_paper_x_offset: u16,
    /// Seed used to reshuffle working paper motion timing.
    pub pet_working_paper_seed: usize,
    /// Update notice shown when a newer GitHub release is available.
    pub version_notice: Option<crate::version::UpdateNotice>,
    /// Shared state across sidebar instances, persisted to tmux global variables.
    pub global: GlobalState,
    /// Whether this sidebar is a full-height column or a full-width
    /// strip. Read once at startup from the `PMUX_SIDEBAR_LAYOUT`
    /// process environment (`split-window -e`, decision 1 of the
    /// `add-strip-layout` design) and never from a pane option: the TUI
    /// starts before `toggle` can write `@pane_sidebar_layout`. A
    /// missing or unknown value gives [`LayoutKind::Column`], so a
    /// sidebar from before the strip layout existed keeps its behavior.
    /// The layout is fixed for the life of the pane.
    pub layout_kind: LayoutKind,
    /// Whether the pet animation is drawn and ticked. Loaded once at startup
    /// from the `@sidebar_pet` tmux option. Defaults to `false`.
    pub pet_enabled: bool,
}

impl AppState {
    pub fn new(tmux_pane: String) -> Self {
        let mut state = Self {
            now: 0,
            repo_groups: vec![],
            focus_state: FocusState::new(),
            flash: None,
            spinner_frame: 0,
            layout: FrameLayout::default(),
            split_drag: None,
            activity: ActivityState::new(),
            tasks: TasksState::new(),
            thought: ThoughtState::from_process(),
            tmux_pane,
            scrolls: ScrollStates::default(),
            theme: ColorTheme::default(),
            icons: StatusIcons::default(),
            git: crate::git::GitData::default(),
            sidebar_pane: SidebarPaneInfo::FALLBACK,
            docked_size: None,
            docked_edge: None,
            pane_states: PaneRuntimeMap::new(),
            timers: RefreshTimers::default(),
            popup: PopupState::None,
            notices: NoticesState::default(),
            pending_osc52_copy: None,
            pet_state: crate::ui::pet::PetState::Idle,
            pet_x: crate::ui::pet::PET_HOME_X,
            pet_frame: 0,
            pet_working_frame_tick: 0,
            pet_walk_tick: 0,
            pet_walk_seed: 1,
            pet_walk_bounce_next_tick: 0,
            pet_walk_bounce_lift_until: 0,
            pet_bob_timer: 0,
            pet_idle_jump_tick: 8,
            pet_idle_blink_tick: 24,
            pet_idle_wave_tick: 0,
            pet_idle_wave_enabled: false,
            pet_idle_seed: 1,
            pet_working_paper_timer: 0,
            pet_working_paper_next_lift_tick: 0,
            pet_working_paper_lift_until: 0,
            pet_working_paper_x_offset: 0,
            pet_working_paper_seed: 1,
            version_notice: None,
            global: GlobalState::new(),
            layout_kind: LayoutKind::Column,
            pet_enabled: false,
        };
        crate::state::pet::reseed_pet_idle_motion(&mut state);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{TaskProgress, TaskStatus};
    use crate::group::{PaneGitInfo, RepoGroup};
    use crate::tmux::{AgentType, PaneInfo, PaneStatus, PermissionMode, WorktreeMetadata};
    use std::fs;

    /// Reset filter click debounce so the next `handle_filter_click` is not
    /// ignored, and mark the sidebar as the active pane: a filter click only
    /// counts once the sidebar already has focus, so that the click which
    /// focuses it cannot select a chip.
    fn reset_filter_debounce(state: &mut AppState) {
        state.timers.last_filter_click =
            std::time::Instant::now() - std::time::Duration::from_millis(200);
        state.sidebar_pane.pane_active = true;
    }

    #[test]
    fn filter_click_is_ignored_while_the_sidebar_is_not_active() {
        // The click that focuses the sidebar is not a chip selection. tmux
        // focuses the pane it delivers the click to, so the event arrives
        // anyway; acting on it changes a server-wide setting from a click
        // the user made only to reach the sidebar.
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.global.status_filter = StatusFilter::All;
        state.timers.last_filter_click =
            std::time::Instant::now() - std::time::Duration::from_millis(200);
        assert!(!state.sidebar_pane.pane_active);

        // A click on the busy chip, while the sidebar is not the active pane.
        state.handle_filter_click(5);

        assert_eq!(
            state.global.status_filter,
            StatusFilter::All,
            "a focusing click must not change the status filter"
        );
    }

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

    fn write_activity_log(pane_id: &str, contents: &str) -> String {
        let path = crate::activity::log_file_path(pane_id);
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn rebuild_row_targets_from_repo_groups() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "dotfiles".into(),
                has_focus: true,
                panes: vec![
                    (test_pane("%1"), PaneGitInfo::default()),
                    (test_pane("%2"), PaneGitInfo::default()),
                ],
            },
            RepoGroup {
                name: "app".into(),
                has_focus: false,
                panes: vec![(test_pane("%3"), PaneGitInfo::default())],
            },
        ];
        state.rebuild_row_targets();

        assert_eq!(state.layout.pane_row_targets.len(), 3);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");
        assert_eq!(state.layout.pane_row_targets[1].pane_id, "%2");
        assert_eq!(state.layout.pane_row_targets[2].pane_id, "%3");
    }

    #[test]
    fn selection_crosses_repo_groups() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "dotfiles".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())],
            },
            RepoGroup {
                name: "app".into(),
                has_focus: false,
                panes: vec![(test_pane("%5"), PaneGitInfo::default())],
            },
        ];
        state.rebuild_row_targets();

        // Start at first group
        assert_eq!(state.global.selected_pane_row, 0);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");

        // Move to second group
        assert!(state.move_pane_selection(1));
        assert_eq!(state.global.selected_pane_row, 1);
        assert_eq!(state.layout.pane_row_targets[1].pane_id, "%5");
    }

    #[test]
    fn task_progress_hides_when_all_completed() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%100".to_string();

        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane("%100"), PaneGitInfo::default())],
        }];

        let log_path = crate::activity::log_file_path(&pane_id);
        fs::write(
            &log_path,
            "10:00|TaskCreate|#1 A\n10:01|TaskCreate|#2 B\n10:02|TaskUpdate|completed #1\n10:03|TaskUpdate|completed #2\n",
        ).unwrap();

        state.refresh_task_progress();

        // All completed → hidden immediately
        assert!(state.pane_task_progress(&pane_id).is_none());
        // Dismissed count should be recorded
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(2));

        // Calling refresh again should still be hidden (no flicker)
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_none());

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn task_progress_reshows_when_new_tasks_added() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%101".to_string();

        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane("%101"), PaneGitInfo::default())],
        }];

        // First: 1 task, completed → dismissed
        let log_path = crate::activity::log_file_path(&pane_id);
        fs::write(
            &log_path,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|completed #1\n",
        )
        .unwrap();
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_none());

        // Now add a new in-progress task → should re-show
        fs::write(
            &log_path,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|completed #1\n10:02|TaskCreate|#2 B\n10:03|TaskUpdate|in_progress #2\n",
        ).unwrap();
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn classify_task_progress_empty_clears() {
        let progress = TaskProgress { tasks: vec![] };
        assert_eq!(
            classify_task_progress(&progress, None),
            TaskProgressDecision::Clear
        );
    }

    #[test]
    fn classify_task_progress_in_progress_shows() {
        let progress = TaskProgress {
            tasks: vec![
                ("A".into(), TaskStatus::Completed),
                ("B".into(), TaskStatus::InProgress),
            ],
        };
        assert_eq!(
            classify_task_progress(&progress, None),
            TaskProgressDecision::Show
        );
    }

    #[test]
    fn classify_task_progress_completed_dismisses_once() {
        let progress = TaskProgress {
            tasks: vec![
                ("A".into(), TaskStatus::Completed),
                ("B".into(), TaskStatus::Completed),
            ],
        };
        assert_eq!(
            classify_task_progress(&progress, None),
            TaskProgressDecision::Dismiss { total: 2 }
        );
        assert_eq!(
            classify_task_progress(&progress, Some(2)),
            TaskProgressDecision::Skip
        );
    }

    #[test]
    fn classify_task_progress_completed_with_different_dismissal_dismisses_again() {
        let progress = TaskProgress {
            tasks: vec![
                ("A".into(), TaskStatus::Completed),
                ("B".into(), TaskStatus::Completed),
            ],
        };
        assert_eq!(
            classify_task_progress(&progress, Some(1)),
            TaskProgressDecision::Dismiss { total: 2 }
        );
    }

    #[test]
    fn refresh_now_updates_current_time() {
        let mut state = AppState::new("%99".into());
        state.refresh_now();
        assert!(state.now > 0);
    }

    #[test]
    fn refresh_activity_log_reads_focused_pane() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%201";
        let log_path = crate::activity::log_file_path(pane_id);
        fs::write(&log_path, "10:00|Read|old\n10:01|Edit|new\n").unwrap();
        state.focus_state.focused_pane_id = Some(pane_id.into());
        state.activity.max_entries = 50;

        state.refresh_activity_log();

        assert_eq!(state.activity.entries.len(), 2);
        assert_eq!(state.activity.entries[0].tool, "Edit");
        assert_eq!(state.activity.entries[0].label, "new");
        assert_eq!(state.activity.entries[1].tool, "Read");

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_activity_log_clears_without_focus() {
        let mut state = AppState::new("%99".into());
        state.activity.entries = vec![crate::activity::ActivityEntry {
            timestamp: "10:00".into(),
            tool: "Read".into(),
            label: "keep?".into(),
        }];

        state.focus_state.focused_pane_id = None;
        state.refresh_activity_log();

        assert!(state.activity.entries.is_empty());
    }

    #[test]
    fn refresh_task_progress_clears_empty_logs_and_dismissal() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%202".to_string();
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        state.set_pane_task_progress(
            &pane_id,
            Some(TaskProgress {
                tasks: vec![("stale".into(), TaskStatus::InProgress)],
            }),
        );
        state.set_pane_task_dismissed_total(&pane_id, Some(1));

        state.refresh_task_progress();

        assert!(state.pane_task_progress(&pane_id).is_none());
        assert_eq!(state.pane_task_dismissed_total(&pane_id), None);
    }

    #[test]
    fn refresh_task_progress_shows_in_progress_and_clears_dismissal() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%203".to_string();
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        state.set_pane_task_dismissed_total(&pane_id, Some(1));
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|in_progress #1\n",
        );

        state.refresh_task_progress();

        assert_eq!(state.pane_task_dismissed_total(&pane_id), None);
        assert_eq!(
            state.pane_task_progress(&pane_id).map(|p| p.total()),
            Some(1)
        );
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_records_completed_dismissal() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%204".to_string();
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|completed #1\n",
        );

        state.refresh_task_progress();

        assert!(state.pane_task_progress(&pane_id).is_none());
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_skips_already_dismissed_completed_tasks() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%205".to_string();
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|completed #1\n",
        );

        state.refresh_task_progress();
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));
        assert!(state.pane_task_progress(&pane_id).is_none());

        state.refresh_task_progress();
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));
        assert!(state.pane_task_progress(&pane_id).is_none());
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_drops_dismissals_for_inactive_panes() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%206".to_string();
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|completed #1\n",
        );
        state.refresh_task_progress();
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));

        // Pane removed — both dismissed and inactive_since should be cleaned up
        // by `prune_pane_states_to_current_panes`, which `refresh()` runs via
        // `apply_session_snapshot` immediately before `refresh_task_progress`.
        state.repo_groups.clear();
        state.set_pane_inactive_since(&pane_id, Some(100));
        state.prune_pane_states_to_current_panes();
        state.refresh_task_progress();

        assert!(state.pane_state(&pane_id).is_none());
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn pane_runtime_state_accessors_round_trip() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%213";

        state.set_pane_ports(pane_id, vec![3000, 5173]);
        state.set_pane_command(pane_id, Some("npm run dev".into()));
        state.set_pane_task_progress(
            pane_id,
            Some(TaskProgress {
                tasks: vec![("A".into(), TaskStatus::InProgress)],
            }),
        );
        state.set_pane_task_dismissed_total(pane_id, Some(4));
        state.set_pane_inactive_since(pane_id, Some(123));

        assert_eq!(state.pane_ports(pane_id), Some(&[3000, 5173][..]));
        assert_eq!(state.pane_command(pane_id), Some("npm run dev"));
        assert_eq!(
            state.pane_task_progress(pane_id).map(|p| p.total()),
            Some(1)
        );
        assert_eq!(state.pane_task_dismissed_total(pane_id), Some(4));
        assert_eq!(state.pane_inactive_since(pane_id), Some(123));

        state.clear_pane_state(pane_id);
        assert!(state.pane_state(pane_id).is_none());
    }

    #[test]
    fn prune_pane_states_to_current_panes_drops_stale_entries() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane("%1"), PaneGitInfo::default())],
        }];
        state.set_pane_ports("%1", vec![3000]);
        state.set_pane_command("%1", Some("npm run dev".into()));
        state.set_pane_ports("%2", vec![5173]);
        state.set_pane_task_dismissed_total("%2", Some(2));

        state.prune_pane_states_to_current_panes();

        assert_eq!(state.pane_ports("%1"), Some(&[3000][..]));
        assert_eq!(state.pane_command("%1"), Some("npm run dev"));
        assert!(state.pane_state("%2").is_none());
    }

    #[test]
    fn refresh_task_progress_dismisses_incomplete_tasks_when_agent_idle() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%207".to_string();
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Idle;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        // 5 out of 6 tasks completed — agent is idle so it won't update further
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskCreate|#2 B\n10:02|TaskCreate|#3 C\n10:03|TaskCreate|#4 D\n10:04|TaskCreate|#5 E\n10:05|TaskCreate|#6 F\n10:06|TaskUpdate|completed #1\n10:07|TaskUpdate|completed #2\n10:08|TaskUpdate|completed #3\n10:09|TaskUpdate|completed #4\n10:10|TaskUpdate|completed #5\n",
        );

        // First refresh: grace period starts, tasks still shown (not dismissed yet)
        state.now = 100;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());
        assert!(state.pane_inactive_since(&pane_id).is_some());

        // After grace period (3s): should be dismissed
        state.now = 104;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_none());
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(6));
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_shows_incomplete_tasks_when_agent_running() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%208".to_string();
        // test_pane defaults to PaneStatus::Running
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(test_pane(&pane_id), PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskCreate|#2 B\n10:02|TaskUpdate|completed #1\n10:03|TaskUpdate|in_progress #2\n",
        );

        state.refresh_task_progress();

        // Agent is running, so incomplete tasks should still be shown
        assert!(state.pane_task_progress(&pane_id).is_some());
        assert_eq!(
            state.pane_task_progress(&pane_id).map(|p| p.total()),
            Some(2)
        );
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_dismisses_incomplete_tasks_when_agent_error() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%209".to_string();
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Error;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|in_progress #1\n",
        );

        // First refresh: grace period starts, tasks still shown
        state.now = 100;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());

        // After grace period: agent errored out — dismiss incomplete tasks
        state.now = 104;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_none());
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));
        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_debounce_resets_when_agent_resumes() {
        // Simulates brief idle flicker: agent goes idle then returns to running
        // before the grace period expires — tasks should remain visible.
        let mut state = AppState::new("%99".into());
        let pane_id = "%210".to_string();
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Idle;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskCreate|#2 B\n10:02|TaskUpdate|completed #1\n",
        );

        // Agent is idle — grace timer starts, tasks still shown
        state.now = 100;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());
        assert!(state.pane_inactive_since(&pane_id).is_some());

        // Agent returns to running before grace expires — timer resets
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Running;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        state.now = 102;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());
        assert!(state.pane_inactive_since(&pane_id).is_none());

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_debounce_exact_boundary() {
        // Grace period is 3 seconds. At exactly 3s the condition is >=,
        // so it should dismiss.
        let mut state = AppState::new("%99".into());
        let pane_id = "%211".to_string();
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Idle;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|in_progress #1\n",
        );

        // t=100: grace timer starts
        state.now = 100;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());

        // t=102 (2s elapsed): still within grace period — tasks shown
        state.now = 102;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_some());

        // t=103 (exactly 3s): grace expired (>= 3) — dismissed
        state.now = 103;
        state.refresh_task_progress();
        assert!(state.pane_task_progress(&pane_id).is_none());
        assert_eq!(state.pane_task_dismissed_total(&pane_id), Some(1));

        fs::remove_file(&log_path).ok();
    }

    #[test]
    fn refresh_task_progress_waiting_does_not_start_debounce() {
        // Waiting is an active state — inactive timer should not be set.
        let mut state = AppState::new("%99".into());
        let pane_id = "%212".to_string();
        let mut pane = test_pane(&pane_id);
        pane.status = PaneStatus::Waiting;
        state.repo_groups = vec![RepoGroup {
            name: "test".into(),
            has_focus: true,
            panes: vec![(pane, PaneGitInfo::default())],
        }];
        let log_path = write_activity_log(
            &pane_id,
            "10:00|TaskCreate|#1 A\n10:01|TaskUpdate|in_progress #1\n",
        );

        state.now = 100;
        state.refresh_task_progress();

        // Tasks shown and no inactive timer started
        assert!(state.pane_task_progress(&pane_id).is_some());
        assert!(state.pane_inactive_since(&pane_id).is_none());

        fs::remove_file(&log_path).ok();
    }

    // ─── ScrollState unit tests ─────────────────────────────────────

    #[test]
    fn scroll_state_clamps_to_max() {
        let mut s = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 4,
        };
        s.scroll(100);
        assert_eq!(s.offset, 6); // max = 10 - 4
    }

    #[test]
    fn scroll_state_clamps_to_zero() {
        let mut s = ScrollState {
            offset: 3,
            total_lines: 10,
            visible_height: 4,
        };
        s.scroll(-100);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn scroll_state_noop_when_content_fits() {
        let mut s = ScrollState {
            offset: 0,
            total_lines: 3,
            visible_height: 5,
        };
        s.scroll(1);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn scroll_state_exact_fit_no_scroll() {
        let mut s = ScrollState {
            offset: 0,
            total_lines: 5,
            visible_height: 5,
        };
        s.scroll(1);
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn scroll_state_incremental() {
        let mut s = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 4,
        };
        s.scroll(1);
        assert_eq!(s.offset, 1);
        s.scroll(2);
        assert_eq!(s.offset, 3);
        s.scroll(-1);
        assert_eq!(s.offset, 2);
    }

    // ─── apply_git_data tests ───────────────────────────────────────

    #[test]
    fn apply_git_data_copies_all_fields() {
        let mut state = AppState::new("%99".into());
        let data = crate::git::GitData {
            diff_stat: Some((10, 5)),
            branch: "feature/test".into(),
            ahead_behind: Some((2, 1)),
            staged_files: vec![crate::git::GitFileEntry {
                status: 'M',
                name: "lib.rs".into(),
                additions: 10,
                deletions: 5,
                path: String::new(),
            }],
            unstaged_files: vec![],
            untracked_files: vec!["new.rs".into()],
            remote_url: "https://github.com/user/repo".into(),
            pr_number: Some("42".into()),
        };

        state.apply_git_data(data);

        assert_eq!(state.git.diff_stat, Some((10, 5)));
        assert_eq!(state.git.branch, "feature/test");
        assert_eq!(state.git.ahead_behind, Some((2, 1)));
        assert_eq!(state.git.staged_files.len(), 1);
        assert_eq!(state.git.staged_files[0].status, 'M');
        assert!(state.git.unstaged_files.is_empty());
        assert_eq!(state.git.untracked_files, vec!["new.rs"]);
        assert_eq!(state.git.changed_file_count(), 2);
        assert_eq!(state.git.remote_url, "https://github.com/user/repo");
        assert_eq!(state.git.pr_number, Some("42".into()));
    }

    #[test]
    fn apply_git_data_with_defaults() {
        let mut state = AppState::new("%99".into());
        // Pre-fill some state
        state.git.branch = "old-branch".into();
        state.git.pr_number = Some("99".into());

        // Apply empty git data
        state.apply_git_data(crate::git::GitData::default());

        assert_eq!(state.git.diff_stat, None);
        assert!(state.git.branch.is_empty());
        assert_eq!(state.git.ahead_behind, None);
        assert!(state.git.staged_files.is_empty());
        assert!(state.git.unstaged_files.is_empty());
        assert!(state.git.untracked_files.is_empty());
        assert_eq!(state.git.changed_file_count(), 0);
        assert!(state.git.remote_url.is_empty());
        assert_eq!(state.git.pr_number, None);
    }

    #[test]
    fn apply_session_snapshot_rebuilds_derived_state() {
        let mut state = AppState::new("%99".into());
        state.global.selected_pane_row = 3;

        let pane = test_pane("%1");
        let sessions = vec![crate::tmux::SessionInfo {
            session_name: "main".into(),
            windows: vec![crate::tmux::WindowInfo {
                window_id: "@0".into(),
                window_name: "project".into(),
                window_active: true,
                auto_rename: false,
                panes: vec![pane],
            }],
        }];

        state.apply_session_snapshot(true, sessions);

        assert!(state.focus_state.sidebar_focused);
        assert_eq!(state.repo_groups.len(), 1);
        assert_eq!(state.layout.pane_row_targets.len(), 1);
        assert_eq!(state.global.selected_pane_row, 0);
        // focused_pane_id is set by find_focused_pane() which queries tmux
        // directly, so we don't assert it here (tmux not available in tests).
    }

    // ─── BottomTab label parse ──────────────────────────────────────

    #[test]
    fn thought_parses_to_thought_and_unknown_still_falls_back_to_activity() {
        // The shared option may name the Thought tab a newer sidebar
        // wrote; an old binary reading it must not invent a tab, and
        // any unknown value stays on the Activity default.
        assert_eq!(BottomTab::from_label("thought"), BottomTab::Thought);
        assert_eq!(BottomTab::from_label("activity"), BottomTab::Activity);
        assert_eq!(BottomTab::from_label("nonsense"), BottomTab::Activity);
        assert_eq!(BottomTab::Thought.as_str(), "thought");
        assert_eq!(BottomTab::Thought.label(), "\u{f07f6}");
    }

    // ─── next_bottom_tab / scroll_bottom tests ──────────────────────

    #[test]
    fn next_bottom_tab_visits_all_three_tabs() {
        let mut state = AppState::new("%99".into());
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
        state.next_bottom_tab();
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
    }

    #[test]
    fn scroll_bottom_dispatches_to_activity() {
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Activity;
        state.activity.scroll = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 3,
        };

        state.scroll_bottom(2);
        assert_eq!(state.activity.scroll.offset, 2);
        assert_eq!(state.scrolls.git.offset, 0);
    }

    #[test]
    fn scroll_bottom_dispatches_to_git() {
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::GitStatus;
        state.scrolls.git = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 3,
        };

        state.scroll_bottom(2);
        assert_eq!(state.scrolls.git.offset, 2);
        assert_eq!(state.activity.scroll.offset, 0);
    }

    #[test]
    fn scroll_bottom_dispatches_to_tasks() {
        // The Tasks tab owns a third scroll offset: scrolling there moves
        // nothing in the other two tabs.
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Tasks;
        state.tasks.scroll = ScrollState {
            offset: 0,
            total_lines: 10,
            visible_height: 3,
        };

        state.scroll_bottom(2);
        assert_eq!(state.tasks.scroll.offset, 2);
        assert_eq!(state.activity.scroll.offset, 0);
        assert_eq!(state.scrolls.git.offset, 0);
    }

    // ─── handle_mouse_scroll tests ────────────────────────────────────

    #[test]
    fn mouse_scroll_in_bottom_panel_scrolls_activity() {
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Activity;
        state.activity.scroll = ScrollState {
            offset: 0,
            total_lines: 30,
            visible_height: 10,
        };
        // term_height=50, bottom_panel=20 → bottom starts at row 30
        // mouse at row 35 → in bottom panel
        state.handle_mouse_scroll(35, 5, 50, 20, 3);
        assert_eq!(state.activity.scroll.offset, 3);
        assert_eq!(state.scrolls.panes.offset, 0);
    }

    #[test]
    fn mouse_scroll_in_agents_panel_scrolls_agents() {
        let mut state = AppState::new("%99".into());
        state.scrolls.panes = ScrollState {
            offset: 0,
            total_lines: 40,
            visible_height: 20,
        };
        // term_height=50, bottom_panel=20 → bottom starts at row 30
        // mouse at row 10 → in agents panel
        state.handle_mouse_scroll(10, 5, 50, 20, 3);
        assert_eq!(state.scrolls.panes.offset, 3);
        assert_eq!(state.activity.scroll.offset, 0);
    }

    #[test]
    fn mouse_scroll_up_in_agents_panel() {
        let mut state = AppState::new("%99".into());
        state.scrolls.panes = ScrollState {
            offset: 5,
            total_lines: 40,
            visible_height: 20,
        };
        state.handle_mouse_scroll(10, 5, 50, 20, -3);
        assert_eq!(state.scrolls.panes.offset, 2);
    }

    #[test]
    fn mouse_scroll_at_boundary_row_goes_to_bottom() {
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::GitStatus;
        state.scrolls.git = ScrollState {
            offset: 0,
            total_lines: 20,
            visible_height: 10,
        };
        // term_height=50, bottom_panel=20 → bottom starts at row 30
        // mouse at exactly row 30 → in bottom panel
        state.handle_mouse_scroll(30, 5, 50, 20, 3);
        assert_eq!(state.scrolls.git.offset, 3);
        assert_eq!(state.scrolls.panes.offset, 0);
    }

    #[test]
    fn mouse_scroll_just_above_boundary_goes_to_agents() {
        let mut state = AppState::new("%99".into());
        state.scrolls.panes = ScrollState {
            offset: 0,
            total_lines: 40,
            visible_height: 20,
        };
        // row 29, just above bottom_start=30
        state.handle_mouse_scroll(29, 5, 50, 20, 3);
        assert_eq!(state.scrolls.panes.offset, 3);
        assert_eq!(state.activity.scroll.offset, 0);
    }

    #[test]
    fn mouse_scroll_in_bottom_panel_scrolls_tasks() {
        // Same band routing as the other two tabs — the wheel over the
        // panel moves the visible tab's own offset.
        let mut state = AppState::new("%99".into());
        state.global.bottom_tab = BottomTab::Tasks;
        state.tasks.scroll = ScrollState {
            offset: 0,
            total_lines: 30,
            visible_height: 10,
        };

        state.handle_mouse_scroll(35, 5, 50, 20, 3);
        assert_eq!(state.tasks.scroll.offset, 3);
        assert_eq!(state.scrolls.panes.offset, 0);
    }

    // ─── handle_terminal_click tests ──────────────────────────────────

    #[test]
    fn terminal_click_on_the_tab_row_switches_the_tab() {
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 20;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);
        state.global.bottom_tab = BottomTab::GitStatus;
        // term_height=50, bottom panel 20 → rule at row 30. The press
        // starts a split drag (any press column resizes — decision 4);
        // only the release, if the mouse never left the rule, selects.
        assert!(state.handle_terminal_click(30, 15, 50));
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "the press alone must not select a tab"
        );

        state.end_split_drag(50);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Activity,
            "the unmoved release is the click that selects"
        );
        assert!(state.split_drag.is_none(), "the release ends the drag");
    }

    #[test]
    fn terminal_click_above_the_tab_row_goes_to_the_agents_panel() {
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 20;
        state.layout.pane_row_targets = vec![RowTarget {
            pane_id: "%1".into(),
        }];
        state.layout.line_to_row = vec![Some(0), Some(0), Some(0)];
        // row 3 → line_index 1 → agent row 0
        assert!(!state.handle_terminal_click(3, 5, 50));
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn terminal_click_inside_the_panel_body_does_nothing() {
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 20;
        // Row 35 is inside the panel, below its tab row at 30.
        assert!(!state.handle_terminal_click(35, 5, 50));
        assert!(state.flash.is_none());
    }

    #[test]
    fn terminal_click_uses_the_clamped_height_while_floating() {
        // The regression the frame-scoped height fixes: while floating
        // the panel is drawn clamped (12 of a 38-row panel), so the hit
        // test must agree with the drawn geometry — a click on the drawn
        // tab row (26) hits the tab, and a click on a drawn agent row in
        // the band the unclamped height wrongly claimed (18..26) selects
        // the agent instead of being swallowed.
        let mut state = AppState::new("%99".into());
        state.sidebar_pane.floating = true;
        // 53% of a 38-row sidebar is 20 rows — the unclamped height the
        // frame's drawn 12 rows replaced.
        state.global.bottom_percent = 53;
        state.layout.bottom_panel_height = 12;
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
        ];
        // Rows 2..26 all map to agent rows, so the band right above the
        // clamped panel (rows 18..25) is clickable agent list.
        state.layout.line_to_row = vec![Some(1); 24];
        state.global.selected_pane_row = 0;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);
        state.global.bottom_tab = BottomTab::GitStatus;

        // Row 26 is the drawn tab row (38 - 12); the unclamped height
        // would have put the boundary at row 18 and swallowed this click.
        // The press starts a drag on the drawn geometry; the unmoved
        // release is the click that selects.
        assert!(state.handle_terminal_click(26, 15, 38));
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
        state.end_split_drag(38);
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);

        // Row 25 is a drawn agent-list row above the clamped panel.
        assert!(!state.handle_terminal_click(25, 5, 38));
        assert_eq!(state.global.selected_pane_row, 1);

        // The wheel over the same band scrolls the agents list, not the
        // bottom panel, and the wheel over the panel scrolls the panel.
        state.scrolls.panes = ScrollState {
            offset: 0,
            total_lines: 40,
            visible_height: 20,
        };
        state.activity.scroll = ScrollState {
            offset: 0,
            total_lines: 30,
            visible_height: 10,
        };
        state.handle_mouse_scroll(24, 5, 38, state.layout.bottom_panel_height, 3);
        assert_eq!(state.scrolls.panes.offset, 3);
        state.handle_mouse_scroll(28, 5, 38, state.layout.bottom_panel_height, 3);
        assert_eq!(state.activity.scroll.offset, 3);
        assert_eq!(state.scrolls.panes.offset, 3);
    }

    // ─── split drag: press, move, release ─────────────────────────────

    #[test]
    fn a_split_drag_resizes_and_the_release_saves_the_percent() {
        // Spec "a drag up": 30% of a 40-row sidebar draws the rule on
        // row 28; dragging to row 20 keeps 20 bottom rows during the
        // drag and the release saves the percent. In tests every tmux
        // write hits the isolated socket and is refused, so the save
        // shows as the pending latch.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        state.global.bottom_tab = BottomTab::GitStatus;

        // Press on the dash fill of the rule.
        assert!(state.handle_terminal_click(28, 30, 40));
        assert_eq!(
            state.global.bottom_percent, 30,
            "the press alone must not resize"
        );
        assert!(state.split_drag.is_some(), "the press starts the drag");

        state.handle_split_drag_move(20, 30, 40);
        let drag = state.split_drag.expect("the drag must still be live");
        assert!(drag.moved, "moving to row 20 is a drag, not a held press");
        assert_eq!(drag.size, 20, "the rule follows the mouse row");

        state.end_split_drag(40);
        assert!(state.split_drag.is_none());
        assert_eq!(
            state.global.bottom_percent, 50,
            "20 bottom rows of a 40-row sidebar are 50 percent"
        );
        assert!(
            state.global.percent_save_pending(),
            "the release must hand the percent to the tmux write"
        );
    }

    #[test]
    fn a_split_drag_past_the_agent_minimum_stops_at_the_limit() {
        // Spec "a drag past the limit": the rule dragged to row 1 of a
        // 40-row sidebar stops at row 3 — 37 bottom rows — so the agent
        // list keeps its 3 minimum rows.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;

        assert!(state.handle_terminal_click(28, 30, 40));
        state.handle_split_drag_move(1, 30, 40);

        let drag = state.split_drag.expect("the drag must still be live");
        assert_eq!(drag.size, 37, "37 bottom rows put the drawn rule on row 3");
    }

    #[test]
    fn an_unmoved_release_on_git_selects_the_git_tab() {
        // Spec "a click on a tab label": press and release on the Git
        // label with no move — the release selects the tab and the
        // split height does not change.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);

        // Press on col 23, the centered Git glyph.
        assert!(state.handle_terminal_click(28, 23, 40));
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Activity,
            "a press on a label must not select it yet"
        );

        state.end_split_drag(40);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "col 23 is the Git glyph"
        );
        assert_eq!(
            state.global.bottom_percent, 30,
            "a click must not resize the split"
        );
    }

    #[test]
    fn an_unmoved_release_on_the_dash_fill_changes_nothing() {
        // Spec "a click on the dashes": the fill right of the labels is
        // outside every label's hitbox, so neither the tab nor the
        // split may change.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        state.global.bottom_tab = BottomTab::Tasks;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);

        state.handle_terminal_click(28, 30, 40);
        state.end_split_drag(40);

        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        assert_eq!(
            state.global.bottom_percent, 30,
            "a click must not resize the split"
        );
    }

    #[test]
    fn a_drag_that_starts_on_a_label_resizes_without_selecting_the_tab() {
        // Spec "a drag that starts on a tab label": pressing the Tasks
        // label and dragging to another row resizes the split; the tab
        // selection must not follow the press.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        state.global.bottom_tab = BottomTab::Tasks;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);

        // Press on col 19, the Tasks glyph, then drag.
        state.handle_terminal_click(28, 19, 40);
        state.handle_split_drag_move(20, 30, 40);
        state.end_split_drag(40);

        assert_eq!(
            state.global.bottom_percent, 50,
            "the drag resized the split"
        );
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Tasks,
            "the drag selected no tab"
        );
    }

    #[test]
    fn drag_and_release_without_a_press_change_nothing() {
        // crossterm can deliver a Drag or an Up whose Down never reached
        // this pane (pane focus mid-press). The handlers must no-op on
        // them instead of inventing a split resize or a tab click.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;

        state.handle_split_drag_move(20, 30, 40);
        state.end_split_drag(40);

        assert!(state.split_drag.is_none());
        assert_eq!(state.global.bottom_percent, 30);
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
    }

    // ─── a live drag ends when a new press arrives ───────────────────────

    #[test]
    fn a_new_press_ends_a_moved_drag_at_its_saved_rows() {
        // A release consumed elsewhere (tmux routed it to another pane)
        // must not leave the drag live blocking percent adoption: the
        // next press ends the stale drag as if released at its last
        // rows — the moved branch runs, so the percent save happens —
        // and the new press is then handled, starting a fresh drag of
        // its own here.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        state.global.bottom_tab = BottomTab::GitStatus;

        // Drag: press on the rule at row 28, move to row 35 (5 rows),
        // no release reaches the pane.
        state.handle_terminal_click(28, 30, 40);
        state.handle_split_drag_move(35, 30, 40);

        // A new press on the rule (row 28 — the frame never redrew, so
        // the drawn geometry is unchanged) ends the stale drag and
        // starts a fresh drag at col 3.
        state.handle_terminal_click(28, 3, 40);

        assert_eq!(
            state.global.bottom_percent, 13,
            "the stale drag saved its last rows (5 of 40 ≈ 13%)"
        );
        assert!(
            state.global.percent_save_pending(),
            "the stale release must hand the percent to the tmux write"
        );
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "the stale end's moved branch selects no tab"
        );
        let drag = state
            .split_drag
            .expect("the new press must start a fresh drag");
        assert_eq!(drag.press_col, 3);
        assert!(!drag.moved, "the fresh press is not yet a drag");
        assert_eq!(drag.size, state.layout.bottom_panel_height);
    }

    #[test]
    fn a_new_press_ends_an_unmoved_drag_without_a_tab_click() {
        // An unmoved drag is only a held press: its stale end runs no
        // save and no tab click, because the user never let the button
        // go on that label and nothing was resized. The fixture makes a
        // save observable: 1% of a 40-row sidebar draws 3 clipped rows
        // (the 3-row minimum) and a save would round-trip them to 8% —
        // the drift the moved branch must not run on this end. The new
        // press — here on Git while the stale one sat on Tasks — is the
        // only thing that resolves into a click.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 3;
        state.global.bottom_percent = 1;
        crate::ui::bottom::draw_tab_targets(&mut state, 40);
        state.global.bottom_tab = BottomTab::Activity;

        // An unmoved press on the Tasks glyph, then a second press on
        // the Git glyph (both on the drawn rule row 37 = 40 - 3).
        state.handle_terminal_click(37, 19, 40);
        state.handle_terminal_click(37, 23, 40);

        assert_eq!(
            state.global.bottom_percent, 1,
            "the stale unmoved drag must end without saving its rows"
        );
        assert!(
            !state.global.percent_save_pending(),
            "the unmoved stale end must leave no tmux write pending"
        );
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::Activity,
            "the stale unmoved press must not release on its label"
        );
        let drag = state
            .split_drag
            .expect("the new press started a fresh drag");
        assert_eq!(drag.press_col, 23);

        state.end_split_drag(40);
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "the new press's unmoved release is the click that selects"
        );
    }

    // ─── strip press routing ──────────────────────────────────────

    /// A strip state whose frame drew the 120×12, 40-percent layout:
    /// filter 0..4, agents 4..72, the divider line on column 72, tabs
    /// 73..120, and the tab bar on row 0 spanning 72..120. The
    /// press routing tests read only the drawn `StripLayout`, the way
    /// the handlers do.
    fn strip_state() -> AppState {
        let mut state = AppState::new("%99".into());
        state.layout_kind = LayoutKind::Strip;
        state.layout.strip = Some(strip_layout(120, 12, 40));
        state
    }

    #[test]
    fn a_strip_chip_click_selects_the_category() {
        // The strip filter bar draws one chip per row — row 2 is the
        // waiting chip on a 12-row bar — and a click on it selects the
        // category the column's filter row click would.
        let mut state = strip_state();
        state.global.status_filter = StatusFilter::All;
        reset_filter_debounce(&mut state);

        state.handle_terminal_click(2, 1, 12);

        assert_eq!(
            state.global.status_filter,
            StatusFilter::Waiting,
            "the click on row 2 selects the waiting chip"
        );
        assert!(state.split_drag.is_none(), "a chip click starts no drag");
    }

    #[test]
    fn a_strip_click_on_the_repo_header_row_changes_no_filter() {
        // The agent panel's first row is the repo header, not a filter
        // row: a click on it must not touch the status filter — it
        // routes to the secondary header, where the repo button opens
        // the repo popup.
        let mut state = strip_state();
        state.global.status_filter = StatusFilter::All;
        state.repo_groups = vec![
            RepoGroup {
                name: "alpha".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())],
            },
            RepoGroup {
                name: "beta".into(),
                has_focus: false,
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.layout.repo_button_col = Some(20);
        reset_filter_debounce(&mut state);

        // agents.y is 0: row 0 is the header. Left of the repo button.
        state.handle_terminal_click(0, 10, 12);
        assert_eq!(
            state.global.status_filter,
            StatusFilter::All,
            "the header row must not change the status filter"
        );
        assert!(!state.is_repo_popup_open());

        // On the repo button: the header's button column is relative
        // to the header area (absolute col 24 = agents.x 4 + 20), and
        // the header click toggles the popup, exactly as row 1 of a
        // column does.
        state.handle_terminal_click(0, 24, 12);
        assert!(state.is_repo_popup_open());
        assert_eq!(state.global.status_filter, StatusFilter::All);
    }

    #[test]
    fn a_strip_click_on_an_agent_row_selects_through_the_scroll_offset() {
        // List rows start one row under the header (no filter row in a
        // strip) and are offset by the scroll the frame drew with: row
        // 2 with offset 2 is line (2 - 0 - 1) + 2 = 3, the third
        // rendered agent row.
        let mut state = strip_state();
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
            RowTarget {
                pane_id: "%3".into(),
            },
        ];
        state.layout.line_to_row = vec![None, Some(0), Some(1), Some(2)];
        state.scrolls.panes.offset = 2;
        state.global.selected_pane_row = 0;

        state.handle_terminal_click(2, 10, 12);

        assert_eq!(
            state.global.selected_pane_row, 2,
            "the clicked list row is the third rendered agent row"
        );
        assert!(state.split_drag.is_none());
    }

    #[test]
    fn a_strip_click_on_a_spawn_button_opens_the_spawn_input() {
        // The + spawn buttons are hit-tested in absolute frame
        // coordinates, before the list-row fallback, so a click on one
        // opens the spawn modal and never shifts the selection.
        let mut state = strip_state();
        // Inside the drawn agent panel (columns 4..72): the + button
        // on list row 3, hit at its middle column.
        state.layout.repo_spawn_targets = vec![RepoSpawnTarget {
            rect: ratatui::layout::Rect::new(50, 3, 3, 1),
            repo_name: "alpha".into(),
            repo_root: "/tmp/alpha".into(),
        }];
        state.global.selected_pane_row = 0;

        state.handle_terminal_click(3, 51, 12);

        assert!(
            matches!(state.popup, PopupState::SpawnInput { .. }),
            "the click on the + button opens the spawn modal"
        );
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn a_strip_press_on_the_divider_starts_a_strip_drag() {
        // The spec's geometry: at 40 percent of 120 columns the
        // divider line sits on column 72 and holds 48 tab columns. A
        // press anywhere on the drawn divider starts a strip drag with
        // that size — and touches no column-side value.
        let mut state = strip_state();
        state.global.bottom_percent = 30;

        assert!(
            state.handle_terminal_click(5, 72, 12),
            "the press on the divider is the press that starts the drag"
        );
        let drag = state.split_drag.expect("the press starts the drag");
        assert_eq!(drag.axis, LayoutKind::Strip);
        assert!(!drag.moved, "the press is not yet a drag");
        assert_eq!(drag.size, 48, "the drawn tab columns at the press");
        assert_eq!(
            state.global.bottom_percent, 30,
            "a strip press must not touch the column's percent"
        );
        assert!(!state.global.percent_save_pending());
    }

    #[test]
    fn a_strip_press_outside_the_divider_starts_no_drag() {
        // The divider line and the tab-bar row are the regions that
        // start a drag — a press on a mid-strip row of the agent
        // panel, the tab panel or the filter column starts none.
        let mut state = strip_state();
        for (row, col) in [(5, 10), (5, 80), (5, 0)] {
            state.handle_terminal_click(row, col, 12);
            assert!(
                state.split_drag.is_none(),
                "a press at ({row}, {col}) must not start a drag"
            );
        }
    }

    #[test]
    fn a_strip_drag_stale_end_saves_the_tab_percent() {
        // The stale-drag end on a new press saves the axis's own value:
        // a strip drag's last tab columns become @sidebar_tab_width,
        // and the column's bottom percent is never touched. The drag
        // here is forced moved (its release was lost elsewhere), with
        // 60 tab columns of the 120-column strip — 50 percent.
        let mut state = strip_state();
        state.global.bottom_percent = 30;
        state.handle_terminal_click(5, 72, 12);
        let drag = state.split_drag.as_mut().expect("the drag is live");
        drag.size = 60;
        drag.moved = true;

        // A new press anywhere ends the stale drag with its save.
        state.handle_terminal_click(5, 10, 12);

        assert_eq!(
            state.global.tab_percent, 50,
            "the stale strip drag saved 60 tab columns of 120 as 50 percent"
        );
        assert!(
            state.global.tab_percent_save_pending(),
            "the stale release must hand the tab width to the tmux write"
        );
        assert_eq!(
            state.global.bottom_percent, 30,
            "a strip drag's save must not touch the bottom percent"
        );
        assert!(!state.global.percent_save_pending());
    }

    #[test]
    fn a_strip_drag_to_column_60_saves_50_percent() {
        // Spec "a drag to the left": a 120-column strip with the
        // divider on column 72 dragged to column 60 relays the
        // divider there and saves the 60 tab columns as 50 percent on
        // release — the column's bottom percent is never touched.
        let mut state = strip_state();
        state.global.bottom_percent = 30;
        state.global.tab_percent = 40;

        assert!(state.handle_terminal_click(5, 72, 12));
        state.handle_split_drag_move(5, 60, 12);

        let drag = state.split_drag.expect("the drag must still be live");
        assert!(drag.moved, "moving to column 60 is a drag, not a hold");
        assert_eq!(drag.size, 60, "the divider follows the mouse column");
        assert_eq!(
            state.global.tab_percent, 40,
            "the drag itself must not write the tab width"
        );

        state.end_split_drag(12);
        assert_eq!(
            state.global.tab_percent, 50,
            "60 tab columns of a 120-column strip are 50 percent"
        );
        assert!(
            state.global.tab_percent_save_pending(),
            "the release must hand the tab width to the tmux write"
        );
        assert_eq!(
            state.global.bottom_percent, 30,
            "a strip drag's save must not touch the bottom percent"
        );
        assert!(!state.global.percent_save_pending());
    }

    #[test]
    fn a_strip_drag_writes_the_tab_width_only_on_release() {
        // Spec "no tmux write during the drag": moves across several
        // columns leave the tab width and its save latch alone; the
        // single write happens on release.
        let mut state = strip_state();
        state.global.tab_percent = 40;

        state.handle_terminal_click(5, 72, 12);
        state.handle_split_drag_move(5, 60, 12);
        assert!(!state.global.tab_percent_save_pending());
        state.handle_split_drag_move(5, 65, 12);
        assert!(!state.global.tab_percent_save_pending());
        assert_eq!(
            state.global.tab_percent, 40,
            "no move may write the tab width"
        );

        state.end_split_drag(12);
        assert!(
            state.global.tab_percent_save_pending(),
            "the release is the one write"
        );
    }

    /// The tab-bar column the Git glyph's drawn hitbox starts on:
    /// read back off the published targets, the same values the
    /// frame's own right-aligning produced.
    fn drawn_glyph_col(state: &AppState, tab: BottomTab) -> u16 {
        state
            .layout
            .bottom_tab_targets
            .iter()
            .find(|target| target.tab == tab)
            .map(|target| target.start)
            .expect("the tab glyph is drawn on the tab bar")
    }

    #[test]
    fn a_strip_drag_from_a_glyph_resizes_without_selecting_the_tab() {
        // Spec "a drag that starts on a glyph": pressing the tab bar
        // on the Git glyph's column and dragging to another column
        // resizes the two panels; the tab selection must not follow
        // the press.
        let mut state = strip_state();
        state.global.bottom_tab = BottomTab::GitStatus;
        crate::ui::bottom::draw_strip_tab_targets(&mut state, 72, 48);
        let git_col = drawn_glyph_col(&state, BottomTab::GitStatus);

        // Row 0 is the tab-bar row of the drawn 12-row strip.
        state.handle_terminal_click(0, git_col, 12);
        state.handle_split_drag_move(0, 60, 12);
        state.end_split_drag(12);

        assert_eq!(
            state.global.tab_percent, 50,
            "the drag resized the two panels"
        );
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "the drag selected no tab"
        );
    }

    #[test]
    fn an_unmoved_release_on_a_tab_bar_glyph_selects_its_tab() {
        // Spec "a click on a glyph": press and release on the Git
        // glyph's column of the tab bar with no move — the release
        // selects the tab and the panel widths do not change.
        let mut state = strip_state();
        state.global.bottom_tab = BottomTab::Activity;
        crate::ui::bottom::draw_strip_tab_targets(&mut state, 72, 48);
        let git_col = drawn_glyph_col(&state, BottomTab::GitStatus);

        state.handle_terminal_click(0, git_col, 12);
        state.end_split_drag(12);

        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "the press column is the Git glyph's"
        );
        assert_eq!(
            state.global.tab_percent, 40,
            "a click must not resize the panels"
        );
        assert!(!state.global.tab_percent_save_pending());
    }

    #[test]
    fn a_click_on_the_tab_bar_band_changes_nothing() {
        // Spec "a click on the band": a tab-bar column with no glyph —
        // the bar's left edge, before the right-aligned run — is
        // outside every hitbox, so neither the tab nor the panel
        // widths may change. The thin divider line keeps the same
        // rule: a press released on the line below the bar selects
        // nothing.
        let mut state = strip_state();
        state.global.bottom_tab = BottomTab::Tasks;
        crate::ui::bottom::draw_strip_tab_targets(&mut state, 72, 48);

        state.handle_terminal_click(0, 72, 12);
        state.end_split_drag(12);
        state.handle_terminal_click(5, 72, 12);
        state.end_split_drag(12);

        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        assert_eq!(
            state.global.tab_percent, 40,
            "a click on the band must not resize the panels"
        );
        assert!(!state.global.tab_percent_save_pending());
    }

    #[test]
    fn a_column_drag_saves_only_the_bottom_height() {
        // The two axes keep their own saves: a column drag's release
        // writes @sidebar_bottom_height and never @sidebar_tab_width.
        let mut state = AppState::new("%99".into());
        state.layout.bottom_panel_height = 12;
        state.global.bottom_percent = 30;
        state.global.tab_percent = 40;

        assert!(state.handle_terminal_click(28, 30, 40));
        state.handle_split_drag_move(20, 30, 40);
        state.end_split_drag(40);

        assert_eq!(state.global.bottom_percent, 50);
        assert!(state.global.percent_save_pending());
        assert_eq!(
            state.global.tab_percent, 40,
            "a column drag must not touch the tab width"
        );
        assert!(!state.global.tab_percent_save_pending());
    }

    #[test]
    fn a_strip_move_within_the_divider_columns_stays_unmoved() {
        // The divider is one column wide: a move that stays inside
        // it (press on column 72, drag event still on column 72,
        // another row) is still the held press a release treats as a
        // click — and the line holds no glyphs, so that click selects
        // nothing — the same rule the column keeps for moves along
        // its one-row rule.
        let mut state = strip_state();
        state.global.bottom_tab = BottomTab::GitStatus;
        crate::ui::bottom::draw_strip_tab_targets(&mut state, 72, 48);

        state.handle_terminal_click(2, 72, 12);
        state.handle_split_drag_move(7, 72, 12);
        state.end_split_drag(12);

        assert!(!state.global.tab_percent_save_pending());
        assert_eq!(
            state.global.bottom_tab,
            BottomTab::GitStatus,
            "an unmoved release on the line selects no tab"
        );
    }

    // ─── strip mouse scroll routing ──────────────────────────────

    #[test]
    fn a_strip_wheel_over_the_agent_panel_scrolls_the_agents() {
        // Column routing: column 10 is inside the drawn agent panel
        // (4..72), so the wheel scrolls the agent list and never the
        // tab panel — on any row, the strip has no bottom band.
        let mut state = strip_state();
        state.scrolls.panes = ScrollState {
            offset: 0,
            total_lines: 40,
            visible_height: 20,
        };
        state.activity.scroll = ScrollState {
            offset: 0,
            total_lines: 30,
            visible_height: 10,
        };

        state.handle_mouse_scroll(7, 10, 12, 0, 3);

        assert_eq!(state.scrolls.panes.offset, 3);
        assert_eq!(state.activity.scroll.offset, 0);
    }

    #[test]
    fn a_strip_wheel_over_the_tab_panel_scrolls_the_tab() {
        // Column routing: column 80 is inside the drawn tab panel
        // (73..120), so the wheel scrolls the displayed tab and never
        // the agent list; the divider line (column 72) scrolls nothing.
        let mut state = strip_state();
        state.scrolls.panes = ScrollState {
            offset: 0,
            total_lines: 40,
            visible_height: 20,
        };
        state.activity.scroll = ScrollState {
            offset: 0,
            total_lines: 30,
            visible_height: 10,
        };

        state.handle_mouse_scroll(7, 80, 12, 0, 3);
        assert_eq!(state.activity.scroll.offset, 3);
        assert_eq!(state.scrolls.panes.offset, 0);

        state.handle_mouse_scroll(7, 72, 12, 0, 3);
        assert_eq!(
            state.activity.scroll.offset, 3,
            "the divider has nothing to scroll"
        );
    }

    // ─── move_pane_selection edge cases ─────────────────────────────

    #[test]
    fn move_pane_selection_returns_false_when_empty() {
        let mut state = AppState::new("%99".into());
        assert!(!state.move_pane_selection(1));
        assert!(!state.move_pane_selection(-1));
    }

    #[test]
    fn move_pane_selection_boundary_returns() {
        let mut state = AppState::new("%99".into());
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
            RowTarget {
                pane_id: "%3".into(),
            },
        ];
        state.global.selected_pane_row = 0;

        assert!(!state.move_pane_selection(-1), "can't go below 0");
        assert!(state.move_pane_selection(1));
        assert!(state.move_pane_selection(1));
        assert_eq!(state.global.selected_pane_row, 2);
        assert!(!state.move_pane_selection(1), "can't go past end");
    }

    // ─── rebuild_row_targets clamp tests ────────────────────────────

    #[test]
    fn rebuild_row_targets_clamps_selection_when_shrinks() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (test_pane("%1"), PaneGitInfo::default()),
                (test_pane("%2"), PaneGitInfo::default()),
                (test_pane("%3"), PaneGitInfo::default()),
            ],
        }];
        state.global.selected_pane_row = 2;
        state.rebuild_row_targets();
        assert_eq!(state.global.selected_pane_row, 2);

        // Shrink to 1 pane
        state.repo_groups[0].panes = vec![(test_pane("%1"), PaneGitInfo::default())];
        state.rebuild_row_targets();
        assert_eq!(
            state.global.selected_pane_row, 0,
            "should clamp to last valid index"
        );
    }

    #[test]
    fn rebuild_row_targets_empty_groups() {
        let mut state = AppState::new("%99".into());
        state.global.selected_pane_row = 5;
        state.repo_groups = vec![];
        state.rebuild_row_targets();
        assert!(state.layout.pane_row_targets.is_empty());
        // selected_pane_row stays as-is when targets empty (no clamp needed)
        assert_eq!(state.global.selected_pane_row, 5);
    }

    #[test]
    fn rebuild_row_targets_respects_filter() {
        let mut state = AppState::new("%99".into());
        let mut p1 = test_pane("%1");
        p1.status = PaneStatus::Running;
        let mut p2 = test_pane("%2");
        p2.status = PaneStatus::Idle;
        let mut p3 = test_pane("%3");
        p3.status = PaneStatus::Running;

        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (p1, PaneGitInfo::default()),
                (p2, PaneGitInfo::default()),
                (p3, PaneGitInfo::default()),
            ],
        }];

        // All filter: all 3 panes
        state.global.status_filter = StatusFilter::All;
        state.rebuild_row_targets();
        assert_eq!(state.layout.pane_row_targets.len(), 3);

        // Busy filter: only 2 panes
        state.global.status_filter = StatusFilter::Busy;
        state.rebuild_row_targets();
        assert_eq!(state.layout.pane_row_targets.len(), 2);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");
        assert_eq!(state.layout.pane_row_targets[1].pane_id, "%3");

        // Idle filter: only 1 pane
        state.global.status_filter = StatusFilter::Idle;
        state.rebuild_row_targets();
        assert_eq!(state.layout.pane_row_targets.len(), 1);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%2");

        // Error filter: no panes
        state.global.status_filter = StatusFilter::Error;
        state.rebuild_row_targets();
        assert!(state.layout.pane_row_targets.is_empty());
    }

    #[test]
    fn rebuild_row_targets_clamps_cursor_on_filter_change() {
        let mut state = AppState::new("%99".into());
        let mut p1 = test_pane("%1");
        p1.status = PaneStatus::Running;
        let mut p2 = test_pane("%2");
        p2.status = PaneStatus::Idle;
        let mut p3 = test_pane("%3");
        p3.status = PaneStatus::Idle;

        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (p1, PaneGitInfo::default()),
                (p2, PaneGitInfo::default()),
                (p3, PaneGitInfo::default()),
            ],
        }];

        // Select last agent in All view
        state.global.status_filter = StatusFilter::All;
        state.rebuild_row_targets();
        state.global.selected_pane_row = 2;

        // Switch to Busy filter (only 1 pane) — cursor should clamp
        state.global.status_filter = StatusFilter::Busy;
        state.rebuild_row_targets();
        assert_eq!(state.global.selected_pane_row, 0);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");
    }

    // ─── handle_mouse_click tests ────────────────────────────────────

    #[test]
    fn mouse_click_selects_agent_row() {
        let mut state = AppState::new("%99".into());
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
        ];
        // line_to_row: line 0 = group header (None), line 1 = agent 0, line 2 = agent 1
        state.layout.line_to_row = vec![None, Some(0), Some(1)];
        state.scrolls.panes.offset = 0;

        // row 0 = filter bar, row 1 = secondary header, row 2+ = agent list rows
        state.handle_mouse_click(3, 5); // row 3 → line_index = (3-2) = 1 → agent row 0
        assert_eq!(state.global.selected_pane_row, 0);

        state.handle_mouse_click(4, 5); // row 4 → line_index = (4-2) = 2 → agent row 1
        assert_eq!(state.global.selected_pane_row, 1);
    }

    #[test]
    fn mouse_click_on_filter_bar_changes_filter() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.layout.pane_row_targets = vec![RowTarget {
            pane_id: "%1".into(),
        }];
        state.layout.line_to_row = vec![None, Some(0)];
        state.global.selected_pane_row = 0;
        state.global.status_filter = StatusFilter::All;

        // Click on "All" (x=1..3) should keep All
        reset_filter_debounce(&mut state);
        state.handle_mouse_click(0, 1);
        assert_eq!(state.global.status_filter, StatusFilter::All);

        // Click on the busy icon area (x=5..) should switch to Busy
        reset_filter_debounce(&mut state);
        state.handle_mouse_click(0, 5);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);

        // agent selection unchanged
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn mouse_click_on_secondary_header_toggles_repo_popup() {
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
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.layout.repo_button_col = Some(20);

        state.handle_mouse_click(1, 19);
        assert!(!state.is_repo_popup_open());

        state.handle_mouse_click(1, 20);
        assert!(state.is_repo_popup_open());
    }

    #[test]
    fn mouse_click_on_repo_popup_title_row_does_not_confirm() {
        // Regression: clicking the popup's top border/title row used
        // to collapse to `item_index == 0` via `saturating_sub(1)` and
        // immediately switch the filter to the first repo.
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
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.global.repo_filter = RepoFilter::Repo("beta".into());
        state.popup = PopupState::Repo {
            selected: 2,
            area: Some(ratatui::layout::Rect::new(0, 3, 20, 5)),
        };

        // Click the top border row (row == area.y = 3).
        state.handle_mouse_click(3, 5);

        assert!(
            state.is_repo_popup_open(),
            "title-row click must keep the popup open"
        );
        assert_eq!(
            state.global.repo_filter,
            RepoFilter::Repo("beta".into()),
            "title-row click must not confirm a selection"
        );
    }

    #[test]
    fn mouse_click_on_repo_popup_item_row_confirms_selection() {
        // Companion to the regression test above: clicks on the item
        // rows (area.y + 1, area.y + 2, …) should still confirm.
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
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.popup = PopupState::Repo {
            selected: 0,
            area: Some(ratatui::layout::Rect::new(0, 3, 20, 5)),
        };

        // Click row area.y + 1 (first list entry = "All").
        state.handle_mouse_click(4, 5);

        assert!(!state.is_repo_popup_open());
        assert_eq!(state.global.repo_filter, RepoFilter::All);
    }

    #[test]
    fn mouse_click_with_scroll_offset() {
        let mut state = AppState::new("%99".into());
        state.layout.pane_row_targets = vec![
            RowTarget {
                pane_id: "%1".into(),
            },
            RowTarget {
                pane_id: "%2".into(),
            },
        ];
        // 5 lines total, scrolled down by 2
        state.layout.line_to_row = vec![None, Some(0), Some(0), None, Some(1)];
        state.scrolls.panes.offset = 2;

        // row 4 → line_index = (4-2) + 2 = 4 → agent row 1
        state.handle_mouse_click(4, 5);
        assert_eq!(state.global.selected_pane_row, 1);
    }

    #[test]
    fn mouse_click_out_of_bounds() {
        let mut state = AppState::new("%99".into());
        state.layout.pane_row_targets = vec![RowTarget {
            pane_id: "%1".into(),
        }];
        state.layout.line_to_row = vec![None, Some(0)];
        state.global.selected_pane_row = 0;

        state.handle_mouse_click(50, 5); // way beyond line_to_row
        assert_eq!(state.global.selected_pane_row, 0); // unchanged
    }

    // ─── StatusFilter tests live in state/filter.rs ──────────────────

    // ─── status_counts tests ─────────────────────────────────────────

    #[test]
    fn status_counts_empty() {
        let state = AppState::new("%99".into());
        let counts = state.status_counts();
        assert_eq!(counts.count(StatusFilter::All), 0);
        for filter in StatusFilter::VARIANTS {
            assert_eq!(
                counts.count(filter),
                0,
                "{filter:?} count on an empty state"
            );
        }
    }

    #[test]
    fn status_counts_mixed() {
        let mut state = AppState::new("%99".into());
        let mut p1 = test_pane("%1");
        p1.status = PaneStatus::Running;
        let mut p2 = test_pane("%2");
        p2.status = PaneStatus::Background;
        let mut p3 = test_pane("%3");
        p3.status = PaneStatus::Idle;
        let mut p4 = test_pane("%4");
        p4.status = PaneStatus::Waiting;
        let mut p5 = test_pane("%5");
        p5.status = PaneStatus::Error;

        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (p1, PaneGitInfo::default()),
                (p2, PaneGitInfo::default()),
                (p3, PaneGitInfo::default()),
                (p4, PaneGitInfo::default()),
                (p5, PaneGitInfo::default()),
            ],
        }];
        // busy sums running + background
        let counts = state.status_counts();
        assert_eq!(counts.count(StatusFilter::All), 5);
        assert_eq!(counts.count(StatusFilter::Busy), 2);
        assert_eq!(counts.count(StatusFilter::Waiting), 1);
        assert_eq!(counts.count(StatusFilter::Idle), 1);
        assert_eq!(counts.count(StatusFilter::Error), 1);
    }

    // ─── handle_filter_click tests ───────────────────────────────────

    #[test]
    fn filter_click_all_positions() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        // With 0 agents, counts are all 0, so layout: " 󱓯 0  0 󰥔 0 󰒲 0  0"
        //                                              0123456789...

        // "󱓯 0" at x=1..3
        state.global.status_filter = StatusFilter::Busy;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(1);
        assert_eq!(state.global.status_filter, StatusFilter::All);

        reset_filter_debounce(&mut state);
        state.handle_filter_click(3);
        assert_eq!(state.global.status_filter, StatusFilter::All);

        // " 0" at x=5..7
        reset_filter_debounce(&mut state);
        state.handle_filter_click(5);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);

        // "󰥔 0" at x=9..11
        reset_filter_debounce(&mut state);
        state.handle_filter_click(9);
        assert_eq!(state.global.status_filter, StatusFilter::Waiting);

        // "󰒲 0" at x=13..15
        reset_filter_debounce(&mut state);
        state.handle_filter_click(13);
        assert_eq!(state.global.status_filter, StatusFilter::Idle);

        // " 0" at x=17..19
        reset_filter_debounce(&mut state);
        state.handle_filter_click(17);
        assert_eq!(state.global.status_filter, StatusFilter::Error);
    }

    #[test]
    fn filter_click_on_the_side_switch_routes_to_the_switch_action() {
        // The switch column (26 of 30) wins before the chip walk: the
        // click neither changes the status filter nor reaches the
        // pop-out branch. On the isolated test tmux the docked switch
        // sweeps a server with no sidebars and stays silent.
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.global.status_filter = StatusFilter::All;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(26);
        assert_eq!(state.global.status_filter, StatusFilter::All);
        assert!(state.flash.is_none());

        // Floating, the same click is a no-op — while the neighbouring
        // pop-out column would flash a refusal, so silence here proves
        // the switch branch ran (and did nothing) rather than the
        // pop-out one.
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.sidebar_pane.floating = true;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(26);
        assert!(state.flash.is_none());
        assert!(state.sidebar_pane.floating);
    }

    #[test]
    fn filter_click_gap_does_nothing() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.global.status_filter = StatusFilter::All;

        // x=0 is leading space, x=4/8/12/16 are separators
        state.handle_filter_click(0);
        assert_eq!(state.global.status_filter, StatusFilter::All);

        state.handle_filter_click(4);
        assert_eq!(state.global.status_filter, StatusFilter::All);

        state.handle_filter_click(8);
        assert_eq!(state.global.status_filter, StatusFilter::All);
    }

    #[test]
    fn filter_click_debounce_ignores_rapid_clicks() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        state.global.status_filter = StatusFilter::All;

        // First click within debounce window should be ignored
        // (AppState::new sets last_filter_click to now)
        state.handle_filter_click(5); // would be Busy
        assert_eq!(state.global.status_filter, StatusFilter::All); // unchanged due to debounce

        // After resetting debounce, click should work
        reset_filter_debounce(&mut state);
        state.handle_filter_click(5);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);

        // Immediate second click should be debounced
        state.handle_filter_click(1); // would be All
        assert_eq!(state.global.status_filter, StatusFilter::Busy); // unchanged
    }

    #[test]
    fn filter_click_with_large_counts() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        // Add 10 running agents to shift positions
        let panes: Vec<_> = (0..10)
            .map(|i| {
                let mut p = test_pane(&format!("%{i}"));
                p.status = PaneStatus::Running;
                (p, PaneGitInfo::default())
            })
            .collect();
        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes,
        }];
        // Layout: " 󱓯 10  10 󰥔 0 󰒲 0  0"
        //          0123456789...
        // " 10" at x=6..9 (icon + "10")
        reset_filter_debounce(&mut state);
        state.handle_filter_click(6);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);
        reset_filter_debounce(&mut state);
        state.handle_filter_click(9);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);

        // "󰥔 0" shifts to x=11..13
        reset_filter_debounce(&mut state);
        state.handle_filter_click(11);
        assert_eq!(state.global.status_filter, StatusFilter::Waiting);
    }

    #[test]
    fn filter_click_rebuilds_row_targets() {
        let mut state = AppState::new("%99".into());
        state.layout.filter_bar_width = 30;
        let mut p1 = test_pane("%1");
        p1.status = PaneStatus::Running;
        let mut p2 = test_pane("%2");
        p2.status = PaneStatus::Idle;
        let mut p3 = test_pane("%3");
        p3.status = PaneStatus::Running;
        state.repo_groups = vec![RepoGroup {
            name: "project".into(),
            has_focus: true,
            panes: vec![
                (p1, PaneGitInfo::default()),
                (p2, PaneGitInfo::default()),
                (p3, PaneGitInfo::default()),
            ],
        }];
        state.global.status_filter = StatusFilter::All;
        state.rebuild_row_targets();
        assert_eq!(state.layout.pane_row_targets.len(), 3);

        // Click Busy filter — row_targets should update immediately
        // Layout: " 󱓯 3  2 󰥔 0 󰒲 1  0" → Busy at x=5
        reset_filter_debounce(&mut state);
        state.handle_filter_click(5);
        assert_eq!(state.global.status_filter, StatusFilter::Busy);
        assert_eq!(state.layout.pane_row_targets.len(), 2);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");
        assert_eq!(state.layout.pane_row_targets[1].pane_id, "%3");

        // Click Idle filter — row_targets should update again
        // Layout: " 󱓯 3  2 󰥔 0 󰒲 1  0" → Idle at x=13
        reset_filter_debounce(&mut state);
        state.handle_filter_click(13);
        assert_eq!(state.global.status_filter, StatusFilter::Idle);
        assert_eq!(state.layout.pane_row_targets.len(), 1);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%2");
    }

    #[test]
    fn filter_click_on_the_popout_control_pops_out_and_keeps_the_filter() {
        // The control is hit-tested before the chip walk: a click on its
        // column runs the pop-out action and never touches the status
        // filter. The pane id cannot exist in any tmux server, so the
        // pane probe fails and the action reports the unreadable window
        // size instead of acting — the flash is the proof that the click
        // reached the pop-out (the refusal flash itself is covered by
        // the injected popout tests).
        let mut state = AppState::new("%4294967295".into());
        state.layout.filter_bar_width = 20;
        state.global.status_filter = StatusFilter::All;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(18);
        assert_eq!(state.global.status_filter, StatusFilter::All);
        assert!(
            state
                .flash
                .as_ref()
                .is_some_and(|(msg, _)| msg.starts_with("Pop-out unavailable:")),
            "clicking the control must reach the pop-out action"
        );

        // A click on a chip column still walks the chips and selects the
        // matching category even with the control column registered, and
        // it must not pop out.
        state.flash = None;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(5); // busy chip
        assert_eq!(state.global.status_filter, StatusFilter::Busy);
        assert!(state.flash.is_none(), "a chip click must not pop out");
    }

    #[test]
    fn filter_click_before_the_first_frame_does_nothing() {
        // Before the first frame the row width is unknown (0): no chip
        // and no control is clickable, and nothing toggles.
        let mut state = AppState::new("%4294967295".into());
        assert_eq!(state.layout.filter_bar_width, 0);
        state.global.status_filter = StatusFilter::All;
        reset_filter_debounce(&mut state);
        state.handle_filter_click(4);
        // The second click must really reach the zero-width path: without
        // resetting the debounce it is swallowed as a phantom double click.
        reset_filter_debounce(&mut state);
        state.handle_filter_click(19);
        assert_eq!(state.global.status_filter, StatusFilter::All);
        assert!(state.flash.is_none());
    }

    // ─── StatusFilter / RepoFilter pure tests live in state/filter.rs ─

    #[test]
    fn repo_filter_all_shows_all_groups() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "dotfiles".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())],
            },
            RepoGroup {
                name: "app".into(),
                has_focus: false,
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.global.repo_filter = RepoFilter::All;
        state.rebuild_row_targets();

        assert_eq!(state.layout.pane_row_targets.len(), 2);
    }

    #[test]
    fn repo_filter_specific_repo() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "dotfiles".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())],
            },
            RepoGroup {
                name: "app".into(),
                has_focus: false,
                panes: vec![(test_pane("%2"), PaneGitInfo::default())],
            },
        ];
        state.global.repo_filter = RepoFilter::Repo("app".into());
        state.rebuild_row_targets();

        assert_eq!(state.layout.pane_row_targets.len(), 1);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%2");
    }

    #[test]
    fn repo_filter_combined_with_status() {
        let mut state = AppState::new("%99".into());
        let mut idle_pane = test_pane("%3");
        idle_pane.status = PaneStatus::Idle;
        state.repo_groups = vec![
            RepoGroup {
                name: "app".into(),
                has_focus: true,
                panes: vec![
                    (test_pane("%1"), PaneGitInfo::default()), // Running
                    (idle_pane, PaneGitInfo::default()),       // Idle
                ],
            },
            RepoGroup {
                name: "lib".into(),
                has_focus: false,
                panes: vec![(test_pane("%2"), PaneGitInfo::default())], // Running
            },
        ];
        state.global.repo_filter = RepoFilter::Repo("app".into());
        state.global.status_filter = StatusFilter::Busy;
        state.rebuild_row_targets();

        // Only busy (running) panes in "app" group
        assert_eq!(state.layout.pane_row_targets.len(), 1);
        assert_eq!(state.layout.pane_row_targets[0].pane_id, "%1");
    }

    #[test]
    fn repo_filter_stale_name_resets() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![RepoGroup {
            name: "app".into(),
            has_focus: true,
            panes: vec![(test_pane("%1"), PaneGitInfo::default())],
        }];
        state.global.repo_filter = RepoFilter::Repo("deleted-repo".into());
        state.rebuild_row_targets();

        assert_eq!(state.global.repo_filter, RepoFilter::All);
        assert_eq!(state.layout.pane_row_targets.len(), 1);
    }

    #[test]
    fn repo_names_returns_all_plus_groups() {
        let mut state = AppState::new("%99".into());
        state.repo_groups = vec![
            RepoGroup {
                name: "alpha".into(),
                has_focus: true,
                panes: vec![],
            },
            RepoGroup {
                name: "beta".into(),
                has_focus: false,
                panes: vec![],
            },
        ];
        assert_eq!(state.repo_names(), vec!["All", "alpha", "beta"]);
    }

    #[test]
    fn status_counts_respects_repo_filter() {
        let mut state = AppState::new("%99".into());
        let mut idle_pane = test_pane("%2");
        idle_pane.status = PaneStatus::Idle;
        state.repo_groups = vec![
            RepoGroup {
                name: "app".into(),
                has_focus: true,
                panes: vec![(test_pane("%1"), PaneGitInfo::default())], // Running
            },
            RepoGroup {
                name: "lib".into(),
                has_focus: false,
                panes: vec![(idle_pane, PaneGitInfo::default())], // Idle
            },
        ];

        // All repos: 2 total
        state.global.repo_filter = RepoFilter::All;
        let counts = state.status_counts();
        assert_eq!(counts.count(StatusFilter::All), 2);
        assert_eq!(counts.count(StatusFilter::Busy), 1);
        assert_eq!(counts.count(StatusFilter::Idle), 1);

        // Filter to "app" only: 1 Running
        state.global.repo_filter = RepoFilter::Repo("app".into());
        let counts = state.status_counts();
        assert_eq!(counts.count(StatusFilter::All), 1);
        assert_eq!(counts.count(StatusFilter::Busy), 1);
        assert_eq!(counts.count(StatusFilter::Idle), 0);
    }
}
