//! The CLI end of the agent-tasks protocol, pinned through the compiled
//! binary: `$TMUX_PANE` dispatch and the exact stdout payloads are
//! child-process behavior the in-crate unit tests cannot observe.

use std::fs;
use tmux_agent_sidebar::task_store;

const BIN: &str = env!("CARGO_BIN_EXE_tmux-agent-sidebar");

#[test]
fn task_commands_round_trip_through_the_binary() {
    let pane = "%CLI_BIN_ROUNDTRIP";
    let path = task_store::store_file_path(pane);
    let _ = fs::remove_file(&path);

    // An unquoted multi-word subject joins into free text — `task-set
    // Fix the login crash` must store the whole words, never just the
    // first — and the id it prints is the one stored for a later
    // task-update.
    let set = std::process::Command::new(BIN)
        .args(["task-set", "Fix the login crash"])
        .env("TMUX_PANE", pane)
        .output()
        .expect("compiled binary must run");
    assert!(set.status.success());
    assert_eq!(String::from_utf8_lossy(&set.stdout), "1\n");

    let items = task_store::read_tasks(pane);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "1");
    assert_eq!(items[0].subject, "Fix the login crash");
    assert_eq!(items[0].status, task_store::TaskItemStatus::Pending);

    let update = std::process::Command::new(BIN)
        .args(["task-update", "1", "completed"])
        .env("TMUX_PANE", pane)
        .output()
        .expect("compiled binary must run");
    assert!(update.status.success());
    assert_eq!(String::from_utf8_lossy(&update.stdout), "1 completed\n");

    let items = task_store::read_tasks(pane);
    assert_eq!(items[0].status, task_store::TaskItemStatus::Completed);

    let _ = fs::remove_file(&path);
}
