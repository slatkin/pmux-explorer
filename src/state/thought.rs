//! The Thought tab's reader state: the incremental feed reader, the
//! last thought it reduced, and the feed environment.

use crate::thinking_feed::{LastThought, ThinkingFeedReader};

/// The Thought tab's state: the focused pi pane's feed, read through
/// [`ThinkingFeedReader`] on the 1 s refresh while the tab is displayed
/// (design D4).
pub struct ThoughtState {
    /// The incremental reader of the focused pane's feed: one cache,
    /// consuming only the bytes a feed grew by since the previous read.
    pub reader: ThinkingFeedReader,
    /// The last thought the reader's most recent read produced — what
    /// the tab draws. Held between ticks, so a tick that reads no feed
    /// (another tab displayed, no focused pane) leaves the previous
    /// thought standing.
    pub last: LastThought,
    /// The `$XDG_RUNTIME_DIR` value, resolved once at startup and held
    /// so [`crate::thinking_feed::feed_path`] takes the env values as
    /// arguments; tests overwrite the field instead of the process env.
    pub xdg_runtime_dir: Option<String>,
    /// The current user's id, for the `/tmp` fallback feed directory.
    pub uid: u32,
}

impl ThoughtState {
    /// Reader state with the feed environment captured from this
    /// process's environment, once at startup.
    pub fn from_process() -> Self {
        ThoughtState {
            reader: ThinkingFeedReader::new(),
            last: LastThought::default(),
            xdg_runtime_dir: std::env::var("XDG_RUNTIME_DIR").ok(),
            uid: crate::mcp::current_uid(),
        }
    }
}
