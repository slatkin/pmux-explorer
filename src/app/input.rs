use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::state::{AppState, BottomTab, Focus, StatusFilter};
use crate::worktree::RemoveMode;

/// Dispatch a single crossterm [`Event`] into the [`AppState`], returning
/// `true` when a redraw should be scheduled.
///
/// The terminal handle is only borrowed to query its size for mouse
/// coordinate conversion; it is never written to from here.
pub(super) fn handle_event(
    ev: Event,
    state: &mut AppState,
    git_tab_active: &AtomicBool,
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
) -> bool {
    match ev {
        Event::Key(key) => handle_key_event(key, state, git_tab_active),
        Event::Mouse(mouse) => {
            let term_height = terminal.size().map(|s| s.height).unwrap_or(0);
            // The height the last frame actually drew — clamped while the
            // sidebar floats, `0` before the first frame (nothing has
            // been drawn, so nothing is clickable as a bottom panel).
            // Hit-testing the raw configured height would put the tab row
            // and the panel boundary where nothing was drawn.
            let bottom_h = state.layout.bottom_panel_height;
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    // A press on the rule starts a split drag and
                    // selects no tab; presses above the rule go to the
                    // agents panel. The background git poller syncs at
                    // the release, which is the event that may change
                    // the tab now.
                    state.handle_terminal_click(mouse.row, mouse.column, term_height);
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    // Held-button motion: while a split drag is live,
                    // resize the drawn split to the mouse row; without
                    // a drag this is a no-op.
                    state.handle_split_drag_move(mouse.row, term_height);
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    // The release resolves a split drag — a moved drag
                    // saves its rows as the shared percent, an unmoved
                    // press selects the tab the press column names.
                    state.end_split_drag(term_height);
                    // Keep the background git poller in sync immediately — the
                    // keyboard `BackTab` path does the same update. Without this,
                    // clicking into Git Status leaves polling disabled until the
                    // next refresh tick and the tab renders stale data.
                    git_tab_active.store(
                        state.displayed_bottom_tab().is_git_status(),
                        Ordering::Relaxed,
                    );
                }
                MouseEventKind::ScrollDown => {
                    state.handle_mouse_scroll(mouse.row, term_height, bottom_h, 3);
                }
                MouseEventKind::ScrollUp => {
                    state.handle_mouse_scroll(mouse.row, term_height, bottom_h, -3);
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

/// Dispatch a single [`KeyEvent`]. Split out from [`handle_event`] so that
/// unit tests can drive the keyboard path without constructing a real
/// terminal handle (the [`Terminal`] argument is only needed for mouse
/// coordinate conversion).
pub(super) fn handle_key_event(
    key: KeyEvent,
    state: &mut AppState,
    git_tab_active: &AtomicBool,
) -> bool {
    if handle_modal_key(key, state) {
        return true;
    }
    handle_main_key(key, state, git_tab_active);
    true
}

/// Swallow the whole keyboard while a modal layer (notices popup, spawn
/// modal, remove confirmation or repo popup) is open; a key the layer
/// does not bind does nothing, but still never reaches the main keys.
/// `false` when no modal layer is open.
fn handle_modal_key(key: KeyEvent, state: &mut AppState) -> bool {
    if state.is_notices_popup_open() {
        handle_notices_popup_key(key, state);
    } else if state.is_spawn_input_open() {
        handle_spawn_input_key(key, state);
    } else if state.is_remove_confirm_open() {
        handle_remove_confirm_key(key, state);
    } else if state.is_repo_popup_open() {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        handle_repo_popup_key(key, state, ctrl);
    } else {
        return false;
    }
    true
}

/// Keys while the notices popup is open: Esc closes it, every other key
/// is swallowed.
fn handle_notices_popup_key(key: KeyEvent, state: &mut AppState) {
    if key.code == KeyCode::Esc {
        state.close_notices_popup();
    }
}

/// Keys while the spawn modal is open: it owns the keyboard — Esc
/// cancels, Enter confirms, Tab/BackTab and the arrows walk the fields,
/// characters edit the focused field.
fn handle_spawn_input_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Esc => state.close_spawn_input(),
        KeyCode::Enter => state.confirm_spawn_input(),
        KeyCode::Tab | KeyCode::Down => state.spawn_input_next_field(),
        KeyCode::BackTab | KeyCode::Up => state.spawn_input_prev_field(),
        KeyCode::Left => state.spawn_input_cycle(-1),
        KeyCode::Right => state.spawn_input_cycle(1),
        KeyCode::Backspace => state.spawn_input_pop_char(),
        KeyCode::Char(c) => state.spawn_input_push_char(c),
        _ => {}
    }
}

/// Keys while the remove confirmation is open: Esc and `n` cancel, `c`
/// removes only the window, Enter and `y` remove the worktree too.
fn handle_remove_confirm_key(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => state.close_remove_confirm(),
        KeyCode::Char('c') => state.confirm_remove(RemoveMode::WindowOnly),
        KeyCode::Enter | KeyCode::Char('y') => state.confirm_remove(RemoveMode::WindowAndWorktree),
        _ => {}
    }
}

