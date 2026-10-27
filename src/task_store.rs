//! Per-pane task store backing the bottom panel's Tasks tab.
//!
//! One JSON file per tmux pane at `/tmp/pmux-explorer-tasks<PANE>.json`,
//! mirroring the activity log's path encoding, mtime polling, and
//! session semantics: the agent writes items and statuses through the
//! `task-set` / `task-update` / `task-describe` CLI subcommands; the
//! sidebar's one write is emptying a finished list — once every item
//! is `completed` and the store has been quiet for [`TASK_CLOSE_SECS`]
//! seconds, the per-tick close sweep removes the file.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Maximum number of items kept in one pane's store. When a record
/// pushes past the cap, completed items are trimmed oldest-first; if
/// none are completed, the oldest item goes so the cap still holds.
pub const MAX_ITEMS: usize = 50;

/// Seconds an all-completed, untouched task list stays open before the
/// sidebar's close sweep empties it. Every store write bumps the file
/// mtime, so an added or reopened item restarts the window.
pub const TASK_CLOSE_SECS: u64 = 90;

/// Status of a single stored task item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskItemStatus {
    Pending,
    InProgress,
    Completed,
}

impl TaskItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

/// Parse a status name as accepted from the CLI. Unrecognized names
/// yield `None` so callers can no-op instead of guessing.
pub fn parse_task_status(s: &str) -> Option<TaskItemStatus> {
    match s {
        "pending" => Some(TaskItemStatus::Pending),
        "in_progress" => Some(TaskItemStatus::InProgress),
        "completed" => Some(TaskItemStatus::Completed),
        _ => None,
    }
}

/// One todo item in a pane's store. The `id` is store-local and stays
/// stable across trims, so `task-update` can name an item after older
/// ones were dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub subject: String,
    pub status: TaskItemStatus,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    /// Pane-wide description of the work the list covers, written by
    /// `task-describe`. `#[serde(default)]` keeps stores written before
    /// the field existed readable as an empty description.
    #[serde(default)]
    description: String,
    tasks: Vec<TaskItem>,
}

/// The pane's store as the Tasks tab consumes it: the pane-wide
/// description plus the item list.
#[derive(Debug, Default)]
pub struct TaskList {
    pub description: String,
    pub items: Vec<TaskItem>,
}

/// Path of a pane's task store. Shares [`crate::activity::encode_pane_id_for_file_name`]
/// with the activity log's path (`%` → `_`) because tmux pane ids
/// contain a `%` that is awkward in file names.
pub fn store_file_path(pane_id: &str) -> PathBuf {
    PathBuf::from(format!(
        "/tmp/pmux-explorer-tasks{}.json",
        crate::activity::encode_pane_id_for_file_name(pane_id)
    ))
}

/// Last-modified time of a pane's task store, or `None` when the file
/// does not exist or its metadata cannot be queried. Refresh paths use
/// this as the cheap "anything to re-read?" check, exactly like the
/// activity log's mtime polling.
pub fn store_mtime(pane_id: &str) -> Option<SystemTime> {
    crate::activity::file_mtime(&store_file_path(pane_id))
}

/// Read a pane's stored items in recorded order. A missing or corrupt
/// store reads as empty, like the activity log reader.
pub fn read_tasks(pane_id: &str) -> Vec<TaskItem> {
    read_task_list(pane_id).items
}

/// Read a pane's whole store — description plus items — in recorded
/// order. A missing or corrupt store reads as empty, like the
/// activity log reader.
pub fn read_task_list(pane_id: &str) -> TaskList {
    if pane_id.is_empty() {
        return TaskList::default();
    }
    let Ok(content) = fs::read_to_string(store_file_path(pane_id)) else {
        return TaskList::default();
    };
    serde_json::from_str::<StoreFile>(&content)
        .map(|file| TaskList {
            description: file.description,
            items: file.tasks,
        })
        .unwrap_or_default()
}

