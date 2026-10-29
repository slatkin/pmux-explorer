use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cli::toggle::{
    SidebarPosition, target_pane_for_position, target_pane_for_strip_position,
};
use crate::state::LayoutKind;
use crate::tmux::PANE_ROLE;
use crate::tmux::PANE_SIDEBAR_LAYOUT;

static ISOLATED: AtomicBool = AtomicBool::new(false);

/// Point every later `tmux` call at a named socket no server listens on, so a
/// test cannot read or write the developer's live tmux server. Unit tests get
/// this automatically; integration tests (which link the library without
/// `cfg(test)`) must call it. Without it, a test that clicked a status
/// chip overwrote the shared `@sidebar_filter` of every running sidebar.
///
/// The isolation uses `tmux -L <name>` rather than a bogus `TMUX_TMPDIR`:
/// when the `TMUX_TMPDIR` path cannot hold a socket, tmux silently falls
/// back to the default `/tmp/tmux-$UID` socket where the live server sits,
/// so the old trick isolated nothing.
pub fn isolate_from_live_server() {
    ISOLATED.store(true, Ordering::Relaxed);
}

const ISOLATION_SOCKET: &str = "sidebar-test-isolated";

pub fn tmux_command() -> Command {
    let mut cmd = Command::new("tmux");
    if cfg!(test) || ISOLATED.load(Ordering::Relaxed) {
        cmd.env_remove("TMUX").args(["-L", ISOLATION_SOCKET]);
    }
    cmd
}

pub fn run_tmux(args: &[&str]) -> Option<String> {
    let output = tmux_command().args(args).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Run a tmux command, returning trimmed stdout on success and stderr on failure.
/// Used by the spawn/remove flow so the UI can surface a meaningful error message
/// instead of a silent fallthrough.
pub fn run_tmux_capture(args: &[&str]) -> Result<String, String> {
    let output = tmux_command()
        .args(args)
        .output()
        .map_err(|e| format!("failed to spawn tmux: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("tmux exited with status {}", output.status)
        } else {
            stderr
        })
    }
}

/// Pure builder for the `break-pane -W` argument list that floats `pane`
/// with the given size and upper-left position (tmux 3.8). Kept separate
/// from the tmux call, the way `cli/toggle.rs` builds its split flags, so
/// a unit test can assert the exact arguments.
fn break_pane_float_args(pane: &str, width: u16, height: u16, x: u16, y: u16) -> Vec<String> {
    vec![
        "break-pane".into(),
        "-W".into(),
        "-s".into(),
        pane.into(),
        "-x".into(),
        width.to_string(),
        "-y".into(),
        height.to_string(),
        "-X".into(),
        x.to_string(),
        "-Y".into(),
        y.to_string(),
    ]
}

/// Pure builder for the `join-pane` argument list that returns a floating
/// `pane` to its previous position in the tiled layout. With the destination
/// unspecified, tmux restores the layout position the pane had before it
/// was floated (tmux 3.8).
fn join_pane_dock_args(pane: &str) -> Vec<String> {
    vec!["join-pane".into(), "-s".into(), pane.into()]
}

/// Pure builder for the `move-pane` argument list that moves floating
/// `pane` so its upper-left corner sits at (`x`, `y`) in its window
/// (tmux 3.8; the reported `pane_left` is 1-based, so `-X 40` puts the
/// pane's left edge at x=40). Kept separate from the tmux call, like the
/// other builders here, so a unit test can assert the exact arguments.
fn move_pane_args(pane: &str, x: u16, y: u16) -> Vec<String> {
    vec![
        "move-pane".into(),
        "-s".into(),
        pane.into(),
        "-X".into(),
        x.to_string(),
        "-Y".into(),
        y.to_string(),
    ]
}