/// Keys while the repo popup is open: j/Ctrl-n and k/Ctrl-p move the
/// selection, Enter confirms, Esc cancels.
fn handle_repo_popup_key(key: KeyEvent, state: &mut AppState, ctrl: bool) {
    match key.code {
        KeyCode::Esc => state.close_repo_popup(),
        KeyCode::Char('j') | KeyCode::Down => repo_popup_nav_down(state),
        KeyCode::Char('n') if ctrl => repo_popup_nav_down(state),
        KeyCode::Char('k') | KeyCode::Up => repo_popup_nav_up(state),
        KeyCode::Char('p') if ctrl => repo_popup_nav_up(state),
        KeyCode::Enter => state.confirm_repo_popup(),
        _ => {}
    }
}

/// Keys with no modal layer open: bottom-panel navigation and focus,
/// the filter bar's keys while it has focus, the agent list's item
/// actions and the bottom tab cycle.
fn handle_main_key(key: KeyEvent, state: &mut AppState, git_tab_active: &AtomicBool) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    // The filter bar's keys only act while it has focus; elsewhere they
    // fall through to the wildcard like in the caller's match below.
    if handle_filter_keys(state, key.code) {
        return;
    }
    match key.code {
        KeyCode::Esc => esc_returns_focus_to_panes(state),
        KeyCode::Char('j') | KeyCode::Down => pane_nav_down(state),
        KeyCode::Char('n') if ctrl => pane_nav_down(state),
        KeyCode::Char('k') | KeyCode::Up => pane_nav_up(state),
        // pmux-explorer.conf forwards tmux prefix + Ctrl-P as M-C-p.
        KeyCode::Char('p') if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            state.toggle_popout();
        }
        KeyCode::Char('p') if ctrl => pane_nav_up(state),
        KeyCode::Char('n') => {
            if state.focus_state.focus == Focus::Panes {
                state.open_spawn_input_from_selection();
            }
        }
        KeyCode::Char('x') => {
            if state.focus_state.focus == Focus::Panes {
                state.open_remove_confirm();
            }
        }
        KeyCode::Enter => {
            if state.focus_state.focus == Focus::Panes {
                state.activate_selected_pane();
            }
        }
        KeyCode::BackTab => handle_back_tab(state, git_tab_active),
        _ => {}
    }
}

/// The filter bar's own keys while it has focus: Left/h and Right/l
/// step the status filter, Tab cycles it like Right does, `r` opens the
/// repo popup. Tab only cycles the status filter while the filter bar
/// has focus — cycling it from the agent list made Tab change a
/// server-wide setting by accident.
fn handle_filter_keys(state: &mut AppState, code: KeyCode) -> bool {
    if state.focus_state.focus != Focus::Filter {
        return false;
    }
    match code {
        KeyCode::Char('h') | KeyCode::Left => {
            apply_status_filter(state, state.global.status_filter.prev());
        }
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
            apply_status_filter(state, state.global.status_filter.next());
        }
        KeyCode::Char('r') => state.toggle_repo_popup(),
        _ => return false,
    }
    true
}

/// Adopt a new status filter: persist it and rebuild the agent rows to
/// match. Shared by the Left/Right keys and Tab.
fn apply_status_filter(state: &mut AppState, filter: StatusFilter) {
    state.global.status_filter = filter;
    state.global.save_filter();
    state.rebuild_row_targets();
}

/// Esc from the bottom panel (activity log or filter bar) returns focus
/// to the agent list.
fn esc_returns_focus_to_panes(state: &mut AppState) {
    if state.focus_state.focus == Focus::ActivityLog || state.focus_state.focus == Focus::Filter {
        state.focus_state.focus = Focus::Panes;
    }
}

/// BackTab advances the bottom panel's tab cycle and keeps the git
/// poller flag in step with the Git tab.
fn handle_back_tab(state: &mut AppState, git_tab_active: &AtomicBool) {
    state.next_bottom_tab();
    git_tab_active.store(
        state.displayed_bottom_tab().is_git_status(),
        Ordering::Relaxed,
    );
}

