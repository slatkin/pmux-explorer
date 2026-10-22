use std::collections::HashSet;
use std::time::Duration;

use crate::activity::{self, TaskProgress};
use crate::cli::sanitize_tmux_value;
use crate::process::ProcessSnapshot;
use crate::task_store;
use crate::tmux::{self, PaneStatus, SessionInfo};

use super::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskProgressDecision {
    Clear,
    Show,
    Dismiss { total: usize },
    Skip,
}

/// A per-pane task-progress update computed in the first pass of
/// `refresh_task_progress`, applied back to `pane_states` in the second pass.
struct PaneTaskUpdate {
    pane_id: String,
    progress: Option<TaskProgress>,
    dismissed_total: Option<usize>,
    inactive_since: Option<u64>,
    log_mtime: Option<std::time::SystemTime>,
}

pub(crate) fn classify_task_progress(
    progress: &TaskProgress,
    dismissed_total: Option<usize>,
) -> TaskProgressDecision {
    if progress.is_empty() {
        return TaskProgressDecision::Clear;
    }
    if progress.all_completed() {
        if dismissed_total == Some(progress.total()) {
            TaskProgressDecision::Skip
        } else {
            TaskProgressDecision::Dismiss {
                total: progress.total(),
            }
        }
    } else {
        TaskProgressDecision::Show
    }
}

impl AppState {
    pub(crate) fn refresh_now(&mut self) {
        self.now = crate::time::now_epoch_secs();
    }

    pub(crate) fn apply_session_snapshot(
        &mut self,
        sidebar_focused: bool,
        sessions: Vec<SessionInfo>,
    ) {
        self.focus_state.sidebar_focused = sidebar_focused;
        self.repo_groups = crate::group::group_panes_by_repo(&sessions);
        self.prune_pane_states_to_current_panes();
        self.rebuild_row_targets();
        self.find_focused_pane();
    }

    fn clear_dead_agent_metadata(pane_id: &str) {
        // The lifecycle set from `tmux::AGENT_SESSION_PANE_OPTIONS`, minus
        // the pending-teardown markers: the hook-side drain owns those,
        // and a fresh SessionStart clears both on the next run.
        for key in tmux::AGENT_SESSION_PANE_OPTIONS {
            if matches!(
                *key,
                tmux::PANE_PENDING_SESSION_END | tmux::PANE_PENDING_WORKTREE_REMOVE
            ) {
                continue;
            }
            tmux::unset_pane_option(pane_id, key);
        }

        let _ = std::fs::remove_file(activity::log_file_path(pane_id));
    }

    fn filter_sessions_to_live_agent_panes(
        sessions: Vec<SessionInfo>,
        live_agent_panes: &HashSet<String>,
    ) -> Vec<SessionInfo> {
        let mut out = Vec::new();
        for mut session in sessions {
            let mut windows = Vec::new();
            for mut window in session.windows {
                window
                    .panes
                    .retain(|pane| live_agent_panes.contains(&pane.pane_id));
                if !window.panes.is_empty() {
                    windows.push(window);
                }
            }
            if !windows.is_empty() {
                session.windows = windows;
                out.push(session);
            }
        }
        out
    }

    fn refresh_activity_data(&mut self) {
        self.refresh_activity_log();
        self.refresh_pane_tasks();
        self.refresh_task_progress();
        self.auto_switch_tab();
    }

