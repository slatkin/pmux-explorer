use crate::cli::plugin_state;
use crate::state::{AppState, LayoutKind};
use crate::tmux;
use crate::ui;

/// Construct and prime the initial [`AppState`] before the event loop starts.
///
/// Equivalent to the original `run_app` prelude in `src/main.rs`: installs the
/// color theme/icons from tmux options, loads global filter state, resolves
/// the Claude plugin install version once at startup, and performs
/// the first refresh pass.
pub(super) fn init_state(tmux_pane: String) -> AppState {
    let mut state = AppState::new(tmux_pane);
    // The layout is read once, from the process environment `toggle`
    // handed the pane (`-e PMUX_SIDEBAR_LAYOUT=strip|column`) — never
    // from a pane option, which the TUI could read before `toggle`
    // writes it (decision 1 of the `add-strip-layout` design). A
    // missing or unknown value keeps the column behavior.
    state.layout_kind =
        LayoutKind::from_env_value(std::env::var("PMUX_SIDEBAR_LAYOUT").ok().as_deref());
    state.theme = ui::colors::ColorTheme::from_tmux();
    // Loud border while the sidebar pane is the active tmux pane: a
    // per-pane `pane-active-border-style` write, once at startup. tmux
    // applies it on every border redraw, so focus transitions need no
    // sidebar involvement. Cosmetic best-effort (see commands.rs).
    tmux::set_pane_active_border_style(
        &state.tmux_pane,
        &ui::colors::tmux_fg_style(state.theme.focus_border),
    );
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