fn pane_nav_down(state: &mut AppState) {
    match state.focus_state.focus {
        Focus::Filter => {
            state.focus_state.focus = Focus::Panes;
        }
        Focus::Panes => {
            if state.move_pane_selection(1) {
                state.global.queue_cursor_save();
            } else {
                state.focus_state.focus = Focus::ActivityLog;
            }
        }
        Focus::ActivityLog => state.scroll_bottom(1),
    }
}

fn pane_nav_up(state: &mut AppState) {
    match state.focus_state.focus {
        Focus::Filter => {}
        Focus::Panes => {
            if state.move_pane_selection(-1) {
                state.global.queue_cursor_save();
            } else {
                state.focus_state.focus = Focus::Filter;
            }
        }
        Focus::ActivityLog => {
            let at_top = match state.displayed_bottom_tab() {
                BottomTab::Activity => state.activity.scroll.offset == 0,
                BottomTab::Tasks => state.tasks.scroll.offset == 0,
                BottomTab::GitStatus => state.scrolls.git.offset == 0,
                BottomTab::Thought => state.scrolls.thought.offset == 0,
            };
            if at_top {
                state.focus_state.focus = Focus::Panes;
            } else {
                state.scroll_bottom(-1);
            }
        }
    }
}

fn repo_popup_nav_down(state: &mut AppState) {
    let count = state.repo_names().len();
    let current = state.repo_popup_selected();
    if current + 1 < count {
        state.set_repo_popup_selected(current + 1);
    }
}