    /// Fast refresh: tmux state + activity log (called every 1s).
    /// Returns whether the sidebar's window is the active tmux window.
    pub fn refresh(&mut self) -> bool {
        self.refresh_now();
        // Retry a filter write tmux rejected before reading the shared
        // options, so the read below sees the value this instance just
        // tried to write instead of the stale one it replaces.
        self.global.retry_pending_filter_save();
        let reading = tmux::get_sidebar_pane_reading(&self.tmux_pane);
        self.sidebar_pane = reading.info;
        // The shared settings ride along on the same per-tick
        // `display-message` call: applying them here means the
        // `rebuild_row_targets` inside `apply_session_snapshot` already
        // sees the filters another instance wrote. `apply_all` guards
        // every field with its matching `last_saved_*` value, so a local
        // change that tmux already holds is not reverted.
        self.global.apply_all(&reading.options);
        // The flags are read back from the state (not from a local probe
        // copy) so every field the state holds has a reader.
        let focused = self.sidebar_pane.pane_active;
        let window_active = self.sidebar_pane.window_active;
        let (mut sessions, mut process_snapshot) = tmux::query_sessions_with_process_snapshot();
        self.sweep_dead_bg_shells_if_due(&mut sessions, &mut process_snapshot);
        if let Some(process_snapshot) = self.refresh_port_data(&sessions, process_snapshot.as_ref())
        {
            let sessions = Self::filter_sessions_to_live_agent_panes(
                sessions,
                &process_snapshot.live_agent_panes,
            );
            self.apply_session_snapshot(focused, sessions);
        } else {
            self.apply_session_snapshot(focused, sessions);
        }
        self.refresh_activity_data();
        window_active
    }

    pub(crate) fn refresh_port_data(
        &mut self,
        sessions: &[SessionInfo],
        process_snapshot: Option<&ProcessSnapshot>,
    ) -> Option<crate::port::PaneProcessSnapshot> {
        const PORT_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

        if !self.timers.port_scan_initialized
            || self.timers.last_port_refresh.elapsed() >= PORT_REFRESH_INTERVAL
        {
            let scanned = crate::port::scan_session_process_snapshot(sessions, process_snapshot)?;
            let mut updates: Vec<(String, Vec<u16>, Option<String>)> = Vec::new();
            let mut dead_panes: Vec<String> = Vec::new();
            for session in sessions {
                for window in &session.windows {
                    for pane in &window.panes {
                        if !scanned.live_agent_panes.contains(&pane.pane_id) {
                            dead_panes.push(pane.pane_id.clone());
                        }
                        updates.push((
                            pane.pane_id.clone(),
                            scanned
                                .ports_by_pane
                                .get(&pane.pane_id)
                                .cloned()
                                .unwrap_or_default(),
                            scanned.command_by_pane.get(&pane.pane_id).cloned(),
                        ));
                    }
                }
            }
            for (pane_id, ports, command) in updates {
                let pane_state = self.pane_state_mut(&pane_id);
                pane_state.ports = ports;
                pane_state.command = command;
            }
            for pane_id in dead_panes {
                Self::clear_dead_agent_metadata(&pane_id);
                self.clear_pane_state(&pane_id);
            }
            self.timers.port_scan_initialized = true;
            self.timers.last_port_refresh = std::time::Instant::now();
            return Some(scanned);
        }

        None
    }

