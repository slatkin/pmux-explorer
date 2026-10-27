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
        "task-describe" => cmd_task_describe(rest),
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
    // SAFETY: `isatty` reads the descriptor only and writes nothing; it
    // accepts any fd value and reports an unusable one as 0.
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
    // SAFETY: `time` writes only through its pointer argument; a null
    // pointer asks for the current time and the return value carries it.
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    // SAFETY: `zeroed` is the valid initial state for `libc::tm` — all
    // integer fields zero and `tm_zone` a null pointer, valid for every
    // field's type — and every field `localtime_r` does not write stays
    // zero. Nothing below dereferences `tm_zone`.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: `localtime_r` reads `now` and writes only `tm`; both are valid
    // for the call. It reports failure by returning null, checked here.
    if unsafe { libc::localtime_r(&now, &mut tm) }.is_null() {
        // The zeroed `tm` would otherwise format as a plausible "00:00" and
        // hide the failure; this sentinel keeps it greppable in the log.
        return "??:??".to_string();
    }
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
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
/// The command takes no arguments — arguments an older prompt may still
/// pass (`--nonce <n>`) are ignored. The answer window is the endpoint's
/// own read timeout (2 seconds), so a server that never replies fails at
/// the same pace the socket rules set.
fn cmd_channel_ready(_args: &[String]) -> i32 {
    let env = mcp::EndpointEnv::from_process();
    let (stdout, stderr, code) = run_channel_ready(env, mcp::DEFAULT_READ_TIMEOUT);
    if let Some(line) = stdout {
        println!("{line}");
    }
    if let Some(line) = stderr {
        eprintln!("{line}");
    }
    code
}

/// Resolves one `channel-ready` invocation into the line stdout prints
/// (the endpoint's reply), the line stderr prints (a failure message), and
/// the exit code, without printing — so the tests run it against a real
/// socket in a temp dir without touching the process env.
fn run_channel_ready(
    env: mcp::EndpointEnv<'_>,
    answer_timeout: Duration,
) -> (Option<String>, Option<String>, i32) {
    let Some(path) = mcp::endpoint_path(env) else {
        return (
            None,
            Some(
                "channel-ready: TMUX and TMUX_PANE must name the pane with the endpoint"
                    .to_string(),
            ),
            1,
        );
    };
    match mcp::send_arm(&path, answer_timeout) {
        Ok(reply) => {
            let parsed = serde_json::from_str::<serde_json::Value>(&reply)
                .ok()
                .unwrap_or(serde_json::Value::Null);
            let armed = parsed
                .get(mcp::reply::ARMED)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if armed {
                (Some(reply), None, 0)
            } else {
                // A reply that is not armed is a failure path: exit 1
                // with the reason, stdout reply still printed.
                let reason = parsed
                    .get(mcp::reply::REASON)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown reply");
                (
                    Some(reply),
                    Some(format!(
                        "channel-ready: the endpoint at {} did not arm (reason: {reason})",
                        path.display(),
                    )),
                    1,
                )
            }
        }
        Err(ArmError::Unreachable(error)) => (
            None,
            Some(format!(
                "channel-ready: no endpoint at {} ({error})",
                path.display(),
            )),
            1,
        ),
        Err(ArmError::NoAnswer) => (
            None,
            Some(format!(
                "channel-ready: the endpoint at {} did not answer within {answer_timeout:?}",
                path.display(),
            )),
            1,
        ),
    }
}

// ─── set-status subcommand ──────────────────────────────────────────────────────

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
         task-describe <text>           set the pane's task list description\n\
         \n\
         reminders:\n\
         a turn starting with an empty todo list gets: {}\n\
         a turn ending with open items gets: {}",
        task_reminders::RECORD_FIRST_REMINDER,
        task_reminders::UPDATE_BEFORE_FINISH_REMINDER,
    )
}

/// Record a pending task item for the calling pane. Exits 0 on every
/// path that changes nothing — including a missing `$TMUX_PANE` — so bad
/// arguments never break the agent's turn, same contract as the hook
/// command. On success the assigned item id is printed so the agent can
/// name it in a later `task-update`; a store that could not be written
/// exits 1 with the io error on stderr, so the id is never reported for
/// an item that was not persisted.
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
    match crate::task_store::record_task(pane, subject) {
        Ok(Some(id)) => {
            println!("{id}");
            0
        }
        Ok(None) => 0,
        Err(error) => store_write_error("task-set", &error),
    }
}

