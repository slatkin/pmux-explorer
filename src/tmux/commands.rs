use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cli::toggle::{SidebarPosition, target_pane_for_position};
use crate::tmux::PANE_ROLE;

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

/// Move `pane` (a window's sidebar) to the left or right edge of its
/// window by joining `target`, the outermost workspace pane on the new
/// side. The pane's process is untouched — tmux only re-parents the
/// pane, so the running TUI keeps its state and just resizes.
fn move_pane_to_side(pane: &str, target: &str, position: SidebarPosition) -> Result<(), String> {
    let owned = join_pane_side_move_args(pane, target, position);
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    run_tmux_capture(&args).map(|_| ())
}

/// One window of the side-switch sweep, parsed from a single
/// `list-panes -a` call: the window's id, its sidebar pane (with the
/// floating flag) if it has one, whether the window is zoomed, and
/// every pane's geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowSweepRecord {
    pub window_id: String,
    /// The window's sidebar pane id and its floating flag; `None` when
    /// the window has no sidebar.
    pub sidebar: Option<(String, bool)>,
    /// Whether the window is zoomed (`#{window_zoomed_flag}`). tmux
    /// 3.8's `join-pane` does not refuse a zoomed window — it unzooms
    /// it and succeeds — so the sweep must skip it before the call.
    pub zoomed: bool,
    /// Geometry of every pane in the window, sidebar included, parsed
    /// from `#{pane_left}|#{pane_width}|#{pane_id}` fields.
    pub panes: Vec<(u32, u32, String)>,
}

/// Parse the sweep's `list-panes -a` output into per-window records,
/// in encounter order. Lines that do not split into the six pipe
/// fields are skipped; a floating flag that tmux cannot report (older
/// than 3.8) parses as not floating. The role field selects which
/// pane is the window's sidebar; a window with two sidebar panes
/// keeps the first.
pub(crate) fn parse_window_sweep(output: &str) -> Vec<WindowSweepRecord> {
    fn push_fields(
        record: &mut WindowSweepRecord,
        pane_id: &str,
        role: &str,
        floating: &str,
        zoomed: &str,
        left: u32,
        width: u32,
    ) {
        if role == "sidebar" && record.sidebar.is_none() {
            record.sidebar = Some((pane_id.to_string(), floating == "1"));
        }
        if zoomed == "1" {
            record.zoomed = true;
        }
        record.panes.push((left, width, pane_id.to_string()));
    }
    let mut records: Vec<WindowSweepRecord> = Vec::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split('|').collect();
        let [window_id, pane_id, role, floating, zoomed, left, width] = fields[..] else {
            continue;
        };
        let Ok(left) = left.parse::<u32>() else {
            continue;
        };
        let Ok(width) = width.parse::<u32>() else {
            continue;
        };
        match records.last_mut() {
            Some(record) if record.window_id == window_id => {
                push_fields(record, pane_id, role, floating, zoomed, left, width);
            }
            _ => {
                let mut record = WindowSweepRecord {
                    window_id: window_id.to_string(),
                    sidebar: None,
                    zoomed: false,
                    panes: Vec::new(),
                };
                push_fields(&mut record, pane_id, role, floating, zoomed, left, width);
                records.push(record);
            }
        }
    }
    records
}