    pub(crate) fn refresh_task_progress(&mut self) {
        let mut updates: Vec<PaneTaskUpdate> = Vec::new();
        for group in &self.repo_groups {
            for (pane, _) in &group.panes {
                let prior_state = self.pane_state(&pane.pane_id).cloned().unwrap_or_default();
                let current_mtime = activity::log_mtime(&pane.pane_id);
                // Skip the (full-file) re-parse when the activity log
                // hasn't been touched since the last tick AND the pane
                // is still active. We must still re-evaluate the
                // inactive-grace path while the agent is idle so that a
                // long-stalled progress bar gets dismissed even if the
                // log file itself stops changing.
                let agent_active = pane.status.is_active();
                let log_unchanged =
                    current_mtime.is_some() && current_mtime == prior_state.task_progress_log_mtime;
                if log_unchanged && agent_active {
                    // Just refresh the mtime bookkeeping so we don't
                    // accidentally drop the cache on a future iteration
                    // where current_mtime suddenly becomes None (e.g.
                    // /tmp clean-up). All other prior_state fields
                    // remain authoritative.
                    updates.push(PaneTaskUpdate {
                        pane_id: pane.pane_id.clone(),
                        progress: prior_state.task_progress.clone(),
                        dismissed_total: prior_state.task_dismissed_total,
                        inactive_since: None,
                        log_mtime: current_mtime,
                    });
                    continue;
                }
                // Read all entries for task progress (not limited to display max)
                // so that TaskCreate entries aren't lost when subagents flood the log
                let entries = activity::read_activity_log(&pane.pane_id, 0);
                let progress = activity::parse_task_progress(&entries);
                // Debounce inactive→dismiss transition to avoid flicker.
                //
                // The agent status can briefly drop to idle during normal operation
                // (e.g. when Claude Code processes a system prompt or between tool
                // calls). Without a grace period, the 1-second refresh cycle can
                // catch that transient idle state and immediately hide the task
                // progress bar, causing a visible flicker.
                //
                // We track when each pane first appeared inactive and only dismiss
                // after INACTIVE_GRACE_SECS have elapsed. If the agent returns to
                // Running/Waiting within that window, the timer is reset.
                const INACTIVE_GRACE_SECS: u64 = 3;

                let next_inactive_since = if !agent_active {
                    Some(prior_state.inactive_since.unwrap_or(self.now))
                } else {
                    None
                };
                let grace_expired = next_inactive_since
                    .is_some_and(|since| self.now.saturating_sub(since) >= INACTIVE_GRACE_SECS);

                let decision = if grace_expired && !progress.is_empty() && !progress.all_completed()
                {
                    TaskProgressDecision::Dismiss {
                        total: progress.total(),
                    }
                } else {
                    classify_task_progress(&progress, prior_state.task_dismissed_total)
                };
                let next_progress = match decision {
                    TaskProgressDecision::Clear => None,
                    TaskProgressDecision::Show => Some(progress),
                    TaskProgressDecision::Dismiss { .. } => None,
                    TaskProgressDecision::Skip => prior_state.task_progress.clone(),
                };
                let next_dismissed_total = match decision {
                    TaskProgressDecision::Clear | TaskProgressDecision::Show => None,
                    TaskProgressDecision::Dismiss { total } => Some(total),
                    TaskProgressDecision::Skip => prior_state.task_dismissed_total,
                };
                updates.push(PaneTaskUpdate {
                    pane_id: pane.pane_id.clone(),
                    progress: next_progress,
                    dismissed_total: next_dismissed_total,
                    inactive_since: next_inactive_since,
                    log_mtime: current_mtime,
                });
            }
        }
        for update in updates {
            let pane_state = self.pane_state_mut(&update.pane_id);
            pane_state.inactive_since = update.inactive_since;
            pane_state.task_dismissed_total = update.dismissed_total;
            pane_state.task_progress = update.progress;
            pane_state.task_progress_log_mtime = update.log_mtime;
        }
    }

    /// Run the background-shell liveness sweep at most once per
    /// `BG_SHELL_SWEEP_INTERVAL`. The first call always runs so the
    /// initial pane state is accurate.
    fn sweep_dead_bg_shells_if_due(
        &mut self,
        sessions: &mut [SessionInfo],
        process_snapshot: &mut Option<ProcessSnapshot>,
    ) {
        const BG_SHELL_SWEEP_INTERVAL: Duration = Duration::from_secs(5);
        let should_run = self
            .timers
            .last_bg_shell_sweep
            .is_none_or(|last| last.elapsed() >= BG_SHELL_SWEEP_INTERVAL);
        if !should_run {
            return;
        }
        sweep_dead_bg_shells(sessions, process_snapshot);
        self.timers.last_bg_shell_sweep = Some(std::time::Instant::now());
    }

    /// Refresh the focused pane's task store with the same mtime check
    /// as `refresh_activity_log`: skip the re-read when neither the
    /// focused pane nor the store file's mtime has changed.
    pub(crate) fn refresh_pane_tasks(&mut self) {
        let Some(ref pane_id) = self.focus_state.focused_pane_id else {
            self.tasks.items.clear();
            self.tasks.store_cache = None;
            return;
        };
        let current_mtime = task_store::store_mtime(pane_id);
        if let (Some(mtime), Some((cached_id, cached_mtime))) =
            (current_mtime, self.tasks.store_cache.as_ref())
            && cached_id == pane_id
            && *cached_mtime == mtime
        {
            return;
        }
        self.tasks.items = task_store::read_tasks(pane_id);
        self.tasks.store_cache = current_mtime.map(|m| (pane_id.clone(), m));
    }

