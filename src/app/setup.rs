use crate::cli::plugin_state;
use crate::state::AppState;
use crate::ui;

/// Construct and prime the initial [`AppState`] before the event loop starts.
///
/// Equivalent to the original `run_app` prelude in `src/main.rs`: installs the
/// color theme/icons from tmux options, loads global filter state, resolves
/// the Claude plugin install version once at startup, and performs
/// the first refresh pass.
pub(super) fn init_state(tmux_pane: String) -> AppState {
    let mut state = AppState::new(tmux_pane);
    state.theme = ui::colors::ColorTheme::from_tmux();
    state.icons = ui::icons::StatusIcons::from_tmux();
    state.pet_enabled = ui::pet_enabled_from_tmux();
    state.global.load_from_tmux();
    state.refresh();

    super::render::refresh_git_for_focused_pane(&mut state);

    // Resolve the installed Claude Code plugin status once at startup,
    // matching the version_notice pattern. Restart the sidebar after a
    // /plugin install, /plugin uninstall, or /plugin update to pick up
    // the new state.
    state.notices.claude_plugin_status = plugin_state::installed_plugin_status();
    // Notice inputs are static after the line above, so compute
    // them once here instead of from the per-tick refresh loop. This
    // also decouples the ⓘ badge from `focused_pane_id`, so killing
    // the last agent pane no longer drops outstanding setup warnings.
    state.refresh_notices();
    state.refresh();

    state
}
