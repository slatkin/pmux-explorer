//! Per-pane task store backing the bottom panel's Tasks tab.
//!
//! One JSON file per tmux pane at `/tmp/pmux-explorer-tasks<PANE>.json`,
//! mirroring the activity log's path encoding, mtime polling, and
//! session semantics: agents write through the `task-set` /
//! `task-update` CLI subcommands, the TUI only reads.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Maximum number of items kept in one pane's store. When a record
/// pushes past the cap, completed items are trimmed oldest-first; if
/// none are completed, the oldest item goes so the cap still holds.
pub const MAX_ITEMS: usize = 50;

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
/// A missing pane id or a blank subject is a no-op returning `None`.
pub fn record_task(pane_id: &str, subject: &str) -> Option<String> {
    if pane_id.is_empty() || subject.trim().is_empty() {
        return None;
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
    write_store(pane_id, &list);
    Some(id)
}

/// Set the status of the item with `id`. Returns `false` when the pane
/// id is empty or the item does not exist, so callers can no-op.
pub fn update_task_status(pane_id: &str, id: &str, status: TaskItemStatus) -> bool {
    if pane_id.is_empty() {
        return false;
    }
    let mut list = read_task_list(pane_id);
    let Some(item) = list.items.iter_mut().find(|task| task.id == id) else {
        return false;
    };
    item.status = status;
    write_store(pane_id, &list);
    true
}

/// Set the pane-wide description shown above the Tasks tab's item
/// list. Later writes replace it; a blank description clears it.
/// Returns `false` when the pane id is empty, so callers can no-op.
pub fn set_description(pane_id: &str, description: &str) -> bool {
    if pane_id.is_empty() {
        return false;
    }
    let mut list = read_task_list(pane_id);
    list.description = description.trim().to_string();
    write_store(pane_id, &list);
    true
}

/// Remove a pane's store. Used at SessionStart so a pane reused for
/// new work starts with an empty list, matching the activity log's
/// session semantics. A missing store is already empty.
pub fn clear_tasks(pane_id: &str) {
    if pane_id.is_empty() {
        return;
    }
    let _ = fs::remove_file(store_file_path(pane_id));
}

fn write_store(pane_id: &str, list: &TaskList) {
    let file = StoreFile {
        description: list.description.clone(),
        tasks: list.items.clone(),
    };
    if let Ok(json) = serde_json::to_string(&file) {
        let _ = fs::write(store_file_path(pane_id), json);
    }
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

        let first = record_task(pane_id, "Fix the login bug").unwrap();
        let second = record_task(pane_id, "Write the docs").unwrap();

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

        assert_eq!(record_task("", "orphan subject"), None);

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

        assert_eq!(record_task(pane_id, "   "), None);
        assert!(!store_file_path(pane_id).exists());

        cleanup(pane_id);
    }

    #[test]
    fn update_changes_status_of_existing_item() {
        let pane_id = "%TASK_UPDATE";
        cleanup(pane_id);
        let id = record_task(pane_id, "Ship it").unwrap();

        assert!(update_task_status(pane_id, &id, TaskItemStatus::Completed));

        let items = read_tasks(pane_id);
        assert_eq!(items[0].status, TaskItemStatus::Completed);
        assert_eq!(items[0].subject, "Ship it", "update keeps the subject");

        cleanup(pane_id);
    }

    #[test]
    fn update_unknown_id_changes_nothing() {
        let pane_id = "%TASK_UPDATE_MISS";
        cleanup(pane_id);
        record_task(pane_id, "real item");
        let before = store_mtime(pane_id);

        assert!(!update_task_status(
            pane_id,
            "999",
            TaskItemStatus::Completed
        ));

        assert_eq!(store_mtime(pane_id), before, "a miss must not rewrite");
        assert_eq!(read_tasks(pane_id)[0].status, TaskItemStatus::Pending);

        cleanup(pane_id);
    }

    #[test]
    fn update_missing_pane_env_is_noop() {
        assert!(!update_task_status("", "1", TaskItemStatus::Completed));
    }

    #[test]
    fn trim_drops_oldest_completed_first() {
        let pane_id = "%TASK_TRIM_COMPLETED";
        cleanup(pane_id);

        for i in 1..=MAX_ITEMS {
            record_task(pane_id, &format!("task {i}"));
        }
        // Complete the ten oldest items, then push five more past the cap.
        for id in 1..=10 {
            assert!(update_task_status(
                pane_id,
                &id.to_string(),
                TaskItemStatus::Completed
            ));
        }
        for i in 0..5 {
            record_task(pane_id, &format!("late {i}"));
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
            record_task(pane_id, &format!("task {i}"));
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

        record_task(pane_id, "a|b\nc");

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

        assert!(set_description(pane_id, "Ship the tasks tab"));
        assert_eq!(read_task_list(pane_id).description, "Ship the tasks tab");

        assert!(set_description(pane_id, "Do something else"));
        assert_eq!(read_task_list(pane_id).description, "Do something else");

        cleanup(pane_id);
    }

    #[test]
    fn description_survives_item_writes_and_blank_clears() {
        let pane_id = "%TASK_DESC_ITEMS";
        cleanup(pane_id);

        record_task(pane_id, "an item").unwrap();
        set_description(pane_id, "the goal");
        // A later task-set keeps the stored description.
        record_task(pane_id, "another item").unwrap();
        let list = read_task_list(pane_id);
        assert_eq!(list.description, "the goal");
        assert_eq!(list.items.len(), 2);

        set_description(pane_id, "   ");
        assert_eq!(read_task_list(pane_id).description, "");

        cleanup(pane_id);
    }

    #[test]
    fn description_missing_pane_is_noop() {
        assert!(!set_description("", "orphan description"));
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

    #[test]
    fn clear_removes_the_store() {
        let pane_id = "%TASK_CLEAR";
        record_task(pane_id, "doomed").unwrap();
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