/// Map a failed task-store write onto the command's exit code: report the
/// io error on stderr and exit 1, the way `channel-ready` reports its own
/// failures. A write that never landed must not read as success.
fn store_write_error(command: &str, error: &std::io::Error) -> i32 {
    eprintln!("{command}: the task store was not written ({error})");
    1
}

/// Set the pane-wide description shown above the Tasks tab's item
/// list. Same join-the-args and exit contract as `task-set`: the
/// agent's turn is never broken by bad arguments, and a description
/// that could not be stored exits 1 with the io error on stderr.
fn cmd_task_describe(args: &[String]) -> i32 {
    if task_wants_help(args, 1) {
        println!("{}", task_help_text());
        return 0;
    }
    let description = args.join(" ");
    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }
    task_describe_for_pane(&pane, &description)
}

fn task_describe_for_pane(pane: &str, description: &str) -> i32 {
    match crate::task_store::set_description(pane, description) {
        Ok(_) => 0,
        Err(error) => store_write_error("task-describe", &error),
    }
}

/// Update one task item's status for the calling pane. Exits 0 when
/// nothing changed — an unknown id or status name changes nothing — and
/// exits 1 with the io error on stderr when the store could not be
/// written.
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
    let Some(status) = crate::task_store::parse_task_status(status) else {
        return 0;
    };
    match crate::task_store::update_task_status(pane, id, status) {
        Ok(true) => {
            println!("{} {}", id, status.as_str());
            0
        }
        Ok(false) => 0,
        Err(error) => store_write_error("task-update", &error),
    }
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

    /// The tmux env values and uid the test endpoint is derived from.
    const TEST_TMUX: &str = "/tmp/tmux-1000/default,1,0";
    const TEST_PANE: &str = "%12";

    /// The tmux environment the served endpoint derives from, with owned
    /// values so each test holds one struct and borrows `EndpointEnv`
    /// from it — no positional triple to misread.
    struct ServedEnv {
        tmux: String,
        pane: String,
        xdg: String,
    }

    impl ServedEnv {
        fn env(&self) -> mcp::EndpointEnv<'_> {
            mcp::EndpointEnv {
                tmux: Some(&self.tmux),
                tmux_pane: Some(&self.pane),
                xdg_runtime_dir: Some(&self.xdg),
                uid: crate::mcp::current_uid(),
            }
        }
    }

    /// Serves an unarmed endpoint state at the endpoint the env derives
    /// to inside `dir`, and returns the env and the shared state.
    fn served_endpoint(dir: &std::path::Path) -> (ServedEnv, Arc<EndpointState>) {
        let served = ServedEnv {
            tmux: TEST_TMUX.to_string(),
            pane: TEST_PANE.to_string(),
            xdg: dir.to_string_lossy().to_string(),
        };
        let path = crate::mcp::endpoint_path(served.env()).unwrap();
        let state = crate::test_support::serve_unarmed_endpoint(
            &path,
            TEST_PANE,
            mcp::DEFAULT_READ_TIMEOUT,
        );
        (served, state)
    }

    // The "Channel-ready command" scenarios, against a socket in a temp
    // dir.

    #[test]
    fn an_arm_request_answers_armed_and_exits_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let (served, _) = served_endpoint(tmp.path());
        let (stdout, stderr, code) = run_channel_ready(served.env(), mcp::DEFAULT_READ_TIMEOUT);
        assert_eq!(code, 0);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stdout.unwrap()).unwrap(),
            json!({"ok": true, "armed": true}),
        );
        assert_eq!(stderr, None);
    }

    #[test]
    fn a_not_armed_reply_prints_and_exits_one() {
        // A listener that answers the request line with a refusal, so the
        // not-armed failure path is pinned without a real server state.
        let tmp = tempfile::tempdir().unwrap();
        let served = ServedEnv {
            tmux: TEST_TMUX.to_string(),
            pane: TEST_PANE.to_string(),
            xdg: tmp.path().to_string_lossy().to_string(),
        };
        let path = crate::mcp::endpoint_path(served.env()).unwrap();
        let listener = crate::mcp::bind(&path, crate::mcp::current_uid()).unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            let _ =
                std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut stream), &mut line);
            use std::io::Write as _;
            let _ = writeln!(stream, "{{\"ok\":false,\"reason\":\"nonce\"}}");
        });
        let (stdout, stderr, code) = run_channel_ready(served.env(), mcp::DEFAULT_READ_TIMEOUT);
        assert_eq!(code, 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stdout.unwrap()).unwrap(),
            json!({"ok": false, "reason": "nonce"}),
        );
        // The refusal is a failure path, so stderr carries the reason
        // alongside the stdout reply print.
        let message = stderr.unwrap();
        assert!(message.contains("did not arm"), "message: {message}");
        assert!(message.contains("nonce"), "message: {message}");
        assert!(message.contains(&served.xdg), "message: {message}");
    }

    #[test]
    fn a_missing_endpoint_exits_one_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        // The env names a pane whose socket was never created.
        let served = ServedEnv {
            tmux: TEST_TMUX.to_string(),
            pane: TEST_PANE.to_string(),
            xdg: tmp.path().to_string_lossy().to_string(),
        };
        let (stdout, stderr, code) = run_channel_ready(served.env(), mcp::DEFAULT_READ_TIMEOUT);
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("no endpoint"), "message: {message}");
        let expected = crate::mcp::endpoint_path(mcp::EndpointEnv {
            tmux: Some(TEST_TMUX),
            tmux_pane: Some(TEST_PANE),
            xdg_runtime_dir: Some(tmp.path().to_string_lossy().as_ref()),
            uid: crate::mcp::current_uid(),
        })
        .unwrap();
        assert!(message.contains(&expected.to_string_lossy().to_string()));
    }

    #[test]
    fn an_endpoint_that_never_answers_exits_one_naming_the_path() {
        let tmp = tempfile::tempdir().unwrap();
        // A raw listener that accepts and answers nothing, so the client's
        // short answer window ends the wait instead of the real 2 seconds.
        let served = ServedEnv {
            tmux: TEST_TMUX.to_string(),
            pane: TEST_PANE.to_string(),
            xdg: tmp.path().to_string_lossy().to_string(),
        };
        let path = crate::mcp::endpoint_path(served.env()).unwrap();
        let listener = crate::mcp::bind(&path, crate::mcp::current_uid()).unwrap();
        thread::spawn(move || {
            let _ = listener.accept();
            thread::sleep(Duration::from_secs(2));
        });
        let (stdout, stderr, code) = run_channel_ready(served.env(), Duration::from_millis(150));
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("did not answer"), "message: {message}");
        assert!(message.contains(&path.to_string_lossy().to_string()));
    }

    #[test]
    fn channel_ready_without_tmux_env_names_the_env_and_sends_no_request() {
        let tmp = tempfile::tempdir().unwrap();
        let (served, state) = served_endpoint(tmp.path());
        // No TMUX and no TMUX_PANE: no endpoint path is derivable.
        let xdg = tmp.path().to_string_lossy().to_string();
        let env = mcp::EndpointEnv {
            tmux: None,
            tmux_pane: None,
            xdg_runtime_dir: Some(&xdg),
            uid: crate::mcp::current_uid(),
        };
        let (stdout, stderr, code) = run_channel_ready(env, mcp::DEFAULT_READ_TIMEOUT);
        let _ = served;
        assert_eq!(code, 1);
        assert_eq!(stdout, None);
        let message = stderr.unwrap();
        assert!(message.contains("TMUX"), "message: {message}");
        // No request reached the endpoint, so it never armed.
        assert!(!state.armed.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn the_channel_ready_dispatch_maps_to_run() {
        assert!(run(&["channel-ready".to_string()]).is_some());
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
        assert_eq!(task_describe_for_pane("", "orphan description"), 0);

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
        assert_eq!(run(&["task-describe".into()]), Some(0));
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
    fn a_store_write_that_fails_exits_one_instead_of_claiming_success() {
        // A directory in the store's place makes every write fail, the
        // one way a unit test can force the io error the CLI boundary
        // must now report: the command exits 1 rather than printing an
        // id for an item that was never persisted.
        let pane = "%CLI_TASK_WRITE_FAIL";
        let path = crate::task_store::store_file_path(pane);
        let _ = std::fs::remove_file(&path);
        std::fs::create_dir_all(&path).unwrap();

        assert_eq!(task_set_for_pane(pane, "unwritable"), 1);
        assert_eq!(task_describe_for_pane(pane, "unwritable"), 1);

        std::fs::remove_dir_all(&path).unwrap();
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
