pub mod capture;
mod hook;
mod label;
pub mod plugin_state;
pub(crate) mod setup;
pub(crate) mod shared_html;
mod spawn;
pub(crate) mod toggle;

use std::io::Read;
use std::time::Duration;

use crate::mcp::{self, ArmError};
use crate::task_reminders;
use crate::tmux;

/// Run a CLI subcommand. Returns Some(exit_code) if a subcommand was matched,
/// None if no subcommand was given (caller should launch TUI).
pub fn run(args: &[String]) -> Option<i32> {
    // `--mcp` is the MCP channel server mode: a flag, and a mode of the
    // binary like the TUI — not a one-shot word subcommand (design
    // decision D6). It serves stdio until stdin's EOF and exits 0.
    if args.first().map(String::as_str) == Some("--mcp") {
        return Some(mcp::run_server());
    }
    let cmd = args.first().map(|s| s.as_str())?;
    let rest = &args[1..];
    let code = match cmd {
        "setup" => setup::cmd_setup(rest),
        "hook" => hook::cmd_hook(rest),
        "toggle" => toggle::cmd_toggle(rest),
        "toggle-all" => toggle::cmd_toggle_all(rest),
        "auto-close" => toggle::cmd_auto_close(rest),
        "set-status" => cmd_set_status(rest),
        "task-set" => cmd_task_set(rest),
        "task-update" => cmd_task_update(rest),
        "spawn" => spawn::cmd_spawn(rest),
        "capture" => capture::cmd_capture(rest),
        "channel-ready" => cmd_channel_ready(rest),
        "--version" | "version" => {
            println!("{}", crate::VERSION);
            0
        }
        _ => return None,
    };
    Some(code)
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn read_stdin_json() -> serde_json::Value {
    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) != 0 };
    if is_tty {
        return serde_json::Value::Null;
    }
    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);
    serde_json::from_str(&buf).unwrap_or(serde_json::Value::Null)
}

fn tmux_pane() -> String {
    std::env::var("TMUX_PANE").unwrap_or_default()
}

fn local_time_hhmm() -> String {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    }
}

fn set_status(pane: &str, status: &str) {
    if status == "clear" {
        tmux::unset_pane_option(pane, tmux::PANE_STATUS);
        tmux::unset_pane_option(pane, tmux::PANE_ATTENTION);
    } else {
        tmux::set_pane_option(pane, tmux::PANE_STATUS, status);
        match status {
            "running" | "idle" => {
                tmux::unset_pane_option(pane, tmux::PANE_ATTENTION);
            }
            _ => {}
        }
    }
}

fn set_attention(pane: &str, state: &str) {
    if state == "clear" {
        tmux::unset_pane_option(pane, tmux::PANE_ATTENTION);
    } else {
        tmux::set_pane_option(pane, tmux::PANE_ATTENTION, state);
    }
}

/// Canonicalize a string before storing it in a tmux pane option. Tmux's
/// pane-option format uses `|` as a field separator and `\n` as a record
/// terminator, so both must be replaced with spaces to keep the stored
/// value a single safe field. Readers that compare against raw process
/// data (e.g. the bg-shell ps sweep) must apply the same normalization
/// so round-tripping through storage doesn't silently break equality.
pub(crate) fn sanitize_tmux_value(s: &str) -> String {
    s.replace(['\n', '|'], " ")
}

// ─── channel-ready subcommand ─────────────────────────────────────────────

/// Run `channel-ready`: one arm request to this pane's endpoint.
///
/// The answer window is the endpoint's own read timeout (2 seconds), so a
/// server that never replies fails at the same pace the socket rules set.
fn cmd_channel_ready(args: &[String]) -> i32 {
    let tmux = std::env::var("TMUX").ok();
    let tmux_pane = std::env::var("TMUX_PANE").ok();
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
    let (stdout, stderr, code) = run_channel_ready(
        nonce_argument(args).as_deref(),
        tmux.as_deref(),
        tmux_pane.as_deref(),
        xdg_runtime_dir.as_deref(),
        current_uid(),
        mcp::DEFAULT_READ_TIMEOUT,
    );
    if let Some(line) = stdout {
        println!("{line}");
    }
    if let Some(line) = stderr {
        eprintln!("{line}");
    }
    code
}