/// Choose the move for one window of the sweep: the sidebar pane and
/// the outermost workspace pane on the new side. `None` when the
/// window is zoomed (tmux 3.8's `join-pane` would unzoom it and move
/// the sidebar — the zoom must win, so skip before the call), its
/// sidebar floats, the window has no sidebar, or no workspace pane to
/// join onto (the sidebar is alone).
pub(crate) fn side_switch_target(
    record: &WindowSweepRecord,
    position: SidebarPosition,
) -> Option<(String, String)> {
    if record.zoomed {
        return None;
    }
    let (sidebar, floating) = record.sidebar.as_ref()?;
    if *floating {
        return None;
    }
    let workspace = record
        .panes
        .iter()
        .filter(|(_, _, pane_id)| pane_id != sidebar)
        .map(|(left, width, pane_id)| format!("{left} {width} {pane_id}"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = target_pane_for_position(&workspace, position)?;
    Some((sidebar.clone(), target))
}

/// The sweep core over already-parsed records: move every window's
/// sidebar to the new side and count moved and skipped windows. A
/// window with no sidebar is not a participant — it is neither moved
/// nor skipped, so the count names only the sidebars the sweep could
/// not move: no usable target (see [`side_switch_target`]) or tmux
/// refused the move (a zoomed window, for one). The injected `mover`
/// keeps the counting unit-testable without a live tmux.
fn sweep_side_switch<F>(
    records: &[WindowSweepRecord],
    position: SidebarPosition,
    mut mover: F,
) -> (usize, usize)
where
    F: FnMut(&str, &str) -> Result<(), String>,
{
    let mut moved = 0;
    let mut skipped = 0;
    for record in records {
        if record.sidebar.is_none() {
            continue;
        }
        match side_switch_target(record, position) {
            None => skipped += 1,
            Some((sidebar, target)) => match mover(&sidebar, &target) {
                Ok(()) => moved += 1,
                Err(_) => skipped += 1,
            },
        }
    }
    (moved, skipped)
}

/// Move the sidebar of every window to `position` in one sweep, from a
/// single `list-panes -a` listing. Returns the counts of moved and
/// skipped windows over the sidebar-bearing windows only: a floating
/// sidebar skips without a tmux call, and any refused `join-pane`
/// (e.g. a zoomed window) skips too. A window with no sidebar does
/// not count toward either number.
pub(crate) fn switch_sidebar_sides(position: SidebarPosition) -> (usize, usize) {
    let format = format!(
        "#{{window_id}}|#{{pane_id}}|#{{{}}}|#{{pane_floating_flag}}|#{{window_zoomed_flag}}|#{{pane_left}}|#{{pane_width}}",
        PANE_ROLE
    );
    let Some(output) = run_tmux(&["list-panes", "-a", "-F", &format]) else {
        return (0, 0);
    };
    let records = parse_window_sweep(&output);
    sweep_side_switch(&records, position, |sidebar, target| {
        move_pane_to_side(sidebar, target, position)
    })
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

    // ─── the side-switch sweep ───────────────────────────────────────

    const SWEEP_LISTING: &str = concat!(
        "@1|%1||0|0|0|20\n",
        "@1|%2|sidebar|0|0|0|20\n",
        "@1|%3|pane|0|0|20|60\n",
        "@2|%4||0|0|0|20\n",
        "@2|%5|sidebar|1|0|60|20\n",
        "@3|%6|sidebar|0|0|40|20\n",
        "@3|%7||0|0|0|40\n",
        "@4|%8|sidebar|0|1|0|80\n",
        "@5|%9||0|0|0|30",
    );

    fn sweep_records() -> Vec<WindowSweepRecord> {
        parse_window_sweep(SWEEP_LISTING)
    }

    #[test]
    fn parse_window_sweep_groups_panes_by_window_and_flags_the_sidebar() {
        let records = sweep_records();
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].window_id, "@1");
        assert_eq!(records[0].sidebar, Some(("%2".to_string(), false)));
        assert_eq!(records[0].panes.len(), 3);
        // The floating sidebar keeps its record and its flag.
        assert_eq!(records[1].sidebar, Some(("%5".to_string(), true)));
        // The zoomed flag comes through as a per-window skip signal.
        assert!(records[3].zoomed);
        assert!(!records[0].zoomed);
        // A window with no sidebar still makes a record.
        assert_eq!(records[4].window_id, "@5");
        assert_eq!(records[4].sidebar, None);
    }

    #[test]
    fn parse_window_sweep_skips_malformed_lines() {
        let records =
            parse_window_sweep("bad-line\n@1|%1|sidebar|0|0|x|20\n@1|%2|sidebar|0|0|0|20");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sidebar, Some(("%2".to_string(), false)));
        assert_eq!(records[0].panes.len(), 1);
    }

    #[test]
    fn side_switch_target_chooses_the_outermost_workspace_pane() {
        let records = sweep_records();
        // Window @1: sidebar %2 docks left, workspaces %1 (0-20) and
        // %3 (20-80) — the right edge is %3.
        assert_eq!(
            side_switch_target(&records[0], SidebarPosition::Right),
            Some(("%2".to_string(), "%3".to_string()))
        );
        // Window @3: the sidebar %6 sits right; the leftmost workspace
        // pane is %7.
        assert_eq!(
            side_switch_target(&records[2], SidebarPosition::Left),
            Some(("%6".to_string(), "%7".to_string()))
        );
    }

    #[test]
    fn side_switch_target_skips_zoomed_floating_and_sidebar_only_windows() {
        let records = sweep_records();
        // A zoomed window skips before any tmux call: tmux 3.8's
        // join-pane would unzoom it and move the sidebar anyway.
        assert_eq!(
            side_switch_target(&records[3], SidebarPosition::Right),
            None
        );
        // The floating sidebar skips before any tmux call.
        assert_eq!(side_switch_target(&records[1], SidebarPosition::Left), None);
        // A window whose only pane is the sidebar has no join target.
        assert_eq!(
            side_switch_target(&records[3], SidebarPosition::Right),
            None
        );
    }

    #[test]
    fn sweep_counts_a_refused_move_as_skipped() {
        // The mover refuses every call: no window counts as moved, one
        // skip per sidebar-bearing window, and the floating and zoomed
        // windows skip without a call at all. The sidebar-less @5 is
        // not counted — a window with no sidebar is not a participant.
        let records = sweep_records();
        let mut called = Vec::new();
        let (moved, skipped) =
            sweep_side_switch(&records, SidebarPosition::Right, |sidebar, target| {
                called.push((sidebar.to_string(), target.to_string()));
                Err("can't rearrange".to_string())
            });
        assert_eq!((moved, skipped), (0, 4));
        // Only the two windows with a usable target were called; the
        // floating and zoomed ones never reached tmux.
        assert_eq!(called.len(), 2);
        assert_eq!(called[0], ("%2".to_string(), "%3".to_string()));
    }

    #[test]
    fn sweep_counts_successful_moves_and_leaves_skips_out_of_the_count() {
        let records = sweep_records();
        let (moved, skipped) = sweep_side_switch(&records, SidebarPosition::Right, |_, _| Ok(()));
        // Two moves, two skips (the floating and zoomed sidebars) —
        // the sidebar-less window counts toward neither.
        assert_eq!((moved, skipped), (2, 2));
    }
}
