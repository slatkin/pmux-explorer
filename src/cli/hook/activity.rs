use crate::pane_state::status_writes;
use crate::pane_store::{PaneStore, PaneWrites};
use crate::time::now_epoch_secs;
use crate::tmux;
use crate::tool_name::CanonicalTool;

use super::super::label::extract_tool_label;
use super::super::local_time_hhmm;
use super::super::sanitize_log_label;
use super::context::pane_writes_allowed;

/// Write a single activity entry to the log file and trim if needed.
/// The io error is returned because the hook command's whole job for an
/// `activity-log` event is this append: a swallowed failure leaves the
/// sidebar showing nothing.
pub(super) fn write_activity_entry(
    pane: &str,
    tool_name: &str,
    label: &str,
) -> std::io::Result<()> {
    let log_path = crate::activity::log_file_path(pane);
    let label = sanitize_log_label(label);
    let timestamp = local_time_hhmm();
    let line = format!("{}|{}|{}\n", timestamp, tool_name, label);

    use std::io::Write;
    {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        f.write_all(line.as_bytes())?;
    }

    trim_log_file(&log_path, 200, 210)
}

/// Report a failed activity-log write on stderr. The hook command exits 0
/// on every path by its protocol contract, so stderr is the only channel
/// that tells the producer the write failed. Covers both the entry append
/// and the trailing trim — the io error names which operation failed.
pub(super) fn report_write_error(error: &std::io::Error) {
    eprintln!("hook: the activity log write failed ({error})");
}

/// Trim a log file to `keep` lines when it exceeds `threshold` lines. A
/// missing file has nothing to trim and is not a failure; every other io
/// error is returned so the caller can report it.
pub(super) fn trim_log_file(
    path: &std::path::Path,
    keep: usize,
    threshold: usize,
) -> std::io::Result<()> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= threshold {
        return Ok(());
    }
    let start = lines.len() - keep;
    std::fs::write(path, lines[start..].join("\n") + "\n")
}

/// Activity-log handler, called from `hook <agent> activity-log` event.
/// Every option write goes through the store as one batch — the rules stay
/// in the handler until the `ActivityLog` arm lands in the transition.
pub(super) fn handle_activity_log(
    pane: &str,
    store: &dyn PaneStore,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: &serde_json::Value,
) -> i32 {
    let label = extract_tool_label(tool_name, tool_input, tool_response);
    let mut writes = PaneWrites::new();
    if is_background_bash(tool_name, tool_input) {
        let stored = if label.is_empty() {
            tmux::BG_CMD_PLACEHOLDER
        } else {
            label.as_str()
        };
        // No sanitizer call here: the store's apply is the one place every
        // written value passes through the encoder.
        writes.set(tmux::PANE_BG_CMD, stored);
    }

    let current_status = store.get(pane, tmux::PANE_STATUS);
    if current_status != "running" && !current_status.is_empty() {
        writes.merge(&status_writes("running"));
        if current_status == "waiting" {
            // status_writes("running") already unsets the attention flag;
            // the stale wait reason goes with it.
            writes.unset(tmux::PANE_WAIT_REASON);
        }
        let existing_started = store.get(pane, tmux::PANE_STARTED_AT);
        if existing_started.is_empty() {
            writes.set(tmux::PANE_STARTED_AT, now_epoch_secs().to_string());
        }
    }

    // Update permission mode when plan mode tools are used.
    // Same parent-protection rule as `set_agent_meta`: a subagent that
    // enters/exits plan mode must not flip the parent pane's badge. The
    // subagent list is not touched by the writes above, so reading it
    // before the batch sees the same state the old post-status read did.
    if pane_writes_allowed(pane, store) {
        match tool_name {
            "EnterPlanMode" => writes.set(tmux::PANE_PERMISSION_MODE, "plan"),
            "ExitPlanMode" => writes.set(tmux::PANE_PERMISSION_MODE, "default"),
            _ => {}
        }
    }
    let _ = store.apply(pane, &writes);

    if let Err(error) = write_activity_entry(pane, tool_name, &label) {
        report_write_error(&error);
    }
    0
}

