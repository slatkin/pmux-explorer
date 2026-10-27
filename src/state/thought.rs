//! The Thought tab's reader state: the incremental feed reader, the
//! last thought it reduced, and the feed environment.

use crate::thinking_feed::{FeedEnv, LastThought, ThinkingFeedReader};

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
    /// The feed environment, resolved once at startup and passed
    /// whole to [`FeedEnv::feed_path`]; tests overwrite the field
    /// instead of the process env.
    pub env: FeedEnv,
    /// The `HH:mm:ss` stamp formatter for the `start_ts` — the
    /// local-time read in production, a fixed-zone one in snapshot
    /// tests, so no test mutates the process timezone.
    pub hhmmss: fn(&str) -> Option<String>,
}

impl ThoughtState {
    /// Reader state with the feed environment captured from this
    /// process's environment, once at startup, and the local-time
    /// stamp formatter.
    pub fn from_process() -> Self {
        ThoughtState {
            reader: ThinkingFeedReader::new(),
            last: LastThought::default(),
            follow: true,
            seen_start_ts: None,
            env: FeedEnv::from_process(),
            hhmmss: crate::time::local_hhmmss_from_ts,
        }
    }
}