/// Float `pane`: lift it out of the tiled layout and make it a floating
/// pane of its window with the given size and upper-left position. Needs
/// tmux 3.8. Reports tmux's error, e.g. a zoomed window refuses with
/// "can't float a pane while window is zoomed".
pub fn float_pane(pane: &str, width: u16, height: u16, x: u16, y: u16) -> Result<(), String> {
    let owned = break_pane_float_args(pane, width, height, x, y);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// Return floating `pane` to its previous position in the tiled layout of
/// its window.
pub fn dock_pane(pane: &str) -> Result<(), String> {
    let owned = join_pane_dock_args(pane);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// Move floating `pane` so its upper-left corner sits at (`x`, `y`) in
/// its window. The reposition used after a window resize, never a
/// re-float — `break-pane -W` fails on an already floating pane.
pub fn move_pane(pane: &str, x: u16, y: u16) -> Result<(), String> {
    let owned = move_pane_args(pane, x, y);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// Pure builder for the `join-pane` argument list that moves `pane`
/// to the left or right edge of its window. `target` is the outermost
/// workspace pane on the new side (see `target_pane_for_position`);
/// the joined pane spans the full window height (`-h -f`, tmux 3.8)
/// and lands before the target for a left move (`-b`), after it for a
/// right move. `-d` keeps the window's active pane — without it the
/// just-moved sidebar steals focus in every window the sweep touches.
/// Kept separate from the tmux call, like the other builders here, so
/// a unit test can assert the exact arguments.
fn join_pane_side_move_args(pane: &str, target: &str, position: SidebarPosition) -> Vec<String> {
    let mut args = vec![
        "join-pane".to_string(),
        "-h".to_string(),
        "-f".to_string(),
        "-d".to_string(),
    ];
    if position == SidebarPosition::Left {
        args.push("-b".to_string());
    }
    args.extend([
        "-s".to_string(),
        pane.to_string(),
        "-t".to_string(),
        target.to_string(),
    ]);
    args
}

/// The vertical twin of [`join_pane_side_move_args`]: the `join-pane`
/// argument list that moves a strip `pane` to the top or bottom edge of
/// its window. `target` is the outermost workspace pane on the new side
/// (see `target_pane_for_strip_position`); the joined pane spans the
/// full window width (`-v -f`, tmux 3.8) and lands before the target
/// for a top move (`-b`), after it for a bottom move. `-d` keeps the
/// window's active pane, for the same reason the horizontal builder
/// passes it.
fn join_pane_strip_move_args(pane: &str, target: &str, position: SidebarPosition) -> Vec<String> {
    let mut args = vec![
        "join-pane".to_string(),
        "-v".to_string(),
        "-f".to_string(),
        "-d".to_string(),
    ];
    if position == SidebarPosition::Top {
        args.push("-b".to_string());
    }
    args.extend([
        "-s".to_string(),
        pane.to_string(),
        "-t".to_string(),
        target.to_string(),
    ]);
    args
}

/// Pure builder for the pop-in of a floating strip: the edge join of
/// the side switch ([`join_pane_strip_move_args`]) when `edge_target`
/// names the workspace pane to dock against, else the bare join the
/// column pop-in uses ([`join_pane_dock_args`]) — a fallback that does
/// not restore the full width, which is why a strip prefers the edge
/// join.
pub(crate) fn strip_dock_args(
    pane: &str,
    edge_target: Option<&str>,
    edge: SidebarPosition,
) -> Vec<String> {
    match edge_target {
        Some(target) => join_pane_strip_move_args(pane, target, edge),
        None => join_pane_dock_args(pane),
    }
}

/// Dock floating strip `pane` at `edge` against `edge_target` (see
/// [`strip_dock_args`]); the caller restores the docked height with
/// `resize-pane -y` after the join succeeds.
pub fn dock_strip_pane(pane: &str, edge_target: &str, edge: SidebarPosition) -> Result<(), String> {
    let owned = strip_dock_args(pane, Some(edge_target), edge);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// The workspace lines of `output` — a `#{pane_floating_flag}
/// #{pane_top} #{pane_height} #{pane_id}` listing, or the same without
/// the leading flag when tmux could not expand it — whose pane is
/// neither `sidebar` itself nor floating, in encounter order: the
/// input [`target_pane_for_strip_position`] picks a strip's edge
/// target from. A floating pane is never a join target — `join-pane`
/// onto one is refused — so another pane's scratch float must not be
/// picked even when it sits outermost.
fn strip_workspace_listing(output: &str, sidebar: &str) -> String {
    output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // `flag top height pane_id`; a flag tmux could not expand
            // (older than 3.8) collapses its slot, leaving three fields.
            let (floating, workspace) = match fields.as_slice() {
                [flag, top, height, pane] => (*flag == "1", [*top, *height, *pane]),
                [top, height, pane] => (false, [*top, *height, *pane]),
                _ => return None,
            };
            let keep = !floating && workspace[2] != sidebar;
            keep.then(|| workspace.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The workspace pane to dock a floating strip against at `edge`, read
/// from the sidebar pane's own window (tmux resolves a pane target of
/// `list-panes` to its window): the panes' vertical geometry, the
/// floating sidebar itself and any other floating pane excluded.
/// `None` when tmux cannot be reached or no workspace pane parses —
/// the pop-in then falls back to the bare join.
pub(crate) fn strip_dock_target(sidebar: &str, edge: SidebarPosition) -> Option<String> {
    let output = run_tmux(&[
        "list-panes",
        "-t",
        sidebar,
        "-F",
        "#{pane_floating_flag} #{pane_top} #{pane_height} #{pane_id}",
    ])?;
    strip_dock_target_from(&output, sidebar, edge)
}

/// [`strip_dock_target`]'s choice over an already-fetched listing —
/// split out so the target choice is unit-testable without a live tmux.
fn strip_dock_target_from(output: &str, sidebar: &str, edge: SidebarPosition) -> Option<String> {
    target_pane_for_strip_position(&strip_workspace_listing(output, sidebar), edge)
}

/// Move `pane` (a window's sidebar) to the edge `position` names: a
/// column joins left or right (the horizontal builder), a strip joins
/// top or bottom (the vertical twin). The pane's process is untouched
/// — tmux only re-parents the pane, so the running TUI keeps its state
/// and just resizes.
fn move_pane_to_side(pane: &str, target: &str, position: SidebarPosition) -> Result<(), String> {
    let owned = match position.axis() {
        LayoutKind::Strip => join_pane_strip_move_args(pane, target, position),
        LayoutKind::Column => join_pane_side_move_args(pane, target, position),
    };
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// One pane's geometry within its window, as the sweep needs it: the
/// horizontal pair picks a column's edge target, the vertical pair a
/// strip's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SweepPaneGeometry {
    pub left: u32,
    pub width: u32,
    pub top: u32,
    pub height: u32,
    pub pane_id: String,
}

/// One window's sidebar pane as the sweep reads it off the pane: its
/// id, floating flag, fixed layout kind (`@pane_sidebar_layout` — a
/// missing or unknown value reads as a column, so a sidebar made before
/// the strip layout existed keeps its behavior) and `#{pane_at_top}`
/// flag, which says which edge a docked strip sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SweepSidebar {
    pub pane_id: String,
    pub floating: bool,
    pub layout: LayoutKind,
    pub left: u32,
    pub at_top: bool,
}

impl SweepSidebar {
    /// The edge this docked sidebar sits on, read by its own layout
    /// kind: `#{pane_at_top}` for a strip, `#{pane_left}` for a column.
    /// Both fields arrive parsed — a line whose geometry does not parse
    /// never becomes a record — so the edge is total.
    fn edge(&self) -> SidebarPosition {
        match self.layout {
            LayoutKind::Strip => {
                if self.at_top {
                    SidebarPosition::Top
                } else {
                    SidebarPosition::Bottom
                }
            }
            LayoutKind::Column => {
                if self.left == 0 {
                    SidebarPosition::Left
                } else {
                    SidebarPosition::Right
                }
            }
        }
    }
}

/// One window of the side-switch sweep, parsed from a single
/// `list-panes -a` call: the window's id, its sidebar pane (with the
/// floating flag, its fixed layout kind and its `pane_at_top` flag) if
/// it has one, whether the window is zoomed, and every pane's geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowSweepRecord {
    pub window_id: String,
    /// The window's sidebar pane; `None` when the window has no sidebar.
    pub sidebar: Option<SweepSidebar>,
    /// Whether the window is zoomed (`#{window_zoomed_flag}`). tmux
    /// 3.8's `join-pane` does not refuse a zoomed window — it unzooms
    /// it and succeeds — so the sweep must skip it before the call.
    pub zoomed: bool,
    /// Geometry of every pane in the window, sidebar included.
    pub panes: Vec<SweepPaneGeometry>,
}

/// Parse the sweep's `list-panes -a` output into per-window records,
/// in encounter order. Lines that do not split into the eleven pipe
/// fields, or whose geometry numbers do not parse, are skipped; a
/// floating flag that tmux cannot report (older than 3.8) parses as
/// not floating, a missing `@pane_sidebar_layout` as a column, and a
/// missing `#{pane_at_top}` as not at the top. The role field selects
/// which pane is the window's sidebar; a window with two sidebar panes
/// keeps the first.
pub(crate) fn parse_window_sweep(output: &str) -> Vec<WindowSweepRecord> {
    fn push_fields(
        record: &mut WindowSweepRecord,
        sidebar: Option<SweepSidebar>,
        pane: SweepPaneGeometry,
    ) {
        if sidebar.is_some() && record.sidebar.is_none() {
            record.sidebar = sidebar;
        }
        record.panes.push(pane);
    }
    let mut records: Vec<WindowSweepRecord> = Vec::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split('|').collect();
        let [
            window_id,
            pane_id,
            role,
            floating,
            zoomed,
            left,
            width,
            top,
            height,
            layout,
            at_top,
        ] = fields[..]
        else {
            continue;
        };
        let (Ok(left), Ok(width), Ok(top), Ok(height)) = (
            left.parse::<u32>(),
            width.parse::<u32>(),
            top.parse::<u32>(),
            height.parse::<u32>(),
        ) else {
            continue;
        };
        let pane = SweepPaneGeometry {
            left,
            width,
            top,
            height,
            pane_id: pane_id.to_string(),
        };
        let sidebar = (role == "sidebar").then(|| SweepSidebar {
            pane_id: pane_id.to_string(),
            floating: floating == "1",
            layout: match layout.trim() {
                "strip" => LayoutKind::Strip,
                _ => LayoutKind::Column,
            },
            left,
            at_top: at_top == "1",
        });
        match records.last_mut() {
            Some(record) if record.window_id == window_id => {
                push_fields(record, sidebar, pane);
                if zoomed == "1" {
                    record.zoomed = true;
                }
            }
            _ => {
                let mut record = WindowSweepRecord {
                    window_id: window_id.to_string(),
                    sidebar: None,
                    zoomed: zoomed == "1",
                    panes: Vec::new(),
                };
                push_fields(&mut record, sidebar, pane);
                records.push(record);
            }
        }
    }
    records
}

/// Choose the move for one window of the sweep: the sidebar pane and
/// the outermost workspace pane on the new side, read along the
/// sidebar's own layout kind — horizontally for a column,
/// vertically for a strip. `position` is already resolved for this
/// sidebar (the sweep resolves it before the call). `None` when the
/// window is zoomed (tmux 3.8's `join-pane` would unzoom it and move
/// the sidebar — the zoom must win, so skip before the call), its
/// sidebar floats, the window has no sidebar, or no workspace pane to
/// join onto (the sidebar is alone).
pub(crate) fn side_switch_target(
    record: &WindowSweepRecord,
    layout: LayoutKind,
    position: SidebarPosition,
) -> Option<(String, String)> {
    if record.zoomed {
        return None;
    }
    let sidebar = record.sidebar.as_ref()?;
    if sidebar.floating {
        return None;
    }
    let workspace = record
        .panes
        .iter()
        .filter(|pane| pane.pane_id != sidebar.pane_id)
        .map(|pane| match layout {
            LayoutKind::Column => format!("{} {} {}", pane.left, pane.width, pane.pane_id),
            LayoutKind::Strip => format!("{} {} {}", pane.top, pane.height, pane.pane_id),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let target = match layout {
        LayoutKind::Column => target_pane_for_position(&workspace, position),
        LayoutKind::Strip => target_pane_for_strip_position(&workspace, position),
    }?;
    Some((sidebar.pane_id.clone(), target))
}

/// The sweep core over already-parsed records: move every window's
/// sidebar to its target edge and count moved and skipped windows. A
/// sidebar on the acting axis moves to `acting_target` — the opposite
/// of the edge the acting sidebar sits on — and a sidebar on the other
/// axis flips to its own opposite edge, because the action names only
/// one edge of one axis. A window with no sidebar is not a participant
/// — it is neither moved nor skipped, so the count names only the
/// sidebars the sweep could not move: no usable target (see
/// [`side_switch_target`]) or tmux refused the move (a zoomed window,
/// for one). The injected `mover` receives each sidebar's own target
/// edge, because the two axes need different `join-pane` flags, and
/// keeps the counting unit-testable without a live tmux.
fn sweep_side_switch<F>(
    records: &[WindowSweepRecord],
    acting_layout: LayoutKind,
    acting_target: SidebarPosition,
    mut mover: F,
) -> (usize, usize)
where
    F: FnMut(&str, &str, SidebarPosition) -> Result<(), String>,
{
    let mut moved = 0;
    let mut skipped = 0;
    for record in records {
        let Some(sidebar) = record.sidebar.as_ref() else {
            continue;
        };
        let position = if sidebar.layout == acting_layout {
            acting_target
        } else {
            sidebar.edge().opposite()
        };
        match side_switch_target(record, sidebar.layout, position) {
            None => skipped += 1,
            Some((sidebar_pane, target)) => match mover(&sidebar_pane, &target, position) {
                Ok(()) => moved += 1,
                Err(_) => skipped += 1,
            },
        }
    }
    (moved, skipped)
}

/// Move the sidebar of every window to its target edge in one sweep,
/// from a single `list-panes -a` listing: a sidebar on `acting_layout`
/// — the axis the acting sidebar runs — moves to `acting_target`, one
/// on the other axis flips to its own opposite edge. Returns the
/// counts of moved and skipped windows over the sidebar-bearing
/// windows only: a floating sidebar skips without a tmux call, and any
/// refused `join-pane` (e.g. a zoomed window) skips too. A window with
/// no sidebar does not count toward either number.
pub(crate) fn switch_sidebar_sides(
    acting_layout: LayoutKind,
    acting_target: SidebarPosition,
) -> (usize, usize) {
    let format = format!(
        "#{{window_id}}|#{{pane_id}}|#{{{}}}|#{{pane_floating_flag}}|#{{window_zoomed_flag}}|#{{pane_left}}|#{{pane_width}}|#{{pane_top}}|#{{pane_height}}|#{{{}}}|#{{pane_at_top}}",
        PANE_ROLE, PANE_SIDEBAR_LAYOUT
    );
    let Some(output) = run_tmux(&["list-panes", "-a", "-F", &format]) else {
        return (0, 0);
    };
    let records = parse_window_sweep(&output);
    sweep_side_switch(&records, acting_layout, acting_target, move_pane_to_side)
}

pub fn display_message(target: &str, format: &str) -> String {
    run_tmux(&["display-message", "-t", target, "-p", format])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Resolve the session name containing `pane_id`. Returns `None` when tmux
/// can't find the pane (e.g. it has just been closed).
pub fn pane_session_name(pane_id: &str) -> Option<String> {
    Some(display_message(pane_id, "#{session_name}")).filter(|s| !s.is_empty())
}

/// Create a new tmux window in `session` whose initial cwd is `cwd` and whose
/// title is `name`. Returns `(pane_id, window_id)` on success — the window id
/// is used by the spawn flow to set markers at window scope so split panes
/// (e.g. Claude Code subagents) inherit them.
pub fn new_window(session: &str, cwd: &str, name: &str) -> Result<(String, String), String> {
    let out = run_tmux_capture(&[
        "new-window",
        "-t",
        session,
        "-c",
        cwd,
        "-n",
        name,
        "-P",
        "-F",
        "#{pane_id} #{window_id}",
    ])?;
    let mut parts = out.split_whitespace();
    let pane = parts
        .next()
        .ok_or_else(|| "new-window returned no pane id".to_string())?
        .to_string();
    let window = parts
        .next()
        .ok_or_else(|| "new-window returned no window id".to_string())?
        .to_string();
    Ok((pane, window))
}

/// Set a user option at window scope. Needed so markers survive through
/// split panes that inherit from the window. Returns an error so the
/// spawn flow can roll back when a marker the remove path relies on
/// cannot be written — silently dropping the failure would leave an
/// un-removable pane.
pub fn set_window_option(window: &str, key: &str, value: &str) -> Result<(), String> {
    run_tmux_capture(&["set", "-w", "-t", window, key, value]).map(|_| ())
}

/// Send a command line to `target` (a pane id) and press Enter so the shell
/// executes it. Used to launch the agent binary right after window creation.
/// The text is sent with `-l` (literal) so nothing in `command` can collide
/// with tmux key names (e.g. `Tab`, `BSpace`); Enter is issued as a
/// separate invocation so it's interpreted as the Return key.
pub fn send_command(target: &str, command: &str) -> Result<(), String> {
    run_tmux_capture(&["send-keys", "-t", target, "-l", command])?;
    run_tmux_capture(&["send-keys", "-t", target, "Enter"]).map(|_| ())
}

/// Kill the tmux window identified by `window_id` (e.g. `@7`).
pub fn kill_window(window_id: &str) -> Result<(), String> {
    run_tmux_capture(&["kill-window", "-t", window_id]).map(|_| ())
}

pub fn select_pane(pane_id: &str) {
    // Find the session containing this pane and switch to it first
    let session_id = display_message(pane_id, "#{session_id}");
    if !session_id.is_empty() {
        let _ = run_tmux(&["switch-client", "-t", &session_id]);
    }
    // Then switch to the correct window
    let window_id = display_message(pane_id, "#{window_id}");
    if !window_id.is_empty() {
        let _ = run_tmux(&["select-window", "-t", &window_id]);
    }
    let _ = run_tmux(&["select-pane", "-t", pane_id]);
}

/// Build the `set-option -p` argument list that colours `pane`'s border
/// while it is the active pane of its window (`pane-active-border-style`).
/// Kept separate from the executor so tests can pin the tmux invocation
/// shape.
pub fn pane_active_border_args(pane_id: &str, style: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane_id.into(),
        "pane-active-border-style".into(),
        style.into(),
    ]
}

/// Colour `pane`'s border with `style` while it is the active pane, via
/// a per-pane `pane-active-border-style` option. tmux consults the
/// option on every border redraw, so focus transitions are reflected
/// without any per-tick sidebar work. Cosmetic best-effort: a failed
/// write leaves tmux's normal border styling in place.
pub fn set_pane_active_border_style(pane_id: &str, style: &str) {
    let args = pane_active_border_args(pane_id, style);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let _ = run_tmux(&arg_refs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_pane_float_args_match_tmux_3_8_flags() {
        assert_eq!(
            break_pane_float_args("%5", 40, 24, 75, 3),
            vec![
                "break-pane",
                "-W",
                "-s",
                "%5",
                "-x",
                "40",
                "-y",
                "24",
                "-X",
                "75",
                "-Y",
                "3",
            ]
        );
    }

    #[test]
    fn join_pane_dock_args_match_tmux_3_8_flags() {
        assert_eq!(join_pane_dock_args("%5"), vec!["join-pane", "-s", "%5"]);
    }

    #[test]
    fn move_pane_args_match_tmux_3_8_flags() {
        assert_eq!(
            move_pane_args("%5", 80, 0),
            vec!["move-pane", "-s", "%5", "-X", "80", "-Y", "0"]
        );
    }

    #[test]
    fn pane_active_border_args_target_the_pane_scope() {
        assert_eq!(
            pane_active_border_args("%7", "fg=#e69875"),
            vec![
                "set-option",
                "-p",
                "-t",
                "%7",
                "pane-active-border-style",
                "fg=#e69875"
            ]
        );
    }

    // ─── join_pane_side_move_args ────────────────────────────────────

    #[test]
    fn join_pane_side_move_args_match_tmux_3_8_flags_for_both_sides() {
        // A move to the left joins before the edge pane (-b) and one to
        // the right joins after it; both span the full window height.
        assert_eq!(
            join_pane_side_move_args("%5", "%2", SidebarPosition::Left),
            vec!["join-pane", "-h", "-f", "-d", "-b", "-s", "%5", "-t", "%2"]
        );
        assert_eq!(
            join_pane_side_move_args("%5", "%2", SidebarPosition::Right),
            vec!["join-pane", "-h", "-f", "-d", "-s", "%5", "-t", "%2"]
        );
    }

    #[test]
    fn join_pane_strip_move_args_match_tmux_3_8_flags_for_both_edges() {
        // A move to the top joins before the edge pane (-b) and one to
        // the bottom joins after it; both span the full window width.
        assert_eq!(
            join_pane_strip_move_args("%5", "%2", SidebarPosition::Top),
            vec!["join-pane", "-v", "-f", "-d", "-b", "-s", "%5", "-t", "%2"]
        );
        assert_eq!(
            join_pane_strip_move_args("%5", "%2", SidebarPosition::Bottom),
            vec!["join-pane", "-v", "-f", "-d", "-s", "%5", "-t", "%2"]
        );
    }

    #[test]
    fn strip_dock_args_take_the_edge_join_when_a_target_exists() {
        // The strip pop-in reuses the side switch's edge join verbatim.
        assert_eq!(
            strip_dock_args("%5", Some("%2"), SidebarPosition::Bottom),
            join_pane_strip_move_args("%5", "%2", SidebarPosition::Bottom)
        );
        assert_eq!(
            strip_dock_args("%5", Some("%2"), SidebarPosition::Top),
            join_pane_strip_move_args("%5", "%2", SidebarPosition::Top)
        );
    }

    #[test]
    fn strip_dock_args_fall_back_to_the_bare_join_without_a_target() {
        assert_eq!(
            strip_dock_args("%5", None, SidebarPosition::Bottom),
            join_pane_dock_args("%5")
        );
        assert_eq!(
            strip_dock_args("%5", None, SidebarPosition::Top),
            vec!["join-pane", "-s", "%5"]
        );
    }

    #[test]
    fn strip_workspace_listing_drops_the_sidebar_and_floating_panes() {
        // The floating sidebar is a pane of its own window, so the
        // listing must lose its line before the edge target is picked —
        // otherwise the strip could dock against itself. Any other
        // floating pane is dropped too: joining onto a floating pane
        // is refused.
        let listing = "0 0 24 %1\n1 29 5 %3\n0 24 6 %2\n";
        assert_eq!(strip_workspace_listing(listing, "%2"), "0 24 %1");
        // Only the sidebar: an empty workspace, the no-target case the
        // bare-join fallback covers.
        assert_eq!(strip_workspace_listing(listing, "%1"), "24 6 %2");
        assert_eq!(strip_workspace_listing("0 0 30 %7", "%7"), "");
        // A flag tmux could not expand (older than 3.8) collapses its
        // slot: the three-field shape keeps parsing, unflagged.
        assert_eq!(
            strip_workspace_listing("0 24 %1\n24 6 %2\n", "%2"),
            "0 24 %1"
        );
    }

    #[test]
    fn strip_dock_target_skips_a_floating_pane_sitting_outermost() {
        // The scratch float %3 sits outermost at the bottom edge
        // (bottom 34 beats %1's 24) — but joining onto a floating pane
        // is refused, so the edge target must skip past it to %1.
        let listing = "0 0 24 %1\n1 29 5 %3\n0 24 6 %2\n";
        assert_eq!(
            strip_dock_target_from(listing, "%2", SidebarPosition::Bottom),
            Some("%1".to_string())
        );
        // All workspace panes floating: no target at all — the bare
        // join fallback.
        assert_eq!(
            strip_dock_target_from("1 0 24 %1\n0 24 6 %2", "%2", SidebarPosition::Bottom),
            None
        );
    }

    #[test]
    fn strip_dock_target_picks_the_outermost_workspace_pane_from_the_listing() {
        let listing = "0 0 24 %1\n0 24 6 %2\n0 0 10 %3\n";
        // The floating sidebar %2 excluded, the largest bottom edge is
        // %1 (0+24) for a bottom dock, the topmost pane %1 too (top 0).
        assert_eq!(
            strip_dock_target_from(listing, "%2", SidebarPosition::Bottom),
            Some("%1".to_string())
        );
        assert_eq!(
            strip_dock_target_from(listing, "%2", SidebarPosition::Top),
            Some("%1".to_string())
        );
        // A listing of the sidebar alone names no target.
        assert_eq!(
            strip_dock_target_from("0 24 6 %2", "%2", SidebarPosition::Bottom),
            None
        );
        // Malformed geometry lines are skipped like the sweep skips them.
        assert_eq!(
            strip_dock_target_from("0 0 x %1\n0 24 6 %2", "%2", SidebarPosition::Bottom),
            None
        );
    }

    // ─── the side-switch sweep ───────────────────────────────────────

    // A column-only server, in the eleven-field format: the layout
    // field stays empty for every sidebar, pinning that a sidebar made
    // before the strip layout existed sweeps like a column.
    const COLUMN_LISTING: &str = concat!(
        "@1|%1||0|0|0|20|0|30||0\n",
        "@1|%2|sidebar|0|0|0|20|0|30||0\n",
        "@1|%3|pane|0|0|20|60|0|30||0\n",
        "@2|%4||0|0|0|20|0|30||0\n",
        "@2|%5|sidebar|1|0|60|20|0|30||0\n",
        "@3|%6|sidebar|0|0|40|20|0|30||0\n",
        "@3|%7||0|0|0|40|0|30||0\n",
        "@4|%8|sidebar|0|1|0|80|0|30||0\n",
        "@5|%9||0|0|0|30|0|30||0",
    );

    // Two strips on different edges: window @1 docks its strip at the
    // bottom, window @2 at the top — a top strip an earlier switch
    // skipped. The vertical geometry rides in the same fields.
    const STRIP_LISTING: &str = concat!(
        "@1|%1||0|0|0|80|0|24|strip|0\n",
        "@1|%2|sidebar|0|0|0|80|24|6|strip|0\n",
        "@2|%4||0|0|0|80|0|24|strip|0\n",
        "@2|%5|sidebar|0|0|0|80|0|6|strip|1",
    );

    // A mixed server: a bottom strip in @1, a left column in @2 and a
    // right column in @3 — the layout kind differs per window, so the
    // sweep must read it per sidebar.
    const MIXED_LISTING: &str = concat!(
        "@1|%1||0|0|0|80|0|24|strip|0\n",
        "@1|%2|sidebar|0|0|0|80|24|6|strip|0\n",
        "@2|%4||0|0|0|60|0|30|column|0\n",
        "@2|%5|sidebar|0|0|0|20|0|30|column|0\n",
        "@3|%6|sidebar|0|0|40|20|0|30|column|0\n",
        "@3|%7||0|0|0|40|0|30|column|0",
    );

    fn parse_records(listing: &str) -> Vec<WindowSweepRecord> {
        parse_window_sweep(listing)
    }

    #[test]
    fn parse_window_sweep_groups_panes_by_window_and_flags_the_sidebar() {
        let records = parse_records(COLUMN_LISTING);
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].window_id, "@1");
        assert_eq!(
            records[0].sidebar,
            Some(SweepSidebar {
                pane_id: "%2".to_string(),
                floating: false,
                layout: LayoutKind::Column,
                left: 0,
                at_top: false,
            })
        );
        assert_eq!(records[0].panes.len(), 3);
        // The floating sidebar keeps its record and its flag.
        assert!(records[1].sidebar.as_ref().unwrap().floating);
        // The zoomed flag comes through as a per-window skip signal.
        assert!(records[3].zoomed);
        assert!(!records[0].zoomed);
        // A window with no sidebar still makes a record.
        assert_eq!(records[4].window_id, "@5");
        assert_eq!(records[4].sidebar, None);
    }

    #[test]
    fn parse_window_sweep_reads_the_strip_layout_and_edge_flags() {
        let records = parse_records(STRIP_LISTING);
        // The bottom strip: layout strip, pane_at_top 0.
        let bottom = records[0].sidebar.as_ref().unwrap();
        assert_eq!(bottom.layout, LayoutKind::Strip);
        assert!(!bottom.at_top);
        // The top strip: pane_at_top 1.
        let top = records[1].sidebar.as_ref().unwrap();
        assert_eq!(top.layout, LayoutKind::Strip);
        assert!(top.at_top);
        // The panes carry both geometry pairs, so a strip target can
        // be picked vertically in the same listing a column uses.
        assert_eq!(
            records[0].panes,
            vec![
                SweepPaneGeometry {
                    left: 0,
                    width: 80,
                    top: 0,
                    height: 24,
                    pane_id: "%1".to_string(),
                },
                SweepPaneGeometry {
                    left: 0,
                    width: 80,
                    top: 24,
                    height: 6,
                    pane_id: "%2".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_window_sweep_skips_malformed_lines() {
        let records = parse_window_sweep(
            "bad-line\n@1|%1|sidebar|0|0|x|20|0|30||0\n@1|%2|sidebar|0|0|0|20|0|30||0",
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sidebar.as_ref().unwrap().pane_id, "%2");
        assert_eq!(records[0].panes.len(), 1);
    }

    #[test]
    fn sidebar_edge_reads_the_edge_by_layout_kind() {
        let records = parse_records(MIXED_LISTING);
        // The strip reads pane_at_top: 0 is the bottom edge.
        assert_eq!(
            records[0].sidebar.as_ref().unwrap().edge(),
            SidebarPosition::Bottom
        );
        // The columns read pane_left: 0 is the left edge, anything
        // else the right.
        assert_eq!(
            records[1].sidebar.as_ref().unwrap().edge(),
            SidebarPosition::Left
        );
        assert_eq!(
            records[2].sidebar.as_ref().unwrap().edge(),
            SidebarPosition::Right
        );
    }

    #[test]
    fn side_switch_target_chooses_the_outermost_workspace_pane() {
        let records = parse_records(COLUMN_LISTING);
        // Window @1: sidebar %2 docks left, workspaces %1 (0-20) and
        // %3 (20-80) — the right edge is %3.
        assert_eq!(
            side_switch_target(&records[0], LayoutKind::Column, SidebarPosition::Right),
            Some(("%2".to_string(), "%3".to_string()))
        );
        // Window @3: the sidebar %6 sits right; the leftmost workspace
        // pane is %7.
        assert_eq!(
            side_switch_target(&records[2], LayoutKind::Column, SidebarPosition::Left),
            Some(("%6".to_string(), "%7".to_string()))
        );
    }

    #[test]
    fn side_switch_target_chooses_the_outermost_workspace_pane_vertically_for_a_strip() {
        let records = parse_records(STRIP_LISTING);
        // Window @1: the bottom strip %2 moves to the top edge; the
        // topmost workspace pane is %1 (top 0).
        assert_eq!(
            side_switch_target(&records[0], LayoutKind::Strip, SidebarPosition::Top),
            Some(("%2".to_string(), "%1".to_string()))
        );
        // The same strip moving to the bottom edge joins the pane with
        // the largest bottom edge — %1 again (0+24 beats nothing else;
        // the strip itself is excluded).
        assert_eq!(
            side_switch_target(&records[0], LayoutKind::Strip, SidebarPosition::Bottom),
            Some(("%2".to_string(), "%1".to_string()))
        );
    }

    #[test]
    fn side_switch_target_skips_zoomed_floating_and_sidebar_only_windows() {
        let records = parse_records(COLUMN_LISTING);
        // A zoomed window skips before any tmux call: tmux 3.8's
        // join-pane would unzoom it and move the sidebar anyway.
        assert_eq!(
            side_switch_target(&records[3], LayoutKind::Column, SidebarPosition::Right),
            None
        );
        // The floating sidebar skips before any tmux call.
        assert_eq!(
            side_switch_target(&records[1], LayoutKind::Column, SidebarPosition::Left),
            None
        );
        // A window whose only pane is the sidebar has no join target.
        assert_eq!(
            side_switch_target(&records[3], LayoutKind::Column, SidebarPosition::Right),
            None
        );
    }

    #[test]
    fn a_column_only_server_with_a_skipped_window_sweeps_as_today() {
        // Acting: the left column %2 of @1, target Right. Every sidebar
        // is a column — the empty layout fields read as one — so all of
        // them take the acting target, exactly as the sweep did before
        // strips existed: %2 to the right edge (%3), %6 (already right)
        // also toward Right, the floating %5 and the zoomed %8 skip.
        let records = parse_records(COLUMN_LISTING);
        let mut called = Vec::new();
        let (moved, skipped) = sweep_side_switch(
            &records,
            LayoutKind::Column,
            SidebarPosition::Right,
            |sidebar, target, position| {
                called.push((sidebar.to_string(), target.to_string(), position));
                Ok(())
            },
        );
        assert_eq!((moved, skipped), (2, 2));
        assert_eq!(
            called,
            vec![
                ("%2".to_string(), "%3".to_string(), SidebarPosition::Right),
                ("%6".to_string(), "%7".to_string(), SidebarPosition::Right),
            ]
        );
    }

    #[test]
    fn two_strips_on_different_edges_both_reach_the_target_edge() {
        // Acting: the bottom strip %2 of @1, target Top. Both strips
        // are on the acting axis, so both go to Top — the top strip
        // %5 an earlier switch skipped comes back into line.
        let records = parse_records(STRIP_LISTING);
        let mut called = Vec::new();
        let (moved, skipped) = sweep_side_switch(
            &records,
            LayoutKind::Strip,
            SidebarPosition::Top,
            |sidebar, target, position| {
                called.push((sidebar.to_string(), target.to_string(), position));
                Ok(())
            },
        );
        assert_eq!((moved, skipped), (2, 0));
        assert_eq!(
            called,
            vec![
                ("%2".to_string(), "%1".to_string(), SidebarPosition::Top),
                ("%5".to_string(), "%4".to_string(), SidebarPosition::Top),
            ]
        );
    }

    #[test]
    fn a_mixed_server_moves_each_axis_to_its_own_edge() {
        // Acting: the bottom strip %2 of @1, target Top. The strip
        // takes the acting target (vertical join onto the topmost
        // workspace pane); the columns flip to their own opposite
        // edges — the left column %5 to the right, the right column %6
        // to the left — with horizontal joins.
        let records = parse_records(MIXED_LISTING);
        let mut called = Vec::new();
        let (moved, skipped) = sweep_side_switch(
            &records,
            LayoutKind::Strip,
            SidebarPosition::Top,
            |sidebar, target, position| {
                called.push((sidebar.to_string(), target.to_string(), position));
                Ok(())
            },
        );
        assert_eq!((moved, skipped), (3, 0));
        assert_eq!(
            called,
            vec![
                ("%2".to_string(), "%1".to_string(), SidebarPosition::Top),
                ("%5".to_string(), "%4".to_string(), SidebarPosition::Right),
                ("%6".to_string(), "%7".to_string(), SidebarPosition::Left),
            ]
        );
    }

    #[test]
    fn sweep_counts_a_refused_move_as_skipped() {
        // The mover refuses every call: no window counts as moved, one
        // skip per sidebar-bearing window, and the floating and zoomed
        // windows skip without a call at all. The sidebar-less @5 is
        // not counted — a window with no sidebar is not a participant.
        let records = parse_records(COLUMN_LISTING);
        let mut called = Vec::new();
        let (moved, skipped) = sweep_side_switch(
            &records,
            LayoutKind::Column,
            SidebarPosition::Right,
            |sidebar, target, _| {
                called.push((sidebar.to_string(), target.to_string()));
                Err("can't rearrange".to_string())
            },
        );
        assert_eq!((moved, skipped), (0, 4));
        // Only the two windows with a usable target were called; the
        // floating and zoomed ones never reached tmux.
        assert_eq!(called.len(), 2);
        assert_eq!(called[0], ("%2".to_string(), "%3".to_string()));
    }

    #[test]
    fn sweep_counts_successful_moves_and_leaves_skips_out_of_the_count() {
        let records = parse_records(COLUMN_LISTING);
        let (moved, skipped) = sweep_side_switch(
            &records,
            LayoutKind::Column,
            SidebarPosition::Right,
            |_, _, _| Ok(()),
        );
        // Two moves, two skips (the floating and zoomed sidebars) —
        // the sidebar-less window counts toward neither.
        assert_eq!((moved, skipped), (2, 2));
    }
}