/// The `--nonce <n>` value in a `channel-ready` argument list: the word
/// after `--nonce`, or the tail of `--nonce=<n>`. A bare `--nonce` with
/// no value counts as missing, like its absence.
fn nonce_argument(args: &[String]) -> Option<String> {
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if let Some(value) = word.strip_prefix("--nonce=") {
            return (!value.is_empty()).then(|| value.to_string());
        }
        if word == "--nonce" {
            return words.next().cloned();
        }
    }
    None
}

/// Resolves one `channel-ready` invocation into the line stdout prints
/// (the endpoint's reply), the line stderr prints (a failure message), and
/// the exit code, without printing — so the tests run it against a real
/// socket in a temp dir without touching the process env.
fn run_channel_ready(
    nonce: Option<&str>,
    tmux: Option<&str>,
    tmux_pane: Option<&str>,
    xdg_runtime_dir: Option<&str>,
    uid: u32,
    answer_timeout: Duration,
) -> (Option<String>, Option<String>, i32) {
    let Some(nonce) = nonce else {
        return (
            None,
            Some("channel-ready requires --nonce <nonce>".to_string()),
            1,
        );
    };
    let Some(path) = mcp::endpoint_path(tmux, tmux_pane, xdg_runtime_dir, uid) else {
        return (
            None,
            Some(
                "channel-ready: TMUX and TMUX_PANE must name the pane with the endpoint"
                    .to_string(),
            ),
            1,
        );
    };
    match mcp::send_arm(&path, nonce, answer_timeout) {
        Ok(reply) => {
            let armed = serde_json::from_str::<serde_json::Value>(&reply)
                .ok()
                .and_then(|value| value.get("armed").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            (Some(reply), None, if armed { 0 } else { 1 })
        }
        Err(ArmError::Unreachable(error)) => (
            None,
            Some(format!(
                "channel-ready: no endpoint at {} ({error})",
                path.display(),
            )),
            1,
        ),
        Err(ArmError::NoAnswer(_)) => (
            None,
            Some(format!(
                "channel-ready: the endpoint at {} did not answer within {answer_timeout:?}",
                path.display(),
            )),
            1,
        ),
    }
}

/// The current user's id, for the endpoint path's `/tmp` fallback.
fn current_uid() -> u32 {
    // SAFETY: `geteuid` has no failure mode and no preconditions.
    unsafe { libc::geteuid() }
}

// ─── set-status subcommand ──────────────────────────────────────────────────

fn cmd_set_status(args: &[String]) -> i32 {
    let status = match args.first() {
        Some(s) => s.as_str(),
        None => return 0,
    };
    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }
    set_status(&pane, status);
    0
}

// ─── task-set / task-update subcommands ─────────────────────────────────────

