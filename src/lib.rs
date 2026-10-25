pub mod activity;
pub mod adapter;
pub mod app;
pub mod cli;
pub mod clipboard;
pub mod desktop_notification;
pub mod event;
pub mod git;
pub mod group;
pub mod mcp;
pub mod pmux_task;
pub mod port;
pub(crate) mod process;
pub mod state;
pub mod task_reminders;
pub mod task_store;
#[cfg(test)]
pub(crate) mod test_support;
pub mod time;
pub mod tmux;
pub mod tool_name;
pub mod ui;
pub mod version;
pub mod worktree;

pub const SPINNER_ICON: &str = "●";
pub const SPINNER_PULSE: &[u8] = &[82, 78, 114, 150, 186, 150, 114, 78];
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