/// Record a new pending item for a pane and return its assigned id.
/// A missing pane id or a blank subject is a no-op returning `None`, and
/// a store that could not be written returns the io error instead of an
/// id — the id is only reported once it reached the file.
pub fn record_task(pane_id: &str, subject: &str) -> io::Result<Option<String>> {
    if pane_id.is_empty() || subject.trim().is_empty() {
        return Ok(None);
    }
    let mut list = read_task_list(pane_id);
    // Ids keep counting past trimmed items so a stored id is never reused.
    let next = list
        .items
        .iter()
        .filter_map(|task| task.id.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let id = next.to_string();
    list.items.push(TaskItem {
        id: id.clone(),
        subject: subject.to_string(),
        status: TaskItemStatus::Pending,
    });
    trim_items(&mut list.items);
    write_store(pane_id, &list)?;
    Ok(Some(id))
}

/// Set the status of the item with `id`. Returns `false` when the pane
/// id is empty or the item does not exist, so callers can no-op, and
/// the io error when the change could not be persisted.
pub fn update_task_status(pane_id: &str, id: &str, status: TaskItemStatus) -> io::Result<bool> {
    if pane_id.is_empty() {
        return Ok(false);
    }
    let mut list = read_task_list(pane_id);
    let Some(item) = list.items.iter_mut().find(|task| task.id == id) else {
        return Ok(false);
    };
    item.status = status;
    write_store(pane_id, &list)?;
    Ok(true)
}

/// Set the pane-wide description shown above the Tasks tab's item
/// list. Later writes replace it; a blank description clears it.
/// Returns `false` when the pane id is empty, so callers can no-op, and
/// the io error when the description could not be persisted.
pub fn set_description(pane_id: &str, description: &str) -> io::Result<bool> {
    if pane_id.is_empty() {
        return Ok(false);
    }
    let mut list = read_task_list(pane_id);
    list.description = description.trim().to_string();
    write_store(pane_id, &list)?;
    Ok(true)
}

/// Whether a pane's task list is due to close: the list holds at
/// least one item, every item is `completed`, and the store mtime is
/// at least [`TASK_CLOSE_SECS`] old. `mtime_epoch` is the store file's
/// mtime in epoch seconds — `None` when there is no store to close —
/// and `now_epoch` matches `AppState::now`.
pub fn close_due(list: &TaskList, mtime_epoch: Option<u64>, now_epoch: u64) -> bool {
    every_item_completed(list) && close_window_elapsed(mtime_epoch, now_epoch)
}

/// The item half of [`close_due`]: the list holds at least one item
/// and every item is `completed`. The close sweep caches this per
/// pane so an unchanged store is not re-read every tick.
pub(crate) fn every_item_completed(list: &TaskList) -> bool {
    !list.items.is_empty()
        && list
            .items
            .iter()
            .all(|item| item.status == TaskItemStatus::Completed)
}

/// The time half of [`close_due`]: the store mtime sits at least
/// [`TASK_CLOSE_SECS`] behind `now_epoch`. A `None` or future mtime
/// reads as not due — a missing store has nothing to close, and a
/// clock jump closes late, never early.
pub(crate) fn close_window_elapsed(mtime_epoch: Option<u64>, now_epoch: u64) -> bool {
    mtime_epoch.is_some_and(|mtime| now_epoch.saturating_sub(mtime) >= TASK_CLOSE_SECS)
}

/// Remove a pane's store. Used at SessionStart so a pane reused for
/// new work starts with an empty list, matching the activity log's
/// session semantics, and by the per-tick close sweep to empty a
/// finished list. A missing store is already empty.
pub fn clear_tasks(pane_id: &str) {
    if pane_id.is_empty() {
        return;
    }
    // Best effort: a store that survives the removal is overwritten by the
    // next write, so a teardown or close sweep must not fail on it.
    let _ = fs::remove_file(store_file_path(pane_id));
}

/// Write a pane's store back as JSON. A `StoreFile` of strings and
/// unit-variant enums always serializes, so the only expected failure is
/// the io one — surfaced to the caller because the task commands report
/// success only when the write landed.
fn write_store(pane_id: &str, list: &TaskList) -> io::Result<()> {
    let file = StoreFile {
        description: list.description.clone(),
        tasks: list.items.clone(),
    };
    let json = serde_json::to_string(&file).map_err(io::Error::other)?;
    fs::write(store_file_path(pane_id), json)
}

/// Enforce [`MAX_ITEMS`]: drop the oldest completed items first, then
/// the oldest items regardless of status, mirroring the activity log's
/// keep-the-newest trim behavior.
fn trim_items(items: &mut Vec<TaskItem>) {
    while items.len() > MAX_ITEMS {
        let drop = items
            .iter()
            .position(|task| task.status == TaskItemStatus::Completed)
            .unwrap_or(0);
        items.remove(drop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn cleanup(pane_id: &str) {
        let _ = fs::remove_file(store_file_path(pane_id));
    }

    #[test]
    fn store_file_path_encodes_percent_like_activity_log() {
        let path = store_file_path("%5");
        assert_eq!(path.to_str().unwrap(), "/tmp/pmux-explorer-tasks_5.json");
    }

    #[test]
    fn record_assigns_sequential_ids_with_pending_status() {
        let pane_id = "%TASK_RECORD";
        cleanup(pane_id);

        let first = record_task(pane_id, "Fix the login bug").unwrap().unwrap();
        let second = record_task(pane_id, "Write the docs").unwrap().unwrap();

        let items = read_tasks(pane_id);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, first);
        assert_eq!(items[0].subject, "Fix the login bug");
        assert_eq!(items[0].status, TaskItemStatus::Pending);
        assert_eq!(items[1].id, second);
        assert_eq!(second, "2", "ids count up within one store");

        cleanup(pane_id);
    }

    #[test]
    fn record_missing_pane_env_is_noop() {
        // Before: whatever state the pane-less store file is in.
        let before = store_mtime("");

        assert_eq!(record_task("", "orphan subject").unwrap(), None);

        assert_eq!(
            store_mtime(""),
            before,
            "a missing pane id must not touch any store file"
        );
    }

    #[test]
    fn record_blank_subject_is_noop() {
        let pane_id = "%TASK_BLANK";
        cleanup(pane_id);

        assert_eq!(record_task(pane_id, "   ").unwrap(), None);
        assert!(!store_file_path(pane_id).exists());

        cleanup(pane_id);
    }

    #[test]
    fn update_changes_status_of_existing_item() {
        let pane_id = "%TASK_UPDATE";
        cleanup(pane_id);
        let id = record_task(pane_id, "Ship it").unwrap().unwrap();

        assert!(update_task_status(pane_id, &id, TaskItemStatus::Completed).unwrap());

        let items = read_tasks(pane_id);
        assert_eq!(items[0].status, TaskItemStatus::Completed);
        assert_eq!(items[0].subject, "Ship it", "update keeps the subject");

        cleanup(pane_id);
    }

    #[test]
    fn update_unknown_id_changes_nothing() {
        let pane_id = "%TASK_UPDATE_MISS";
        cleanup(pane_id);
        record_task(pane_id, "real item").unwrap();
        let before = store_mtime(pane_id);

        assert!(!update_task_status(pane_id, "999", TaskItemStatus::Completed).unwrap());

        assert_eq!(store_mtime(pane_id), before, "a miss must not rewrite");
        assert_eq!(read_tasks(pane_id)[0].status, TaskItemStatus::Pending);

        cleanup(pane_id);
    }

    #[test]
    fn update_missing_pane_env_is_noop() {
        assert!(!update_task_status("", "1", TaskItemStatus::Completed).unwrap());
    }

    #[test]
    fn trim_drops_oldest_completed_first() {
        let pane_id = "%TASK_TRIM_COMPLETED";
        cleanup(pane_id);

        for i in 1..=MAX_ITEMS {
            record_task(pane_id, &format!("task {i}")).unwrap();
        }
        // Complete the ten oldest items, then push five more past the cap.
        for id in 1..=10 {
            assert!(
                update_task_status(pane_id, &id.to_string(), TaskItemStatus::Completed).unwrap()
            );
        }
        for i in 0..5 {
            record_task(pane_id, &format!("late {i}")).unwrap();
        }

        let items = read_tasks(pane_id);
        assert_eq!(items.len(), MAX_ITEMS, "cap must hold after trim");
        assert_eq!(
            items[0].id, "6",
            "the five oldest completed items are dropped first"
        );
        assert_eq!(items.last().unwrap().subject, "late 4");

        cleanup(pane_id);
    }

    #[test]
    fn trim_falls_back_to_oldest_when_nothing_completed() {
        let pane_id = "%TASK_TRIM_OLDEST";
        cleanup(pane_id);

        for i in 1..=MAX_ITEMS + 3 {
            record_task(pane_id, &format!("task {i}")).unwrap();
        }

        let items = read_tasks(pane_id);
        assert_eq!(items.len(), MAX_ITEMS);
        assert_eq!(items[0].id, "4", "with nothing completed, oldest go first");
        assert_eq!(items.last().unwrap().id, (MAX_ITEMS + 3).to_string());

        cleanup(pane_id);
    }

    #[test]
    fn store_round_trips_pipes_and_newlines_in_subject() {
        // The reason the store is a file and not a tmux option: free
        // text with `|` and newlines must survive a write/read cycle.
        let pane_id = "%TASK_FREE_TEXT";
        cleanup(pane_id);

        record_task(pane_id, "a|b\nc").unwrap();

        let items = read_tasks(pane_id);
        assert_eq!(items[0].subject, "a|b\nc");

        cleanup(pane_id);
    }

    #[test]
    fn corrupt_store_reads_as_empty() {
        let pane_id = "%TASK_CORRUPT";
        let path = store_file_path(pane_id);
        fs::write(&path, "not json at all").unwrap();

        assert!(read_tasks(pane_id).is_empty());

        cleanup(pane_id);
    }

    #[test]
    fn description_round_trips_and_replaces() {
        let pane_id = "%TASK_DESC";
        cleanup(pane_id);

        assert!(set_description(pane_id, "Ship the tasks tab").unwrap());
        assert_eq!(read_task_list(pane_id).description, "Ship the tasks tab");

        assert!(set_description(pane_id, "Do something else").unwrap());
        assert_eq!(read_task_list(pane_id).description, "Do something else");

        cleanup(pane_id);
    }

    #[test]
    fn description_survives_item_writes_and_blank_clears() {
        let pane_id = "%TASK_DESC_ITEMS";
        cleanup(pane_id);

        record_task(pane_id, "an item").unwrap().unwrap();
        set_description(pane_id, "the goal").unwrap();
        // A later task-set keeps the stored description.
        record_task(pane_id, "another item").unwrap().unwrap();
        let list = read_task_list(pane_id);
        assert_eq!(list.description, "the goal");
        assert_eq!(list.items.len(), 2);

        set_description(pane_id, "   ").unwrap();
        assert_eq!(read_task_list(pane_id).description, "");

        cleanup(pane_id);
    }

    #[test]
    fn description_missing_pane_is_noop() {
        assert!(!set_description("", "orphan description").unwrap());
    }

    #[test]
    fn legacy_store_without_description_reads_as_empty() {
        // Stores written before the description field existed carry no
        // `description` key; the serde default must read them cleanly.
        let pane_id = "%TASK_DESC_LEGACY";
        cleanup(pane_id);
        fs::write(
            store_file_path(pane_id),
            r#"{"tasks":[{"id":"1","subject":"old","status":"pending"}]}"#,
        )
        .unwrap();

        let list = read_task_list(pane_id);
        assert_eq!(list.description, "");
        assert_eq!(list.items.len(), 1);

        cleanup(pane_id);
    }

    fn completed_item(id: &str) -> TaskItem {
        TaskItem {
            id: id.into(),
            subject: "done".into(),
            status: TaskItemStatus::Completed,
        }
    }

    #[test]
    fn close_due_hits_the_window_boundary_exactly() {
        let list = TaskList {
            description: String::new(),
            items: vec![completed_item("1")],
        };
        let mtime = Some(1_000);

        assert!(
            !close_due(&list, mtime, 1_000 + TASK_CLOSE_SECS - 1),
            "89 seconds old is inside the window"
        );
        assert!(
            close_due(&list, mtime, 1_000 + TASK_CLOSE_SECS),
            "90 seconds old closes"
        );
    }

    #[test]
    fn close_due_rejects_an_empty_list() {
        // A description-only store never closes: there is nothing to
        // retire and the lingering file dies with /tmp.
        let list = TaskList {
            description: "agent described, then died".into(),
            items: vec![],
        };
        assert!(!close_due(&list, Some(1_000), 1_000 + TASK_CLOSE_SECS + 60));
    }

    #[test]
    fn close_due_rejects_mixed_statuses() {
        let mut list = TaskList {
            description: String::new(),
            items: vec![completed_item("1")],
        };
        list.items.push(TaskItem {
            id: "2".into(),
            subject: "still open".into(),
            status: TaskItemStatus::Pending,
        });

        assert!(!close_due(&list, Some(1_000), 1_000 + TASK_CLOSE_SECS + 60));
    }

    #[test]
    fn close_due_accepts_a_single_completed_item() {
        let list = TaskList {
            description: String::new(),
            items: vec![completed_item("1")],
        };
        assert!(close_due(&list, Some(1_000), 1_000 + TASK_CLOSE_SECS));
    }

    #[test]
    fn close_due_reads_a_future_mtime_as_not_due() {
        // A clock jump must close the list late, never early.
        let list = TaskList {
            description: String::new(),
            items: vec![completed_item("1")],
        };
        assert!(!close_due(&list, Some(1_000), 1_000 - 1));
    }

    #[test]
    fn close_due_reads_a_missing_store_as_not_due() {
        let list = TaskList {
            description: String::new(),
            items: vec![completed_item("1")],
        };
        assert!(!close_due(&list, None, 1_000 + TASK_CLOSE_SECS + 60));
    }

    #[test]
    fn clear_removes_the_store() {
        let pane_id = "%TASK_CLEAR";
        record_task(pane_id, "doomed").unwrap().unwrap();
        assert!(store_file_path(pane_id).exists());

        clear_tasks(pane_id);

        assert!(!store_file_path(pane_id).exists());
        assert!(read_tasks(pane_id).is_empty());
    }

    #[test]
    fn clear_missing_store_and_missing_pane_are_noop() {
        let pane_id = "%TASK_CLEAR_MISS";
        cleanup(pane_id);
        clear_tasks(pane_id); // must not panic on a missing file
        clear_tasks(""); // must not panic without a pane id
    }
}