/// Decide whether a task command invocation should print help instead
/// of acting: too few args for the command to do anything (`task-set`
/// needs a subject, `task-update` needs an id and a status), or a help
/// token as the ONLY argument — an invocation with no payload to act
/// on. Once anything follows the token, the words belong to the
/// subject: a subject starting with `help` or `-h` still stores its
/// whole text instead of being silently swallowed by help. Help
/// remains reachable as the bare token (`task-set --help`).
fn task_wants_help(args: &[String], min_args: usize) -> bool {
    args.len() < min_args
        || (args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h" | "help"))
}

/// Help text for the task subcommands. The reminder lines embed the
/// constants from `src/task_reminders.rs` verbatim — that module is the
/// single wording source the help, the protocol doc, and the pmux
/// companion change all reference, so this builder must not paraphrase.
fn task_help_text() -> String {
    format!(
        "task-set <subject>             record a pending task item for the calling pane\n\
         task-update <id> <status>      update one item's status (pending|in_progress|completed)\n\
         \n\
         reminders:\n\
         a turn starting with an empty todo list gets: {}\n\
         a turn ending with open items gets: {}",
        task_reminders::RECORD_FIRST_REMINDER,
        task_reminders::UPDATE_BEFORE_FINISH_REMINDER,
    )
}

/// Record a pending task item for the calling pane. Exits 0 on every
/// path — including a missing `$TMUX_PANE` — so a task command can
/// never break the agent's turn, same contract as the hook command.
/// On success the assigned item id is printed so the agent can name it
/// in a later `task-update`.
fn cmd_task_set(args: &[String]) -> i32 {
    if task_wants_help(args, 1) {
        println!("{}", task_help_text());
        return 0;
    }
    // Agents quote subjects unreliably, so every argument after
    // `task-set` joins into the subject: `task-set Fix the login
    // crash` records the whole words instead of storing "Fix" and
    // exiting 0 with the rest lost.
    let subject = args.join(" ");
    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }
    task_set_for_pane(&pane, &subject)
}

fn task_set_for_pane(pane: &str, subject: &str) -> i32 {
    if let Some(id) = crate::task_store::record_task(pane, subject) {
        println!("{id}");
    }
    0
}

/// Update one task item's status for the calling pane. Exits 0 on
/// every path; an unknown id or status name changes nothing.
fn cmd_task_update(args: &[String]) -> i32 {
    if task_wants_help(args, 2) {
        println!("{}", task_help_text());
        return 0;
    }
    let id = args[0].as_str();
    let status = args[1].as_str();
    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }
    task_update_for_pane(&pane, id, status)
}