fn repo_popup_nav_up(state: &mut AppState) {
    let current = state.repo_popup_selected();
    if current > 0 {
        state.set_repo_popup_selected(current - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::RepoGroup;
    use crate::state::RowTarget;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn popout_key() -> KeyEvent {
        KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        )
    }

    /// Build an AppState with three navigable pane rows and Panes focus,
    /// which is the precondition the navigation arms operate against.
    fn state_with_three_panes() -> AppState {
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
        state.focus_state.focus = Focus::Panes;
        state
    }

    fn state_with_repo_popup_open() -> AppState {
        let mut state = AppState::new("%99".into());
        // toggle_repo_popup uses repo_names(), which always includes the
        // "All" sentinel — pad with two named groups so the selection has
        // somewhere to move.
        state.repo_groups = vec![
            RepoGroup {
                name: "repo-a".into(),
                has_focus: false,
                panes: vec![],
            },
            RepoGroup {
                name: "repo-b".into(),
                has_focus: false,
                panes: vec![],
            },
        ];
        state.toggle_repo_popup();
        state.set_repo_popup_selected(0);
        state
    }

    #[test]
    fn ctrl_n_moves_pane_selection_down() {
        let mut state = state_with_three_panes();
        let flag = AtomicBool::new(false);
        handle_key_event(ctrl_key('n'), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 1);
        handle_key_event(ctrl_key('n'), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 2);
    }

    #[test]
    fn ctrl_p_moves_pane_selection_up() {
        let mut state = state_with_three_panes();
        state.global.selected_pane_row = 2;
        let flag = AtomicBool::new(false);
        handle_key_event(ctrl_key('p'), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 1);
        handle_key_event(ctrl_key('p'), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn bare_j_and_k_still_navigate_panes() {
        let mut state = state_with_three_panes();
        let flag = AtomicBool::new(false);
        handle_key_event(key(KeyCode::Char('j')), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 1);
        handle_key_event(key(KeyCode::Char('k')), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn bare_n_does_not_move_selection() {
        // The bare `n` arm is wired to the spawn input flow, not navigation.
        // We don't assert the popup opens (that requires repo_groups +
        // git metadata, exercised elsewhere) — only that it does NOT
        // shadow the Ctrl-N navigation arm.
        let mut state = state_with_three_panes();
        let flag = AtomicBool::new(false);
        handle_key_event(key(KeyCode::Char('n')), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 0);
    }

    #[test]
    fn bare_p_does_not_pop_out_or_navigate() {
        let mut state = state_with_three_panes();
        state.tmux_pane = "%4294967295".into();
        state.global.selected_pane_row = 1;
        let flag = AtomicBool::new(false);
        handle_key_event(key(KeyCode::Char('p')), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 1);
        assert!(state.flash.is_none());
    }

    #[test]
    fn popout_shortcut_does_not_toggle_while_a_popup_is_open() {
        let mut state = state_with_repo_popup_open();
        state.tmux_pane = "%4294967295".into();
        let flag = AtomicBool::new(false);
        handle_key_event(popout_key(), &mut state, &flag);
        assert!(state.flash.is_none(), "no pop-out while a popup is open");
        assert!(state.is_repo_popup_open());
    }

    #[test]
    fn popout_shortcut_toggles_without_navigating() {
        // A nonexistent pane makes the action report a failed probe,
        // so reaching it is observable without floating a real pane.
        let mut state = state_with_three_panes();
        state.tmux_pane = "%4294967295".into();
        state.global.selected_pane_row = 1;
        let flag = AtomicBool::new(false);
        handle_key_event(popout_key(), &mut state, &flag);
        assert_eq!(state.global.selected_pane_row, 1);
        assert_eq!(
            state.flash.as_ref().map(|(msg, _)| msg.as_str()),
            Some(
                "Pop-out unavailable: could not read the window size, so the sidebar stays docked."
            ),
        );
    }

    #[test]
    fn ctrl_n_navigates_repo_popup_down() {
        let mut state = state_with_repo_popup_open();
        let flag = AtomicBool::new(false);
        handle_key_event(ctrl_key('n'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 1);
        handle_key_event(ctrl_key('n'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 2);
        // Past the last entry the popup nav helper is a no-op.
        handle_key_event(ctrl_key('n'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 2);
    }

    #[test]
    fn ctrl_p_navigates_repo_popup_up() {
        let mut state = state_with_repo_popup_open();
        state.set_repo_popup_selected(2);
        let flag = AtomicBool::new(false);
        handle_key_event(ctrl_key('p'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 1);
        handle_key_event(ctrl_key('p'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 0);
        // Below 0 the popup nav helper is a no-op.
        handle_key_event(ctrl_key('p'), &mut state, &flag);
        assert_eq!(state.repo_popup_selected(), 0);
    }

    #[test]
    fn backtab_cycles_the_three_bottom_tabs() {
        // BackTab walks Activity, Tasks, Git and stays on Git's flag only
        // while Git is showing — the background git poller keys on it.
        let mut state = state_with_three_panes();
        let flag = AtomicBool::new(false);

        handle_key_event(key(KeyCode::BackTab), &mut state, &flag);
        assert_eq!(state.global.bottom_tab, BottomTab::Tasks);
        assert!(!flag.load(Ordering::Relaxed));

        handle_key_event(key(KeyCode::BackTab), &mut state, &flag);
        assert_eq!(state.global.bottom_tab, BottomTab::GitStatus);
        assert!(flag.load(Ordering::Relaxed));

        handle_key_event(key(KeyCode::BackTab), &mut state, &flag);
        assert_eq!(state.global.bottom_tab, BottomTab::Activity);
        assert!(!flag.load(Ordering::Relaxed));
    }

    #[test]
    fn k_from_the_tasks_tab_scrolls_until_it_reaches_the_top() {
        // The Tasks tab joins the bottom-panel focus chain: k walks its
        // scroll like Activity/Git and hands focus back only at the top.
        let mut state = state_with_three_panes();
        state.focus_state.focus = Focus::ActivityLog;
        state.global.bottom_tab = BottomTab::Tasks;
        state.tasks.scroll = crate::state::ScrollState {
            offset: 2,
            total_lines: 10,
            visible_height: 3,
        };
        let flag = AtomicBool::new(false);

        handle_key_event(key(KeyCode::Up), &mut state, &flag);
        assert_eq!(state.focus_state.focus, Focus::ActivityLog);
        assert_eq!(state.tasks.scroll.offset, 1);

        handle_key_event(key(KeyCode::Up), &mut state, &flag);
        assert_eq!(state.tasks.scroll.offset, 0);
        assert_eq!(state.focus_state.focus, Focus::ActivityLog);

        // One more press at the top hands focus back to the pane list.
        handle_key_event(key(KeyCode::Up), &mut state, &flag);
        assert_eq!(state.tasks.scroll.offset, 0);
        assert_eq!(
            state.focus_state.focus,
            Focus::Panes,
            "k at the top of the Tasks list must leave the bottom panel"
        );
    }

    #[test]
    fn tab_from_the_agent_list_does_not_change_the_status_filter() {
        // Tab used to cycle the status filter with no focus check, so a
        // Tab pressed while working in the agent list silently changed a
        // setting every sidebar on the tmux server shares.
        let mut state = state_with_three_panes();
        let flag = AtomicBool::new(false);
        let before = state.global.status_filter;

        handle_key_event(key(KeyCode::Tab), &mut state, &flag);

        assert_eq!(
            state.global.status_filter, before,
            "Tab from the agent list must not change the status filter"
        );
    }

    #[test]
    fn tab_still_cycles_the_status_filter_while_it_has_focus() {
        let mut state = state_with_three_panes();
        state.focus_state.focus = Focus::Filter;
        let flag = AtomicBool::new(false);
        let before = state.global.status_filter;

        handle_key_event(key(KeyCode::Tab), &mut state, &flag);

        assert_eq!(state.global.status_filter, before.next());
    }
}