    pub(crate) fn refresh_activity_log(&mut self) {
        let Some(ref pane_id) = self.focus_state.focused_pane_id else {
            self.activity.entries.clear();
            self.activity.log_cache = None;
            return;
        };
        let current_mtime = activity::log_mtime(pane_id);
        if let (Some(mtime), Some((cached_id, cached_mtime))) =
            (current_mtime, self.activity.log_cache.as_ref())
            && cached_id == pane_id
            && *cached_mtime == mtime
        {
            return;
        }
        // Task-reset markers are internal bookkeeping for parse_task_progress;
        // they should never appear in the user-facing Activity tab.
        let mut entries = activity::read_activity_log(pane_id, self.activity.max_entries);
        entries.retain(|e| e.tool != activity::TASK_RESET_MARKER);
        self.activity.entries = entries;
        self.activity.log_cache = current_mtime.map(|m| (pane_id.clone(), m));
    }
}

pub(crate) fn sweep_dead_bg_shells(
    sessions: &mut [SessionInfo],
    process_snapshot: &mut Option<ProcessSnapshot>,
) {
    let has_any = sessions
        .iter()
        .flat_map(|s| s.windows.iter())
        .flat_map(|w| w.panes.iter())
        .any(|p| p.bg_shell_cmd.is_some());
    if !has_any {
        return;
    }
    if process_snapshot.is_none() {
        *process_snapshot = ProcessSnapshot::scan();
    }
    if let Some(snapshot) = process_snapshot.as_ref() {
        clear_dead_bg_shells(sessions, snapshot);
    }
}

pub(crate) fn clear_dead_bg_shells(
    sessions: &mut [SessionInfo],
    process_snapshot: &ProcessSnapshot,
) {
    for session in sessions.iter_mut() {
        for window in &mut session.windows {
            for pane in &mut window.panes {
                let Some(cmd) = pane.bg_shell_cmd.as_deref() else {
                    continue;
                };
                if cmd == tmux::BG_CMD_PLACEHOLDER {
                    continue;
                }
                let Some(pane_pid) = pane.pane_pid else {
                    continue;
                };
                if process_snapshot
                    .command_lines_for_tree(&[pane_pid])
                    .iter()
                    .map(|line| sanitize_tmux_value(line))
                    .any(|line| ps_line_matches_cmd(&line, cmd))
                {
                    continue;
                }
                tmux::unset_pane_option(&pane.pane_id, tmux::PANE_BG_CMD);
                if pane.status == PaneStatus::Background {
                    tmux::set_pane_option(&pane.pane_id, tmux::PANE_STATUS, "idle");
                    pane.status = PaneStatus::Idle;
                }
                pane.bg_shell_cmd = None;
            }
        }
    }
}

/// Token-boundary substring match against a ps `command=` line that has
/// already been run through [`sanitize_tmux_value`]. Callers must pre-
/// normalize so the match sees the same `|`/`\n` → space canonicalization
/// applied when `@pane_bg_cmd` was stored; otherwise a piped bg command
/// (`tail -f log | grep X`) would miss on its first sweep.
///
/// Boundary rule treats `-`, `_`, `.` as part of the token (alongside
/// alphanumerics) so that a stored `"cargo-watch"` does not falsely
/// match `"cargo-watch-bin"` in ps. `/` is a boundary so a bare cmd
/// still matches when ps emits the full path (`/usr/local/bin/cargo-watch`).
fn ps_line_matches_cmd(normalized_line: &str, cmd: &str) -> bool {
    if cmd.is_empty() {
        return false;
    }
    let bytes = normalized_line.as_bytes();
    normalized_line.match_indices(cmd).any(|(idx, _)| {
        let end = idx + cmd.len();
        let before_ok = idx == 0 || !is_cmd_token_byte(bytes[idx - 1]);
        let after_ok = end == bytes.len() || !is_cmd_token_byte(bytes[end]);
        before_ok && after_ok
    })
}

