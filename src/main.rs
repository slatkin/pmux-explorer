use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use pmux_explorer::{app, tmux};
use ratatui::{Terminal, backend::CrosstermBackend};

static NEEDS_REFRESH: AtomicBool = AtomicBool::new(false);
/// Set by SIGTERM/SIGHUP so the event loop returns and the drop guards run.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

struct TuiSession {
    entered_alt_screen: bool,
}

impl TuiSession {
    fn enter(stdout: &mut io::Stdout) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(err) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            // Best effort: the terminal setup error is the one to report, and
            // a raw mode that survives it is reset by the next run or shell.
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self {
            entered_alt_screen: true,
        })
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        // Best effort in Drop: there is no caller to report to, and leaving
        // the terminal usable outranks surfacing a reset error on exit.
        let _ = disable_raw_mode();
        if self.entered_alt_screen {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        }
    }
}

/// Owns the pane's `@sidebar_pid` option for the life of the process. The
/// option marks the pane as a sidebar (key bindings and the focus hook read
/// it), so it must not outlive the process: a pane that falls back to a
/// shell and then runs an agent would otherwise still pass for a sidebar.
struct SidebarPidGuard {
    pane: String,
}

impl Drop for SidebarPidGuard {
    fn drop(&mut self) {
        tmux::unset_pane_option(&self.pane, tmux::SIDEBAR_PID);
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = pmux_explorer::cli::run(&args) {
        std::process::exit(code);
    }

    let tmux_pane = std::env::var("TMUX_PANE").unwrap_or_default();
    if tmux_pane.is_empty() {
        eprintln!("TMUX_PANE not set");
        std::process::exit(1);
    }

    // SAFETY: `zeroed` is the valid initial state for `libc::sigaction` (a
    // plain C struct whose zero is "no flags, default mask, no handler"),
    // every field read by the call is written below, `sa_sigaction` holds a
    // valid `extern "C" fn(c_int)` cast per the libc convention, and the
    // call runs once at startup before any thread exists.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigusr1_handler as *const () as libc::sighandler_t;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigaction(libc::SIGUSR1, &sa, std::ptr::null_mut());

        // SIGTERM (`kill`, `pkill`) ends the loop instead of the process,
        // so the guards below clear the pane option and restore the
        // terminal. SA_RESETHAND puts the default action back after the
        // first signal: a loop that never returns can still be killed by a
        // second one.
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = shutdown_handler as *const () as libc::sighandler_t;
        sa.sa_flags = libc::SA_RESTART | libc::SA_RESETHAND;
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());

        // SIGHUP (pane closing, tmux server dying) must exit from the
        // handler itself: a closed pty makes crossterm's event source spin
        // on EIO inside `poll` without ever returning, so a flag the loop
        // checks on its next iteration is never seen (observed: orphaned
        // sidebars pinning a core after `kill-server`). The terminal is
        // already dead, so there is nothing for the guards to restore and
        // `_exit` — the only async-signal-safe way out — is correct.
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = hangup_handler as *const () as libc::sighandler_t;
        sa.sa_flags = libc::SA_RESETHAND;
        libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
    }

    let pid = std::process::id();
    let _ = tmux::tmux_command()
        .args([
            "set",
            "-t",
            &tmux_pane,
            "-p",
            tmux::SIDEBAR_PID,
            &pid.to_string(),
        ])
        .output();
    let _sidebar_pid = SidebarPidGuard {
        pane: tmux_pane.clone(),
    };

    let mut stdout = io::stdout();
    let _tui_session = TuiSession::enter(&mut stdout)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app::run(&mut terminal, tmux_pane, &NEEDS_REFRESH, &SHUTDOWN)
}

extern "C" fn sigusr1_handler(_: libc::c_int) {
    NEEDS_REFRESH.store(true, Ordering::Relaxed);
}

extern "C" fn shutdown_handler(_: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

extern "C" fn hangup_handler(_: libc::c_int) {
    // SAFETY: `_exit` is async-signal-safe and never returns.
    unsafe { libc::_exit(0) }
}