fn task_update_for_pane(pane: &str, id: &str, status: &str) -> i32 {
    if let Some(status) = crate::task_store::parse_task_status(status)
        && crate::task_store::update_task_status(pane, id, status)
    {
        println!("{} {}", id, status.as_str());
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::json_str;
    use crate::mcp::EndpointState;
    use serde_json::json;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // ─── run() dispatch tests ─────────────────────────────────────────

    #[test]
    fn run_returns_none_for_empty_args() {
        assert_eq!(run(&[]), None);
    }

    #[test]
    fn run_returns_none_for_unknown_command() {
        assert_eq!(run(&["unknown-cmd".into()]), None);
    }

    #[test]
    fn run_returns_none_for_tui_mode_no_args() {
        assert_eq!(run(&[]), None);
    }

    // ─── json_str tests ──────────────────────────────────────────────

    #[test]
    fn json_str_extracts_string() {
        let val = json!({"name": "claude", "count": 42});
        assert_eq!(json_str(&val, "name"), "claude");
    }

    #[test]
    fn json_str_returns_empty_for_missing_key() {
        assert_eq!(json_str(&json!({"name": "claude"}), "missing"), "");
    }

    #[test]
    fn json_str_returns_empty_for_non_string() {
        assert_eq!(json_str(&json!({"count": 42}), "count"), "");
    }

    #[test]
    fn json_str_returns_empty_for_null() {
        assert_eq!(json_str(&serde_json::Value::Null, "anything"), "");
    }

    #[test]
    fn json_str_returns_empty_for_array() {
        assert_eq!(json_str(&json!({"arr": [1, 2]}), "arr"), "");
    }

    #[test]
    fn json_str_returns_empty_for_bool() {
        assert_eq!(json_str(&json!({"flag": true}), "flag"), "");
    }

    // ─── sanitize_tmux_value tests ───────────────────────────────────

    #[test]
    fn sanitize_replaces_newlines() {
        assert_eq!(
            sanitize_tmux_value("line1\nline2\nline3"),
            "line1 line2 line3"
        );
    }

    #[test]
    fn sanitize_replaces_pipes() {
        assert_eq!(sanitize_tmux_value("a|b|c"), "a b c");
    }

    #[test]
    fn sanitize_replaces_both() {
        assert_eq!(sanitize_tmux_value("a|b\nc"), "a b c");
    }

    #[test]
    fn sanitize_leaves_clean_text() {
        assert_eq!(sanitize_tmux_value("hello world"), "hello world");
    }

    #[test]
    fn sanitize_empty() {
        assert_eq!(sanitize_tmux_value(""), "");
    }

    #[test]
    fn sanitize_consecutive_pipes_and_newlines() {
        assert_eq!(sanitize_tmux_value("a||\n\nb"), "a    b");
    }

    // ─── local_time_hhmm tests ──────────────────────────────────────

    #[test]
    fn local_time_hhmm_format() {
        let t = local_time_hhmm();
        assert_eq!(t.len(), 5);
        assert_eq!(t.as_bytes()[2], b':');
        let h: u32 = t[..2].parse().unwrap();
        let m: u32 = t[3..].parse().unwrap();
        assert!(h < 24);
        assert!(m < 60);
    }

    // ─── channel-ready tests ─────────────────────────────────────────

    /// The probe nonce the test endpoint carries.
    const CHANNEL_NONCE: &str = "00112233445566778899aabbccddeeff";

    /// The tmux env values and uid the test endpoint is derived from.
    const TEST_TMUX: &str = "/tmp/tmux-1000/default,1,0";
    const TEST_PANE: &str = "%12";

    fn own_uid() -> u32 {
        // SAFETY: `geteuid` has no failure mode and no preconditions.
        unsafe { libc::geteuid() }
    }

    /// Serves an unarmed endpoint state — the probe nonce, a fixed clock —
    /// at the endpoint the env triple derives to inside `dir`, and returns
    /// the env triple and the shared state.
    fn served_endpoint(dir: &std::path::Path) -> ((String, String, String), Arc<EndpointState>) {
        let xdg = dir.to_string_lossy().to_string();
        let path =
            crate::mcp::endpoint_path(Some(TEST_TMUX), Some(TEST_PANE), Some(&xdg), own_uid())
                .unwrap();
        let listener = crate::mcp::bind(&path, own_uid()).unwrap();
        let probe = crate::mcp::Probe::new(
            CHANNEL_NONCE.to_string(),
            std::path::Path::new("/usr/bin/pmux-explorer"),
            Box::new(|| 0),
        );
        let state = Arc::new(EndpointState::new(own_uid(), TEST_PANE.to_string(), probe));
        // The loop runs until the listener fails, which never happens in
        // this test, so the thread is left detached after the exchange.
        let dir = std::sync::Mutex::new(Some(dir.to_path_buf()));
        let served = Arc::clone(&state);
        thread::spawn(move || {
            let _keep_dir_alive = dir;
            let _ = crate::mcp::accept_loop(
                listener,
                served,
                crate::mcp::LineWriter::new(std::io::sink()),
                mcp::DEFAULT_READ_TIMEOUT,
            );
        });
        ((TEST_TMUX.to_string(), TEST_PANE.to_string(), xdg), state)
    }

    /// The env triple `run_channel_ready` takes, from the helper's tuple.
    fn env_of(
        served: &(String, String, String),
    ) -> (Option<String>, Option<String>, Option<String>) {
        (
            Some(served.0.clone()),
            Some(served.1.clone()),
            Some(served.2.clone()),
        )
    }

    // Row 4.2: the "Channel-ready command" scenarios, against a socket in
    // a temp dir.

    #[test]
    fn the_correct_nonce_answers_armed_and_exits_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let (served, _) = served_endpoint(tmp.path());
        let (stdout, stderr, code) = run_channel_ready(
            Some(CHANNEL_NONCE),
            Some(served.0.as_str()),
            Some(served.1.as_str()),
            Some(served.2.as_str()),
            own_uid(),
            mcp::DEFAULT_READ_TIMEOUT,
        );
        assert_eq!(code, 0);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stdout.unwrap()).unwrap(),
            json!({"ok": true, "armed": true}),
        );
        assert_eq!(stderr, None);
    }

    #[test]
    fn a_nonce_reply_prints_and_exits_one() {
        let tmp = tempfile::tempdir().unwrap();
        let (served, _) = served_endpoint(tmp.path());
        let (stdout, stderr, code) = run_channel_ready(
            Some("f".repeat(32).as_str()),
            Some(served.0.as_str()),
            Some(served.1.as_str()),
            Some(served.2.as_str()),
            own_uid(),
            mcp::DEFAULT_READ_TIMEOUT,
        );
        assert_eq!(code, 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stdout.unwrap()).unwrap(),
            json!({"ok": false, "reason": "nonce"}),
        );
        assert_eq!(stderr, None);
    }

    #[test]
    fn a_missing_endpoint_exits_one_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        // The env triple names a pane whose socket was never created.
        let env = (
            TEST_TMUX.to_string(),
            TEST_PANE.to_string(),
            tmp.path().to_string_lossy().to_string(),
        );
        let (stdout, stderr, code) = run_channel_ready(
            Some(CHANNEL_NONCE),
            Some(env.0.as_str()),
            Some(env.1.as_str()),
            Some(env.2.as_str()),
            own_uid(),
            mcp::DEFAULT_READ_TIMEOUT,
        );
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("no endpoint"), "message: {message}");
        let expected = crate::mcp::endpoint_path(
            Some(TEST_TMUX),
            Some(TEST_PANE),
            Some(tmp.path().to_string_lossy().as_ref()),
            own_uid(),
        )
        .unwrap();
        assert!(message.contains(&expected.to_string_lossy().to_string()));
    }

    #[test]
    fn a_missing_nonce_exits_one_names_the_flag_and_sends_no_request() {
        let tmp = tempfile::tempdir().unwrap();
        let (served, state) = served_endpoint(tmp.path());
        let env = env_of(&served);
        let (stdout, stderr, code) = run_channel_ready(
            None,
            env.0.as_deref(),
            env.1.as_deref(),
            env.2.as_deref(),
            own_uid(),
            mcp::DEFAULT_READ_TIMEOUT,
        );
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("--nonce"), "message: {message}");
        // No request reached the endpoint, so it never armed.
        assert!(!state.armed.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn an_endpoint_that_never_answers_exits_one_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        // A raw listener that accepts and answers nothing, so the client's
        // short answer window ends the wait instead of the real 2 seconds.
        let path = crate::mcp::endpoint_path(
            Some(TEST_TMUX),
            Some(TEST_PANE),
            Some(tmp.path().to_string_lossy().as_ref()),
            own_uid(),
        )
        .unwrap();
        let listener = crate::mcp::bind(&path, own_uid()).unwrap();
        thread::spawn(move || {
            let _ = listener.accept();
            thread::sleep(Duration::from_secs(2));
        });
        let (stdout, stderr, code) = run_channel_ready(
            Some(CHANNEL_NONCE),
            Some(TEST_TMUX),
            Some(TEST_PANE),
            Some(tmp.path().to_string_lossy().as_ref()),
            own_uid(),
            Duration::from_millis(150),
        );
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("did not answer"), "message: {message}");
        assert!(message.contains(&path.to_string_lossy().to_string()));
    }

    // Row 4.2: the --nonce argument forms.

    #[test]
    fn the_nonce_argument_takes_both_flag_forms() {
        assert_eq!(
            nonce_argument(&["--nonce".to_string(), "abc".to_string()]),
            Some("abc".to_string()),
        );
        assert_eq!(
            nonce_argument(&["--nonce=abc".to_string()]),
            Some("abc".to_string()),
        );
        assert_eq!(nonce_argument(&[]), None);
        assert_eq!(nonce_argument(&["--nonce".to_string()]), None);
        assert_eq!(nonce_argument(&["--nonce=".to_string()]), None);
    }

    #[test]
    fn the_channel_ready_dispatch_maps_to_run() {
        // A missing --nonce returns before any env read or socket touch,
        // so the dispatch mapping can be asserted in any environment.
        assert_eq!(run(&["channel-ready".to_string()]), Some(1));
    }

    // ─── task-set / task-update tests ────────────────────────────────

    #[test]
    fn task_commands_exit_zero_without_a_pane() {
        // A missing $TMUX_PANE reads as an empty pane id: both task
        // commands no-op without touching any store file, and still
        // exit 0 so the agent's turn cannot be broken.
        let before = crate::task_store::store_mtime("");

        assert_eq!(task_set_for_pane("", "orphan subject"), 0);
        assert_eq!(task_update_for_pane("", "1", "completed"), 0);

        assert_eq!(
            crate::task_store::store_mtime(""),
            before,
            "no pane env must not touch any store file"
        );
    }

    #[test]
    fn task_set_dispatch_maps_to_run() {
        // No subject → exits before any env read or store write, so the
        // dispatch mapping can be asserted without touching $TMUX_PANE.
        assert_eq!(run(&["task-set".into()]), Some(0));
        assert_eq!(run(&["task-update".into()]), Some(0));
    }

    #[test]
    fn task_update_unknown_status_or_id_is_a_noop_exit_zero() {
        let pane = "%CLI_TASK_UPDATE_NOOP";
        let path = crate::task_store::store_file_path(pane);
        let _ = std::fs::remove_file(&path);
        crate::task_store::record_task(pane, "real item").unwrap();
        let before = crate::task_store::store_mtime(pane);

        assert_eq!(task_update_for_pane(pane, "1", "archived"), 0);
        assert_eq!(task_update_for_pane(pane, "999", "completed"), 0);
        assert_eq!(
            crate::task_store::store_mtime(pane),
            before,
            "unknown status or id must not rewrite the store"
        );
        assert_eq!(
            crate::task_store::read_tasks(pane)[0].status,
            crate::task_store::TaskItemStatus::Pending
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn task_update_without_pane_env_exits_zero_and_writes_nothing() {
        let before = crate::task_store::store_mtime("");

        let code = task_update_for_pane("", "1", "completed");

        assert_eq!(code, 0);
        assert_eq!(crate::task_store::store_mtime(""), before);
    }

    #[test]
    fn task_help_carries_both_reminder_strings_verbatim() {
        // The task-command help is the first surface that references the
        // single-source reminder wording (src/task_reminders.rs); the
        // protocol doc and the pi proof prompt reuse the same strings.
        // If help ever paraphrases, the strings a downstream surface
        // copies from this file stop matching what an agent reads here.
        let help = task_help_text();
        assert!(
            help.contains(crate::task_reminders::RECORD_FIRST_REMINDER),
            "help must embed the record-first wording verbatim"
        );
        assert!(
            help.contains(crate::task_reminders::UPDATE_BEFORE_FINISH_REMINDER),
            "help must embed the update-before-finish wording verbatim"
        );
    }

    #[test]
    fn task_commands_print_help_for_flag_or_missing_args() {
        // Help wins only when the arity cannot act or when the help
        // token is the ONLY argument: an invocation with a payload
        // always acts, so no subject is silently swallowed behind help.
        assert!(task_wants_help(&["--help".to_string()], 1));
        assert!(task_wants_help(&["-h".to_string()], 2));
        assert!(task_wants_help(&["help".to_string()], 2));
        assert!(task_wants_help(&[], 1));
        assert!(task_wants_help(&["3".to_string()], 2));

        // A real subject, or a full id/status pair, still acts instead.
        assert!(!task_wants_help(&["Record the login bug".to_string()], 1));
        assert!(!task_wants_help(
            &["2".to_string(), "completed".to_string()],
            2
        ));
        // A payload behind a help token belongs to the subject — this is
        // the swallow the previous leading-token check lost silently.
        assert!(!task_wants_help(
            &["help".to_string(), "words".to_string()],
            1
        ));
        assert!(!task_wants_help(
            &["--help".to_string(), "me".to_string()],
            1
        ));
    }
}