fn is_cmd_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{
        AgentType, PaneInfo, PaneStatus, PermissionMode, SessionInfo, WindowInfo, WorktreeMetadata,
    };

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

    fn test_session(panes: Vec<PaneInfo>) -> Vec<SessionInfo> {
        vec![SessionInfo {
            session_name: "main".into(),
            windows: vec![WindowInfo {
                window_id: "@0".into(),
                window_name: "test".into(),
                window_active: true,
                auto_rename: false,
                panes,
            }],
        }]
    }

    // ─── clear_dead_bg_shells ───────────────────────────────────────

    fn pane_with_bg(id: &str, cmd: &str, status: PaneStatus) -> PaneInfo {
        let mut p = test_pane(id);
        p.bg_shell_cmd = Some(cmd.into());
        p.status = status;
        p.pane_pid = Some(100);
        p
    }

    fn process_snapshot(ps_out: &str) -> ProcessSnapshot {
        ProcessSnapshot::from_ps_output(ps_out)
    }

    #[test]
    fn clear_dead_bg_shells_retains_shell_present_in_ps_output() {
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_ALIVE";
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, "sleep 300");

        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            "sleep 300",
            PaneStatus::Background,
        )]);
        let snapshot = process_snapshot("100 1 zsh /bin/zsh -c eval 'sleep 300' < /dev/null\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        assert_eq!(
            sessions[0].windows[0].panes[0].bg_shell_cmd.as_deref(),
            Some("sleep 300"),
            "a matching ps line must leave the marker intact",
        );
        assert!(tmux::test_mock::contains(pane_id, tmux::PANE_BG_CMD));
    }

    #[test]
    fn clear_dead_bg_shells_ignores_matching_command_in_other_pane_tree() {
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_OTHER_PANE";
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, "sleep 300");
        tmux::test_mock::set(pane_id, tmux::PANE_STATUS, "background");

        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            "sleep 300",
            PaneStatus::Background,
        )]);
        let snapshot = process_snapshot("100 1 zsh /bin/zsh\n200 1 zsh /bin/zsh -c 'sleep 300'\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        let pane = &sessions[0].windows[0].panes[0];
        assert!(
            pane.bg_shell_cmd.is_none(),
            "a matching command outside the pane process tree must not keep the marker alive",
        );
        assert_eq!(pane.status, PaneStatus::Idle);
    }

    #[test]
    fn clear_dead_bg_shells_clears_when_shell_missing_and_downgrades_background() {
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_DEAD";
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, "sleep 300");
        tmux::test_mock::set(pane_id, tmux::PANE_STATUS, "background");

        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            "sleep 300",
            PaneStatus::Background,
        )]);
        // ps output contains nothing matching "sleep 300".
        let snapshot = process_snapshot("100 1 zsh /bin/zsh\n200 1 ssh /usr/bin/ssh host\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        let pane = &sessions[0].windows[0].panes[0];
        assert!(
            pane.bg_shell_cmd.is_none(),
            "local pane copy must be cleared so this tick's render reflects it",
        );
        assert_eq!(
            pane.status,
            PaneStatus::Idle,
            "background with a dead shell must downgrade to idle",
        );
        assert!(!tmux::test_mock::contains(pane_id, tmux::PANE_BG_CMD));
        assert_eq!(
            tmux::test_mock::get(pane_id, tmux::PANE_STATUS).as_deref(),
            Some("idle"),
        );
    }

    #[test]
    fn clear_dead_bg_shells_does_not_touch_non_background_status() {
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_STALE_RUNNING";
        tmux::test_mock::set(pane_id, tmux::PANE_STATUS, "running");
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, "npm run dev");

        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            "npm run dev",
            PaneStatus::Running,
        )]);
        let snapshot = process_snapshot("100 1 zsh /bin/zsh\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        let pane = &sessions[0].windows[0].panes[0];
        assert!(pane.bg_shell_cmd.is_none());
        assert_eq!(
            pane.status,
            PaneStatus::Running,
            "non-background status must be left alone",
        );
        assert_eq!(
            tmux::test_mock::get(pane_id, tmux::PANE_STATUS).as_deref(),
            Some("running"),
        );
    }

    #[test]
    fn clear_dead_bg_shells_preserves_placeholder_cmd() {
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_PLACEHOLDER";
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, tmux::BG_CMD_PLACEHOLDER);

        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            tmux::BG_CMD_PLACEHOLDER,
            PaneStatus::Background,
        )]);
        let snapshot = process_snapshot("");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        assert_eq!(
            sessions[0].windows[0].panes[0].bg_shell_cmd.as_deref(),
            Some(tmux::BG_CMD_PLACEHOLDER),
            "placeholder must survive — we cannot prove the shell is dead",
        );
        assert!(tmux::test_mock::contains(pane_id, tmux::PANE_BG_CMD));
    }

    #[test]
    fn clear_dead_bg_shells_treats_prefix_collision_as_dead() {
        // Regression: a naive `str::contains` match kept the marker
        // alive forever when a shorter stored cmd was a prefix of a
        // live longer cmd.
        let _guard = tmux::test_mock::install();
        let pane_id = "%BG_PREFIX_COLLIDE";
        tmux::test_mock::set(pane_id, tmux::PANE_BG_CMD, "sleep 3");
        let mut sessions = test_session(vec![pane_with_bg(
            pane_id,
            "sleep 3",
            PaneStatus::Background,
        )]);
        let snapshot = process_snapshot("100 1 zsh /bin/zsh -c 'sleep 30' < /dev/null\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        assert!(
            sessions[0].windows[0].panes[0].bg_shell_cmd.is_none(),
            "the marker for `sleep 3` must clear when only `sleep 30` is running",
        );
    }

    #[test]
    fn clear_dead_bg_shells_no_op_when_no_pane_has_bg_marker() {
        let _guard = tmux::test_mock::install();
        let mut sessions = test_session(vec![test_pane("%1")]);
        let snapshot = process_snapshot("100 1 zsh /bin/zsh\n");

        clear_dead_bg_shells(&mut sessions, &snapshot);

        assert!(sessions[0].windows[0].panes[0].bg_shell_cmd.is_none());
    }

    // ─── ps_line_matches_cmd ────────────────────────────────────────

    #[test]
    fn ps_line_matches_cmd_empty_cmd_never_matches() {
        assert!(!ps_line_matches_cmd("anything", ""));
        assert!(!ps_line_matches_cmd("", ""));
    }

    #[test]
    fn ps_line_matches_cmd_no_occurrence_is_false() {
        assert!(!ps_line_matches_cmd("/bin/zsh", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_full_line_match() {
        assert!(ps_line_matches_cmd("sleep 300", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_match_at_start() {
        assert!(ps_line_matches_cmd("sleep 300 --flag", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_match_at_end() {
        assert!(ps_line_matches_cmd("/bin/zsh -c sleep 300", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_rejects_trailing_alnum() {
        // Stored "sleep 3" must not match a live "sleep 30" process.
        assert!(!ps_line_matches_cmd("sleep 30", "sleep 3"));
        assert!(!ps_line_matches_cmd("/bin/zsh sleep 300 end", "sleep 3"));
    }

    #[test]
    fn ps_line_matches_cmd_rejects_leading_alnum() {
        // `mysleep 300` must not match `sleep 300`.
        assert!(!ps_line_matches_cmd("mysleep 300", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_accepts_non_alnum_boundary_chars() {
        // Quotes, parens, semicolons — all count as word boundaries.
        assert!(ps_line_matches_cmd(
            "/bin/zsh -c 'sleep 300' end",
            "sleep 300"
        ));
        assert!(ps_line_matches_cmd("(sleep 300);", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_multibyte_adjacent_treated_as_boundary() {
        // A non-ASCII char adjacent to the match must not panic and
        // must count as a boundary (the byte is not ASCII-alnum).
        assert!(ps_line_matches_cmd("🚀sleep 300", "sleep 300"));
        assert!(ps_line_matches_cmd("sleep 300🚀", "sleep 300"));
    }

    #[test]
    fn ps_line_matches_cmd_piped_cmd_matches_ps_line_with_pipe() {
        // Regression for Bug A: `sanitize_tmux_value` replaces `|` with
        // a space before writing `@pane_bg_cmd`, so the stored value
        // never contains a pipe. ps, however, emits the raw command line
        // with `|` intact — so callers must pre-normalize ps lines through
        // the same filter before this match can see the two sides as equal.
        let raw_line = "/bin/zsh -c 'tail -f log.txt | grep ERROR'";
        let normalized = sanitize_tmux_value(raw_line);
        let stored = "tail -f log.txt   grep ERROR"; // post-sanitize
        assert!(ps_line_matches_cmd(&normalized, stored));
    }

    #[test]
    fn ps_line_matches_cmd_newline_cmd_matches_ps_line() {
        // Same story for `\n` in the original command.
        let raw_line = "/bin/zsh -c 'echo one\necho two'";
        let normalized = sanitize_tmux_value(raw_line);
        let stored = "echo one echo two";
        assert!(ps_line_matches_cmd(&normalized, stored));
    }

    #[test]
    fn ps_line_matches_cmd_rejects_hyphenated_continuation() {
        // Regression for Bug B: `cargo-watch` must not match
        // `cargo-watch-bin` — `-` is part of the token, not a boundary.
        assert!(!ps_line_matches_cmd(
            "/usr/local/bin/cargo-watch-bin",
            "cargo-watch"
        ));
        assert!(!ps_line_matches_cmd("npm-run-all-ng foo", "npm-run-all"));
    }

    #[test]
    fn ps_line_matches_cmd_rejects_dot_continuation() {
        assert!(!ps_line_matches_cmd("node app.js.bak watch", "node app.js"));
    }

    #[test]
    fn ps_line_matches_cmd_accepts_path_prefixed_cmd() {
        // `/` stays a boundary so a bare `cargo-watch` still matches
        // the full-path form ps typically emits for installed binaries.
        assert!(ps_line_matches_cmd(
            "/usr/local/bin/cargo-watch",
            "cargo-watch"
        ));
        assert!(ps_line_matches_cmd(
            "./cargo-watch --watch src",
            "cargo-watch"
        ));
    }

    #[test]
    fn ps_line_matches_cmd_multiple_occurrences_one_valid_matches() {
        // If any occurrence satisfies the boundary check, return true.
        // First occurrence is glued to `mysleep 3`, second is standalone.
        assert!(ps_line_matches_cmd(
            "mysleep 30 /bin/sh sleep 30",
            "sleep 30"
        ));
    }

    #[test]
    fn clear_dead_agent_metadata_clears_the_subagent_identity() {
        // A killed pi subagent must not leave its identity options behind:
        // `@pane_agent` is wiped by the same sweep, and a next agent in
        // this pane would otherwise render as a pmux subagent with a
        // foreign task and lose its permission badge.
        let _guard = tmux::test_mock::install();
        let pane = "%DEAD_SUBAGENT";
        for key in tmux::AGENT_SESSION_PANE_OPTIONS {
            tmux::test_mock::set(pane, key, "x");
        }

        AppState::clear_dead_agent_metadata(pane);

        for key in tmux::AGENT_SESSION_PANE_OPTIONS {
            let preserved = matches!(
                *key,
                tmux::PANE_PENDING_SESSION_END | tmux::PANE_PENDING_WORKTREE_REMOVE
            );
            assert_eq!(
                tmux::test_mock::contains(pane, key),
                preserved,
                "expected {key} {}",
                if preserved { "preserved" } else { "cleared" }
            );
        }
    }

    #[test]
    fn filter_sessions_to_live_agent_panes_removes_dead_panes() {
        let sessions = test_session(vec![test_pane("%1"), test_pane("%2")]);
        let live = HashSet::from(["%2".to_string()]);

        let filtered = AppState::filter_sessions_to_live_agent_panes(sessions, &live);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].windows.len(), 1);
        assert_eq!(filtered[0].windows[0].panes.len(), 1);
        assert_eq!(filtered[0].windows[0].panes[0].pane_id, "%2");
    }

    #[test]
    fn filter_sessions_to_live_agent_panes_drops_empty_sessions() {
        let sessions = test_session(vec![test_pane("%1")]);
        let live = HashSet::new();

        let filtered = AppState::filter_sessions_to_live_agent_panes(sessions, &live);

        assert!(filtered.is_empty());
    }

    // ─── refresh_pane_tasks ──────────────────────────────────────

    fn cleanup_task_store(pane_id: &str) {
        let _ = std::fs::remove_file(crate::task_store::store_file_path(pane_id));
    }

    #[test]
    fn refresh_pane_tasks_reads_the_focused_pane_store() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%TASK_FRESH";
        state.focus_state.focused_pane_id = Some(pane_id.into());
        crate::task_store::record_task(pane_id, "first").unwrap();
        crate::task_store::record_task(pane_id, "second").unwrap();

        state.refresh_pane_tasks();

        assert_eq!(state.tasks.items.len(), 2);
        assert_eq!(state.tasks.items[0].subject, "first");
        assert_eq!(state.tasks.items[1].subject, "second");
        assert_eq!(
            state.tasks.store_cache.as_ref().map(|(id, _)| id.as_str()),
            Some(pane_id),
        );

        cleanup_task_store(pane_id);
    }

    #[test]
    fn refresh_pane_tasks_skips_reparse_when_mtime_unchanged() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%TASK_STALE";
        state.focus_state.focused_pane_id = Some(pane_id.into());
        crate::task_store::record_task(pane_id, "v1 item").unwrap();
        state.refresh_pane_tasks();
        assert_eq!(state.tasks.items.len(), 1);

        // Rewrite the store behind the sidebar's back but pin its mtime
        // to the cached value, so the only observable difference is the
        // content — a skipped re-parse must keep the cached items.
        let cached_mtime = state.tasks.store_cache.as_ref().unwrap().1;
        crate::task_store::record_task(pane_id, "v2 item").unwrap();
        let file = std::fs::File::options()
            .write(true)
            .open(crate::task_store::store_file_path(pane_id))
            .unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(cached_mtime))
            .unwrap();

        state.refresh_pane_tasks();

        assert_eq!(
            state.tasks.items.len(),
            1,
            "unchanged mtime must keep the cached items"
        );
        assert_eq!(state.tasks.items[0].subject, "v1 item");

        // A real mtime bump reloads the store.
        crate::task_store::record_task(pane_id, "v3 item").unwrap();
        state.refresh_pane_tasks();
        assert_eq!(state.tasks.items.len(), 3);

        cleanup_task_store(pane_id);
    }

    #[test]
    fn refresh_pane_tasks_clears_when_store_is_removed() {
        let mut state = AppState::new("%99".into());
        let pane_id = "%TASK_CLEARED";
        state.focus_state.focused_pane_id = Some(pane_id.into());
        crate::task_store::record_task(pane_id, "doomed").unwrap();
        state.refresh_pane_tasks();
        assert_eq!(state.tasks.items.len(), 1);

        // SessionStart clear (or /tmp cleanup): the file is gone, so the
        // mtime is None and the next tick must drop the cached list.
        crate::task_store::clear_tasks(pane_id);
        state.refresh_pane_tasks();

        assert!(state.tasks.items.is_empty());
        assert!(state.tasks.store_cache.is_none());
    }

    #[test]
    fn refresh_pane_tasks_clears_without_focus() {
        let mut state = AppState::new("%99".into());
        state.tasks.items = vec![crate::task_store::TaskItem {
            id: "1".into(),
            subject: "keep?".into(),
            status: crate::task_store::TaskItemStatus::Pending,
        }];

        state.focus_state.focused_pane_id = None;
        state.refresh_pane_tasks();

        assert!(state.tasks.items.is_empty());
        assert!(state.tasks.store_cache.is_none());
    }

    #[test]
    fn refresh_pane_tasks_follows_focus_to_another_pane() {
        let mut state = AppState::new("%99".into());
        let pane_a = "%TASK_FOCUS_A";
        let pane_b = "%TASK_FOCUS_B";
        crate::task_store::record_task(pane_a, "a's item").unwrap();
        crate::task_store::record_task(pane_b, "b's item").unwrap();

        state.focus_state.focused_pane_id = Some(pane_a.into());
        state.refresh_pane_tasks();
        assert_eq!(state.tasks.items[0].subject, "a's item");

        state.focus_state.focused_pane_id = Some(pane_b.into());
        state.refresh_pane_tasks();
        assert_eq!(state.tasks.items[0].subject, "b's item");

        cleanup_task_store(pane_a);
        cleanup_task_store(pane_b);
    }
}
