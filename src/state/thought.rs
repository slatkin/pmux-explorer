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
    /// because another tab is displayed leaves the previous thought
    /// standing; a tick with no focused pane, or one whose pane has
    /// no session id, resets to the empty state.
    pub last: LastThought,
    /// Whether the view follows the end of the thought (design D6).
    /// True until the user scrolls up; a scroll back to the bottom
    /// sets it again, and a new `start` resets it, so a growing thought
    /// keeps its last rows on screen unless the user scrolled away.
    pub follow: bool,
    /// The `start_ts` the drawn thought carried when `follow` was last
    /// decided: a different value on the next draw is a new `start`
    /// block, which resets the scroll to follow.
    pub seen_start_ts: Option<String>,
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
            follow: true,
            seen_start_ts: None,
            xdg_runtime_dir: std::env::var("XDG_RUNTIME_DIR").ok(),
            uid: crate::mcp::current_uid(),
        }
    }
}
