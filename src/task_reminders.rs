//! Task reminder wording, defined here exactly once for every surface.
//!
//! The two strings below are the repo's single source for the agent-tasks
//! reminders described in the `agent-tasks` spec: each is one short,
//! deliberately low-key single-line sentence, so a surface can quote a
//! constant verbatim. Each constant's doc comment carries the firing
//! condition its wording no longer names. The `task-set` / `task-update`
//! help text embeds both; `docs/agent-protocol.md` and the pmux-extension
//! companion carry the same wording by referencing this file. Delivery is
//! wired outside this crate, so editing a string here never changes when a
//! reminder fires.

/// Wording delivered when a turn starts and the pane's todo list is empty.
///
/// Covers a fresh pane and a pane whose store was cleared at session
/// start alike: both are the same "empty store" condition, so the
/// wording does not distinguish them.
pub const RECORD_FIRST_REMINDER: &str =
    "No tasks recorded yet: before starting work, record your todo list with task-set <subject>.";

/// Wording delivered when a turn ends while todo items are still open.
pub const UPDATE_BEFORE_FINISH_REMINDER: &str = "Reminder: make sure the task board is up to date.";

#[cfg(test)]
mod tests {
    use super::*;

    /// One reminder is one short single-line string: multi-line or long
    /// text would not survive verbatim injection into agent prompt text.
    fn assert_short_one_line(text: &str) {
        assert!(!text.contains('\n'), "a reminder must stay on one line");
        assert!(
            text.len() <= 120,
            "a reminder must stay short enough to inject verbatim: {text}"
        );
    }

    #[test]
    fn reminders_are_one_short_line_each() {
        assert_short_one_line(RECORD_FIRST_REMINDER);
        assert_short_one_line(UPDATE_BEFORE_FINISH_REMINDER);
    }
}
