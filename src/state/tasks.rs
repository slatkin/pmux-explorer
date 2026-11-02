use std::time::SystemTime;

use super::ScrollState;
use crate::task_store::TaskItem;

/// The board owner's task store, refreshed with the same mtime check
/// as the activity log. Tasks never render from a direct file read;
/// `refresh_pane_tasks` fills `items` once per change. The board
/// owner is the focused pane itself, except for a pi pmux subagent
/// with a live parent, which shows its ultimate parent's board.
#[derive(Debug, Clone)]
pub struct TasksState {
    /// The board owner's stored items in recorded order.
    pub items: Vec<TaskItem>,
    /// The board owner's stored description, rendered pinned above
    /// the item list.
    pub description: String,
    /// `(board owner pane id, mtime)` of the task store most recently
    /// read into `items`. The board owner is the focused pane itself,
    /// except for a pmux subagent with a live parent, which reads its
    /// ultimate parent — so a focus move between a parent and its
    /// subagent hits the cache. `refresh_pane_tasks` skips re-reading
    /// the store when neither field has changed.
    pub store_cache: Option<(String, SystemTime)>,
    /// The Tasks tab's own scroll offset. Content is laid out fresh
    /// every frame; `total_lines`/`visible_height` are written by the
    /// renderer and shifts are clamped here, exactly like the
    /// Activity tab's [`super::ActivityState::scroll`].
    pub scroll: ScrollState,
}

impl TasksState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            description: String::new(),
            store_cache: None,
            scroll: ScrollState::default(),
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
        assert_eq!(default_state.scroll.offset, new_state.scroll.offset);
    }
}
