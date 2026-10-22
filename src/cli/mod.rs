pub mod capture;
mod hook;
mod label;
pub mod plugin_state;
pub(crate) mod setup;
pub(crate) mod shared_html;
mod spawn;
pub(crate) mod toggle;

use std::io::Read;

use crate::task_reminders;
use crate::tmux;

/// Run a CLI subcommand. Returns Some(exit_code) if a subcommand was matched,
/// None if no subcommand was given (caller should launch TUI).
pub fn run(args: &[String]) -> Option<i32> {
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

/// Decide whether a task command invocation should print help instead of
/// acting: an explicit help flag, or too few args for the command to do
/// anything (`task-set` needs a subject, `task-update` needs an id and
/// a status). The arity case previously exited 0 silently; printing the
/// usage plus the shared reminder wording is the discoverable path.
fn task_wants_help(args: &[String], min_args: usize) -> bool {
    matches!(
        args.first().map(String::as_str),
        Some("--help") | Some("-h") | Some("help")
    ) || args.len() < min_args
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
    let subject = args[0].as_str();
    let pane = tmux_pane();
    if pane.is_empty() {
        return 0;
    }
    task_set_for_pane(&pane, subject)
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
    use serde_json::json;

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

    // ─── task-set / task-update tests ───────────────────────────────

    #[test]
    fn task_set_records_pending_item_for_calling_pane() {
        let pane = "%CLI_TASK_SET";
        let path = crate::task_store::store_file_path(pane);
        let _ = std::fs::remove_file(&path);

        let code = task_set_for_pane(pane, "Fix the login bug");

        assert_eq!(code, 0);
        let items = crate::task_store::read_tasks(pane);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].subject, "Fix the login bug");
        assert_eq!(items[0].status, crate::task_store::TaskItemStatus::Pending);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn task_set_without_pane_env_exits_zero_and_writes_nothing() {
        // A missing $TMUX_PANE reads as an empty pane id; the store
        // guards make it a no-op while still exiting successfully.
        let before = crate::task_store::store_mtime("");

        let code = task_set_for_pane("", "orphan subject");

        assert_eq!(code, 0);
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
    fn task_update_changes_status_for_calling_pane() {
        let pane = "%CLI_TASK_UPDATE";
        let path = crate::task_store::store_file_path(pane);
        let _ = std::fs::remove_file(&path);
        crate::task_store::record_task(pane, "Ship it").unwrap();

        let code = task_update_for_pane(pane, "1", "completed");

        assert_eq!(code, 0);
        let items = crate::task_store::read_tasks(pane);
        assert_eq!(
            items[0].status,
            crate::task_store::TaskItemStatus::Completed
        );

        let _ = std::fs::remove_file(&path);
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
        // A help flag beats any arity, and an arity that cannot act
        // (no subject, or an id without a status) prints help too.
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
    }
}
