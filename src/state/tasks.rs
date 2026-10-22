use std::time::SystemTime;

use crate::task_store::TaskItem;

/// The focused pane's task store, refreshed with the same mtime check
/// as the activity log. Tasks never render from a direct file read;
/// `refresh_pane_tasks` fills `items` once per change.
#[derive(Debug, Clone)]
pub struct TasksState {
    /// The focused pane's stored items in recorded order.
    pub items: Vec<TaskItem>,
    /// `(focused_pane_id, mtime)` of the task store most recently read
    /// into `items`. `refresh_pane_tasks` skips re-reading the store
    /// when neither field has changed.
    pub store_cache: Option<(String, SystemTime)>,
}

impl TasksState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            store_cache: None,
        }
    }
}

impl Default for TasksState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_expected_defaults() {
        let state = TasksState::new();
        assert!(state.items.is_empty());
        assert!(state.store_cache.is_none());
    }

    #[test]
    fn default_delegates_to_new() {
        let default_state = TasksState::default();
        let new_state = TasksState::new();
        assert_eq!(default_state.items.len(), new_state.items.len());
        assert!(default_state.store_cache.is_none());
    }
}
