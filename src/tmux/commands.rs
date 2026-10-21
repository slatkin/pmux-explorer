use std::process::Command;

pub fn run_tmux(args: &[&str]) -> Option<String> {
    let output = Command::new("tmux").args(args).output().ok()?;
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
    let output = Command::new("tmux")
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
}