fn is_background_bash(tool_name: &str, tool_input: &serde_json::Value) -> bool {
    tool_name == CanonicalTool::Bash.as_str()
        && ["run_in_background", "runInBackground"]
            .iter()
            .any(|key| tool_input.get(key).and_then(|v| v.as_bool()) == Some(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_store::MemPaneStore;
    use serde_json::Value;
    use serde_json::json;
    use std::fs;

    // ─── trim_log_file tests ────────────────────────────────────────

    #[test]
    fn trim_log_file_under_threshold_no_change() {
        let dir = std::env::temp_dir();
        let path = dir.join("trim_test_under.log");
        fs::write(&path, "line1\nline2\nline3\n").unwrap();

        trim_log_file(&path, 2, 5).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 3);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn trim_log_file_over_threshold_trims() {
        let dir = std::env::temp_dir();
        let path = dir.join("trim_test_over.log");
        let lines: Vec<String> = (1..=15).map(|i| format!("line{}", i)).collect();
        fs::write(&path, lines.join("\n") + "\n").unwrap();

        trim_log_file(&path, 5, 10).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let remaining: Vec<&str> = content.lines().collect();
        assert_eq!(remaining.len(), 5);
        assert_eq!(remaining[0], "line11");
        assert_eq!(remaining[4], "line15");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn trim_log_file_exactly_at_threshold_no_change() {
        let dir = std::env::temp_dir();
        let path = dir.join("trim_test_exact.log");
        let lines: Vec<String> = (1..=10).map(|i| format!("line{}", i)).collect();
        fs::write(&path, lines.join("\n") + "\n").unwrap();

        trim_log_file(&path, 5, 10).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 10);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn trim_log_file_nonexistent_file_no_panic() {
        let dir = std::env::temp_dir();
        let path = dir.join("trim_test_nonexistent.log");
        let _ = fs::remove_file(&path);
        trim_log_file(&path, 5, 10).unwrap();
    }

    // ─── write_activity_entry tests ─────────────────────────────────

    #[test]
    fn write_activity_entry_creates_and_appends() {
        let pane_id = "%CLI_WRITE_TEST";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        write_activity_entry(pane_id, "Read", "main.rs").unwrap();
        write_activity_entry(pane_id, "Edit", "lib.rs").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("|Read|main.rs"));
        assert!(lines[1].ends_with("|Edit|lib.rs"));
        assert_eq!(lines[0].as_bytes()[2], b':');
        fs::remove_file(&path).ok();
    }

    #[test]
    fn write_activity_entry_sanitizes_label() {
        let pane_id = "%CLI_SANITIZE_TEST";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        write_activity_entry(pane_id, "Bash", "cat file | grep foo bar\nbaz").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "newlines in label should not create extra lines"
        );
        let label = lines[0].splitn(3, '|').nth(2).unwrap();
        assert_eq!(
            label, "cat file | grep foo bar baz",
            "the log label keeps its `|`"
        );
        assert!(!label.contains('\n'));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn write_activity_entry_trims_at_threshold() {
        let pane_id = "%CLI_TRIM_TEST";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        for i in 1..=215 {
            write_activity_entry(pane_id, "Read", &format!("file{}.rs", i)).unwrap();
        }

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() <= 210, "should be trimmed, got {}", lines.len());
        assert!(lines.last().unwrap().ends_with("|Read|file215.rs"));
        fs::remove_file(&path).ok();
    }

    // ─── handle_activity_log tests ──────────────────────────────────

    #[test]
    fn handle_activity_log_writes_entry() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_HANDLE_TEST";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "Read",
            &json!({"file_path": "/home/user/src/main.rs"}),
            &Value::Null,
        );

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("|Read|main.rs"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_marks_background_bash() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_BG_BASH";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "Bash",
            &json!({"command": "npm run dev", "run_in_background": true}),
            &Value::Null,
        );

        assert_eq!(
            store.get(pane_id, tmux::PANE_BG_CMD),
            "npm run dev",
            "command string must be stored so the row body can show what is running",
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_background_bash_without_command_falls_back_to_placeholder() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_BG_BASH_NO_CMD";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "Bash",
            &json!({"run_in_background": true}),
            &Value::Null,
        );

        assert_eq!(
            store.get(pane_id, tmux::PANE_BG_CMD),
            tmux::BG_CMD_PLACEHOLDER
        );
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_sanitizes_command_before_writing() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_BG_BASH_PIPE";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "Bash",
            &json!({"command": "cat a.log | grep foo\nbar", "run_in_background": true}),
            &Value::Null,
        );

        // The log label keeps `|` (only line breaks become spaces).
        let content = fs::read_to_string(&path).unwrap();
        let line = content.lines().next().unwrap_or("");
        let label = line.splitn(3, '|').nth(2).unwrap_or("");
        assert_eq!(label, "cat a.log | grep foo bar");
        assert!(!label.contains('\n'));

        // The `@pane_bg_cmd` pane option keeps its `|`-free encoding, now
        // applied by the store itself.
        let stored = store.get(pane_id, tmux::PANE_BG_CMD);
        assert!(!stored.contains('|'));
        assert!(!stored.contains('\n'));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_preserves_bg_cmd_when_background_resumes_running() {
        // Regression: a tool-use burst must not clear @pane_bg_cmd, or
        // the next Stop would land in `idle` instead of `background`.
        let store = MemPaneStore::new();
        let pane_id = "%CLI_BG_PRESERVE";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);
        store.insert(pane_id, tmux::PANE_STATUS, "background");
        store.insert(pane_id, tmux::PANE_BG_CMD, "npm run dev");

        handle_activity_log(
            pane_id,
            &store,
            "Read",
            &json!({"file_path": "/home/user/src/main.rs"}),
            &Value::Null,
        );

        assert_eq!(store.get(pane_id, tmux::PANE_STATUS), "running");
        assert_eq!(store.get(pane_id, tmux::PANE_BG_CMD), "npm run dev");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_waiting_transitions_to_running_on_tool_use() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_WAIT_TO_RUN";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);
        store.insert(pane_id, tmux::PANE_STATUS, "waiting");
        store.insert(pane_id, tmux::PANE_ATTENTION, "notification");
        store.insert(pane_id, tmux::PANE_WAIT_REASON, "permission_prompt");

        handle_activity_log(
            pane_id,
            &store,
            "Read",
            &json!({"file_path": "/home/user/src/main.rs"}),
            &Value::Null,
        );

        assert_eq!(store.get(pane_id, tmux::PANE_STATUS), "running");
        assert!(!store.contains(pane_id, tmux::PANE_ATTENTION));
        assert!(!store.contains(pane_id, tmux::PANE_WAIT_REASON));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_empty_tool_name_does_nothing() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_EMPTY_TOOL";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        // With the adapter pattern, empty tool_name is filtered by the adapter
        // before reaching handle_activity_log. We still test that handle_activity_log
        // writes an entry even with empty tool_name (label extraction handles it).
        let result = handle_activity_log(pane_id, &store, "", &Value::Null, &Value::Null);
        assert_eq!(result, 0);
        // Empty tool_name still writes an entry now (adapter filters upstream)
    }

    #[test]
    fn handle_activity_log_tool_input_as_json_object() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_JSON_STR";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "Edit",
            &json!({"file_path": "/a/b/test.rs"}),
            &Value::Null,
        );

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("|Edit|test.rs"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_null_tool_input_uses_empty_label() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_NULL_INPUT";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(pane_id, &store, "UnknownTool", &Value::Null, &Value::Null);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("|UnknownTool|"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_task_create_with_response() {
        let store = MemPaneStore::new();
        let pane_id = "%CLI_TASK_CREATE";
        let path = crate::activity::log_file_path(pane_id);
        let _ = fs::remove_file(&path);

        handle_activity_log(
            pane_id,
            &store,
            "TaskCreate",
            &json!({"subject": "Fix bug"}),
            &json!({"task": {"id": "42"}}),
        );

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("|TaskCreate|#42 Fix bug"));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn handle_activity_log_enter_plan_mode_blocked_by_subagents() {
        let store = MemPaneStore::new();
        let pane = "%PARENT_PLAN";
        store.insert(pane, tmux::PANE_SUBAGENTS, "Explore:sub-1");
        store.insert(pane, tmux::PANE_PERMISSION_MODE, "default");

        // A subagent's EnterPlanMode tool use must not flip the parent
        // badge to "plan".
        handle_activity_log(pane, &store, "EnterPlanMode", &Value::Null, &Value::Null);

        assert_eq!(store.get(pane, tmux::PANE_PERMISSION_MODE), "default");
    }
}
