//! The thinking feed of a pi pane: its path, the last-thought reducer, and
//! the incremental reader.
//!
//! pmux writes the thinking of one pi session as JSON records, one per
//! line, to a per-session feed file; the sidebar reduces those records to
//! a single last thought and reads only the bytes a feed grew by since
//! the previous tick. The record shape and the path rule are shared with
//! the pmux `thinking-feed` change and pinned by tests on both sides.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

/// The feed directory under `XDG_RUNTIME_DIR`, shared with pmux.
const RUNTIME_SUBDIR: &str = "pmux-thinking";

/// The feed directory prefix under `/tmp`, followed by `-<uid>`.
const TMP_SUBDIR_PREFIX: &str = "/tmp/pmux-thinking-";

/// While a thinking block is open, text longer than this many characters
/// is trimmed ...
const STREAM_TRIM_LIMIT: usize = 16_000;
/// ... down to its last this many characters. The half-drop keeps a block
/// from trimming on every delta: after a trim there is room for another
/// 8000 characters before the limit is crossed again.
const STREAM_TRIM_KEEP: usize = 8_000;

/// The size of one backward-scan chunk, in bytes.
const SCAN_CHUNK: u64 = 64 * 1024;

/// The thinking feed path of one session, or `None` for an unsafe id.
///
/// The path is `<dir>/<session id>.jsonl`: `<dir>` is
/// `$XDG_RUNTIME_DIR/pmux-thinking`, or `/tmp/pmux-thinking-<uid>` when
/// `XDG_RUNTIME_DIR` is unset or empty. The env values are arguments, not
/// process env, so tests set no process env. `None` means no feed: an
/// empty session id, `.` or `..`, or an id holding a `/` (which would
/// escape the feed directory).
pub fn feed_path(xdg_runtime_dir: Option<&str>, uid: u32, session_id: &str) -> Option<PathBuf> {
    if session_id.is_empty() || session_id == "." || session_id == ".." || session_id.contains('/')
    {
        return None;
    }
    let dir = match xdg_runtime_dir.filter(|dir| !dir.is_empty()) {
        Some(runtime_dir) => PathBuf::from(runtime_dir).join(RUNTIME_SUBDIR),
        None => PathBuf::from(format!("{TMP_SUBDIR_PREFIX}{uid}")),
    };
    Some(dir.join(format!("{session_id}.jsonl")))
}

/// The last thought of one feed: the text of the newest thinking block,
/// the `ts` of the record that began it, and whether that block is still
/// open. This is the whole reducer state (design D3) — no multi-block
/// buffer, no per-producer blocks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LastThought {
    /// The text of the last thought, terminal-safe from ingest on.
    pub text: String,
    /// The `ts` of the `start` record that began the thought, or `None`
    /// when no `start` is in the feed (deltas alone still make a text).
    pub start_ts: Option<String>,
    /// Whether the block is still open (`start` seen, no `end` yet).
    pub open: bool,
}

impl LastThought {
    /// Whether no thought is in the feed: no text has arrived yet.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// One feed line: `{kind, ts, producer, text?}`. The `producer` field is
/// part of the record shape but is not read — pmux is the only producer,
/// and the last thought is the newest block whatever its producer.
#[derive(Debug, Deserialize)]
struct FeedRecord {
    kind: String,
    ts: String,
    text: Option<String>,
}

/// Feed one line to the last-thought reducer. A line that is not a valid
/// record — broken JSON, a missing `ts`, an unknown `kind` — is ignored.
fn apply_line(state: &mut LastThought, line: &str) {
    let Ok(record) = serde_json::from_str::<FeedRecord>(line) else {
        return;
    };
    match record.kind.as_str() {
        "start" => {
            state.text.clear();
            state.start_ts = Some(record.ts);
            state.open = true;
        }
        "delta" => {
            if let Some(text) = record.text {
                state.text.push_str(&terminal_safe(&text));
                if state.open {
                    let count = state.text.chars().count();
                    if count > STREAM_TRIM_LIMIT {
                        state.text = state.text.chars().skip(count - STREAM_TRIM_KEEP).collect();
                    }
                }
            }
        }
        "end" => {
            if let Some(text) = record.text {
                state.text = terminal_safe(&text);
            }
            state.open = false;
        }
        _ => {}
    }
}

/// Terminal-safe text: control characters other than `\n` and `\t` and
/// every escape sequence are removed, the same intent as the producer's
/// own `terminalSafe`. The sidebar re-applies it at ingest, so no slip on
/// either side reaches the terminal.
fn terminal_safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.peek() {
                // CSI: parameter and intermediate bytes, then one final byte.
                Some('[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                // OSC: terminated by BEL or by ST (`ESC \`).
                Some(']') => {
                    chars.next();
                    loop {
                        match chars.next() {
                            None | Some('\x07') => break,
                            Some('\x1b') => {
                                chars.next();
                                break;
                            }
                            Some(_) => {}
                        }
                    }
                }
                // Any other escape: intermediate bytes, then one final byte.
                _ => {
                    while matches!(chars.peek(), Some(n) if ('\u{20}'..='\u{2f}').contains(n)) {
                        chars.next();
                    }
                    chars.next();
                }
            },
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Per-feed reader state: which session the cache belongs to, how much of
/// the file has been consumed, the mtime the consumed bytes were read
/// under, and the reduced last thought.
#[derive(Debug)]
struct FeedCache {
    session_id: String,
    offset: u64,
    mtime: Option<SystemTime>,
    state: LastThought,
}

impl FeedCache {
    /// The cache of a feed seen for the first time: nothing consumed, no
    /// mtime, no thought.
    fn new(session_id: &str) -> Self {
        FeedCache {
            session_id: session_id.to_string(),
            offset: 0,
            mtime: None,
            state: LastThought::default(),
        }
    }
}

/// The incremental reader of the thinking feeds (design D4): one cache
/// per feed, read only on the refresh tick while the Thought tab is
/// drawn.
#[derive(Debug, Default)]
pub struct ThinkingFeedReader {
    cache: Option<FeedCache>,
}

impl ThinkingFeedReader {
    /// A reader with no feed read yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The last thought of one feed after this tick's read.
    ///
    /// `path` is the feed path resolved by [`feed_path`] — `None` for a
    /// missing or unsafe session id, which forgets the cache and gives
    /// the empty state. A read processes only the bytes added since the
    /// previous read of the same feed; the first read of a feed (and a
    /// read after the file was replaced, shortened, or recreated) begins
    /// at its newest `start` record, not at the start of the file.
    pub fn read(&mut self, session_id: &str, path: Option<&Path>) -> LastThought {
        let Some(path) = path else {
            self.cache = None;
            return LastThought::default();
        };
        let (len, mtime) = match fs::metadata(path) {
            Ok(meta) => (Some(meta.len()), meta.modified().ok()),
            Err(_) => (None, None),
        };

        let mut cache = match self.cache.take() {
            Some(c) if c.session_id == session_id => c,
            _ => FeedCache::new(session_id),
        };

        // The feed is gone — the sweep deletes it. Empty for now; the
        // `mtime` of `None` below makes a re-appearing file a rescan.
        let Some(len) = len else {
            self.cache = Some(FeedCache::new(session_id));
            return LastThought::default();
        };

        // An unchanged feed costs no read at all. The length must equal
        // the consumed offset too: a feed that grew while its mtime stayed
        // equal (coarse timestamp granularity) is not unchanged.
        if cache.mtime.is_some() && mtime == cache.mtime && len == cache.offset {
            let state = cache.state.clone();
            self.cache = Some(cache);
            return state;
        }
        let rescan = cache.mtime.is_none() || len < cache.offset;

        let result = File::open(path).and_then(|mut file| {
            if rescan {
                let from = backward_scan_start(&mut file, len)?.unwrap_or(0);
                let mut state = LastThought::default();
                let offset = read_region_into(&mut file, from, len, &mut state)?;
                Ok((state, offset))
            } else {
                let offset = read_region_into(&mut file, cache.offset, len, &mut cache.state)?;
                Ok((std::mem::take(&mut cache.state), offset))
            }
        });
        if let Ok((state, offset)) = result {
            cache.state = state;
            cache.offset = offset;
            cache.mtime = mtime;
        }
        // On an error (the feed vanished mid-read) the previous state and
        // offset stand, and the next tick's mtime check tries again.
        let state = cache.state.clone();
        self.cache = Some(cache);
        state
    }
}

/// Read the feed from `from` to the end of its complete lines, feeding
/// each complete line to the reducer, and return the offset the next read
/// continues at. A trailing partial line is held back by leaving the
/// offset before it, so a line being written completes on a later tick.
fn read_region_into(
    file: &mut File,
    from: u64,
    len: u64,
    state: &mut LastThought,
) -> std::io::Result<u64> {
    file.seek(SeekFrom::Start(from))?;
    let mut bytes = Vec::new();
    file.take(len - from).read_to_end(&mut bytes)?;
    let consumed = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    for line in bytes[..consumed].split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        if let Ok(line) = std::str::from_utf8(line) {
            apply_line(state, line);
        }
    }
    Ok(from + consumed as u64)
}

/// Byte offset of the newest `start` record in the file, found by reading
/// backward in 64 KiB chunks, or `None` when the file holds no `start`
/// record (reading then begins at the start of the file). The first read
/// of a multi-megabyte feed parses only its newest block.
fn backward_scan_start(file: &mut File, len: u64) -> std::io::Result<Option<u64>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut pos = len;
    loop {
        let chunk_start = pos.saturating_sub(SCAN_CHUNK);
        let mut chunk = vec![0u8; (pos - chunk_start) as usize];
        file.seek(SeekFrom::Start(chunk_start))?;
        file.read_exact(&mut chunk)?;
        let mut merged = chunk;
        merged.extend_from_slice(&buf);
        buf = merged;
        pos = chunk_start;
        if let Some(offset) = last_start_offset_in(&buf, chunk_start == 0) {
            return Ok(Some(chunk_start + offset));
        }
        if chunk_start == 0 {
            return Ok(None);
        }
    }
}

/// Byte offset within `buf` of the newest line whose record is a `start`.
///
/// The first segment is a possibly-partial line whose beginning lies in a
/// chunk not yet read; it counts only when `buf` begins at the start of
/// the file. The trailing segment after the last newline may be a line
/// still being written and counts as well — a partial record fails to
/// parse, and a complete one is a genuine newest `start`.
fn last_start_offset_in(buf: &[u8], at_file_start: bool) -> Option<u64> {
    let mut cursor = 0usize;
    let mut first = true;
    let mut found = None;
    while cursor < buf.len() {
        let end = cursor
            + buf[cursor..]
                .iter()
                .position(|&b| b == b'\n')
                .unwrap_or(buf.len() - cursor);
        if (!first || at_file_start) && is_start_record(&buf[cursor..end]) {
            found = Some(cursor as u64);
        }
        first = false;
        cursor = end + 1;
    }
    found
}

/// Whether one line's record is a `start` record.
fn is_start_record(line: &[u8]) -> bool {
    line.starts_with(b"{")
        && std::str::from_utf8(line)
            .ok()
            .and_then(|line| serde_json::from_str::<FeedRecord>(line).ok())
            .is_some_and(|record| record.kind == "start")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::TempDir;

    /// A feed record line, in the shape pmux writes.
    fn line(kind: &str, ts: &str, text: Option<&str>) -> String {
        let text = text.map_or(String::new(), |t| {
            format!(r#","text":{}"#, serde_json::to_string(t).unwrap())
        });
        format!(r#"{{"kind":"{kind}","ts":"{ts}","producer":"pmux"{text}}}"#)
    }

    /// Reduce a run of feed lines to one last thought.
    fn reduced(lines: &[&str]) -> LastThought {
        let mut state = LastThought::default();
        for line in lines {
            apply_line(&mut state, line);
        }
        state
    }

    /// A mtime step, so two writes of one test never share an mtime.
    fn mtime_step() {
        std::thread::sleep(Duration::from_millis(5));
    }

    // Row 1.1: the path rule.

    #[test]
    fn the_shared_path_vector_holds() {
        // The vector the pmux `thinking-feed` change pins: session `abc`,
        // uid 1000, runtime dir `/run/user/1000`.
        assert_eq!(
            feed_path(Some("/run/user/1000"), 1000, "abc")
                .unwrap()
                .to_string_lossy(),
            "/run/user/1000/pmux-thinking/abc.jsonl"
        );
        assert_eq!(
            feed_path(None, 1000, "abc").unwrap().to_string_lossy(),
            "/tmp/pmux-thinking-1000/abc.jsonl"
        );
    }

    #[test]
    fn an_empty_runtime_dir_falls_back_to_the_tmp_directory() {
        assert_eq!(
            feed_path(Some(""), 1000, "abc").unwrap().to_string_lossy(),
            "/tmp/pmux-thinking-1000/abc.jsonl"
        );
    }

    #[test]
    fn an_unsafe_session_id_gives_no_feed() {
        for id in ["", ".", "..", "/", "a/b", "../x", "./x"] {
            assert!(
                feed_path(Some("/run/user/1000"), 1000, id).is_none(),
                "session id {id:?} must be rejected"
            );
        }
    }

    // Row 1.2: the records make the last thought.

    #[test]
    fn a_thought_grows() {
        let state = reduced(&[
            &line("start", "2026-10-27T08:00:07Z", None),
            &line("delta", "t", Some("I should")),
            &line("delta", "t", Some(" read the file")),
        ]);
        assert_eq!(state.text, "I should read the file");
        assert_eq!(state.start_ts.as_deref(), Some("2026-10-27T08:00:07Z"));
        assert!(state.open);
    }

    #[test]
    fn the_end_replaces_the_text() {
        let state = reduced(&[
            &line("start", "t1", None),
            &line("delta", "t", Some("draft")),
            &line("end", "t", Some("final")),
        ]);
        assert_eq!(state.text, "final");
        assert!(!state.open);
        // The thought keeps the `ts` of the record that began it.
        assert_eq!(state.start_ts.as_deref(), Some("t1"));
    }

    #[test]
    fn an_end_without_text_keeps_the_text() {
        let state = reduced(&[
            &line("start", "t1", None),
            &line("delta", "t", Some("draft")),
            &line("end", "t", None),
        ]);
        assert_eq!(state.text, "draft");
        assert!(!state.open);
    }

    #[test]
    fn a_finished_thought_stays() {
        let state = reduced(&[&line("start", "t1", None), &line("end", "t", Some("done"))]);
        assert_eq!(state.text, "done");
    }

    #[test]
    fn a_new_block_clears_the_old_one() {
        let state = reduced(&[
            &line("start", "t1", None),
            &line("delta", "t", Some("first")),
            &line("end", "t", None),
            &line("start", "t2", None),
        ]);
        assert!(state.is_empty());
        assert_eq!(state.start_ts.as_deref(), Some("t2"));
        assert!(state.open);
    }

    #[test]
    fn a_delta_without_a_start_makes_a_text() {
        let state = reduced(&[
            &line("delta", "t", Some("orphan")),
            &line("delta", "t", Some(" bits")),
        ]);
        assert_eq!(state.text, "orphan bits");
        assert_eq!(state.start_ts, None);
    }

    #[test]
    fn a_broken_line_is_ignored() {
        let state = reduced(&[
            &line("start", "t1", None),
            "not json",
            r#"{"kind":"delta"}"#,    // no ts
            r#"{"kind":"nonsense"}"#, // unknown kind
            &line("delta", "t", Some("kept")),
        ]);
        assert_eq!(state.text, "kept");
    }

    // Row 1.2: the last thought is bounded.

    #[test]
    fn a_long_streaming_thought_keeps_its_tail() {
        // 40 deltas of 1000 characters each. The half-drop trims at the
        // 17th, 26th and 34th delta, leaving the last 13000 characters.
        let deltas: Vec<String> = (0..40).map(|i| format!("{i:0>1000}")).collect();
        let lines: Vec<String> = std::iter::once(line("start", "t1", None))
            .chain(deltas.iter().map(|d| line("delta", "t", Some(d))))
            .collect();
        let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let state = reduced(&line_refs);
        assert_eq!(state.text.chars().count(), 13_000);
        let expected: String = deltas[27..].concat();
        assert_eq!(state.text, expected);
    }

    #[test]
    fn the_text_of_an_end_record_is_kept_whole() {
        let long = "x".repeat(20_000);
        let state = reduced(&[&line("start", "t1", None), &line("end", "t", Some(&long))]);
        assert_eq!(state.text.chars().count(), 20_000);
    }

    // Row 1.2: the thought is shown as plain text.

    #[test]
    fn an_escape_sequence_is_removed() {
        let state = reduced(&[&line("delta", "t", Some("\u{1b}[31mred"))]);
        assert_eq!(state.text, "red");
    }

    #[test]
    fn other_escapes_and_control_characters_are_removed() {
        let state = reduced(&[&line(
            "end",
            "t",
            Some("a\u{7}b\u{13}\u{1b}]0;title\u{7}c\u{1b}(Bd\u{1b}M"),
        )]);
        assert_eq!(state.text, "abcd");
    }

    #[test]
    fn line_breaks_and_tabs_are_kept() {
        let state = reduced(&[&line("delta", "t", Some("one\ntwo\tend"))]);
        assert_eq!(state.text, "one\ntwo\tend");
    }

    // Row 1.3: the incremental reader, on temp files.

    fn feed_in(dir: &TempDir, name: &str) -> PathBuf {
        dir.path().join(format!("{name}.jsonl"))
    }

    #[test]
    fn the_first_read_of_a_large_feed_parses_only_the_newest_block() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "large");
        // Two thousand earlier blocks, roughly 1 MB, then the newest one.
        let mut content = String::new();
        for i in 0..2000 {
            content.push_str(&format!(
                r#"{{"kind":"start","ts":"2026-01-01T00:{:02}:{:02}Z","producer":"pmux"}}"#,
                i / 60,
                i % 60
            ));
            content.push('\n');
            content.push_str(&line("delta", "t", Some(&"x".repeat(200))));
            content.push('\n');
            content.push_str(&line("end", "t", Some("old")));
            content.push('\n');
        }
        let last_start_offset = content.len() as u64;
        content.push_str(&line("start", "2026-10-27T08:00:07Z", None));
        content.push('\n');
        content.push_str(&line("delta", "t", Some("newest thought")));
        content.push('\n');
        fs::write(&path, &content).unwrap();

        // The backward scan lands on the newest `start` record.
        let mut file = File::open(&path).unwrap();
        assert_eq!(
            backward_scan_start(&mut file, content.len() as u64).unwrap(),
            Some(last_start_offset)
        );

        let mut reader = ThinkingFeedReader::new();
        let thought = reader.read("sess", Some(&path));
        assert_eq!(thought.text, "newest thought");
        assert_eq!(thought.start_ts.as_deref(), Some("2026-10-27T08:00:07Z"));
        assert!(thought.open);
    }

    #[test]
    fn an_append_between_reads_shows_on_the_next_read() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "append");
        fs::write(&path, line("start", "t1", None) + "\n").unwrap();

        let mut reader = ThinkingFeedReader::new();
        let thought = reader.read("s", Some(&path));
        assert!(thought.is_empty() && thought.open);

        mtime_step();
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", line("delta", "t2", Some("hello"))).unwrap();
        drop(file);

        let thought = reader.read("s", Some(&path));
        assert_eq!(thought.text, "hello");
    }

    #[test]
    fn a_partial_trailing_line_is_held_back_until_it_completes() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "partial");
        fs::write(&path, line("start", "t1", None) + "\n").unwrap();

        let mut reader = ThinkingFeedReader::new();
        assert_eq!(reader.read("s", Some(&path)).text, "");

        mtime_step();
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        write!(
            file,
            r#"{{"kind":"delta","ts":"t2","producer":"pmux","text":"hel"#
        )
        .unwrap();
        drop(file);
        mtime_step();
        // The half-written line is not a record yet: nothing is applied.
        assert_eq!(reader.read("s", Some(&path)).text, "");

        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, r#"lo"}}"#).unwrap();
        drop(file);
        mtime_step();
        assert_eq!(reader.read("s", Some(&path)).text, "hello");
    }

    #[test]
    fn a_replaced_shorter_file_is_scanned_anew() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "replace");
        let long = line("start", "t1", None)
            + "\n"
            + &line("delta", "t", Some(&"x".repeat(100_000)))
            + "\n";
        fs::write(&path, &long).unwrap();

        let mut reader = ThinkingFeedReader::new();
        let thought = reader.read("s", Some(&path));
        // The open block trims to its last 8000 characters.
        assert_eq!(thought.text.chars().count(), 8_000);

        mtime_step();
        let fresh = line("start", "t9", None) + "\n" + &line("delta", "t", Some("fresh")) + "\n";
        fs::write(&path, &fresh).unwrap();
        mtime_step();

        let thought = reader.read("s", Some(&path));
        assert_eq!(thought.text, "fresh");
        assert_eq!(thought.start_ts.as_deref(), Some("t9"));
    }

    #[test]
    fn a_missing_feed_gives_the_empty_state_and_a_later_file_is_scanned() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "missing");
        let mut reader = ThinkingFeedReader::new();
        assert!(reader.read("s", Some(&path)).is_empty());
        // The missing feed is remembered: the next read is a quiet no-op.
        assert!(reader.read("s", Some(&path)).is_empty());

        mtime_step();
        fs::write(
            &path,
            line("start", "t1", None) + "\n" + &line("delta", "t2", Some("late")) + "\n",
        )
        .unwrap();
        assert_eq!(reader.read("s", Some(&path)).text, "late");
    }

    #[test]
    fn no_path_forgets_the_cache_and_gives_the_empty_state() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "nopath");
        fs::write(
            &path,
            line("start", "t1", None) + "\n" + &line("delta", "t2", Some("kept")) + "\n",
        )
        .unwrap();

        let mut reader = ThinkingFeedReader::new();
        assert_eq!(reader.read("s", Some(&path)).text, "kept");
        assert!(reader.read("s", None).is_empty());
        // A later read of the same feed starts over cleanly.
        assert_eq!(reader.read("s", Some(&path)).text, "kept");
    }

    #[test]
    fn an_unchanged_feed_reads_the_same_state_twice() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "quiet");
        fs::write(
            &path,
            line("start", "t1", None) + "\n" + &line("delta", "t2", Some("settled")) + "\n",
        )
        .unwrap();

        let mut reader = ThinkingFeedReader::new();
        let first = reader.read("s", Some(&path));
        let second = reader.read("s", Some(&path));
        assert_eq!(first, second);
        assert_eq!(second.text, "settled");
    }

    #[test]
    fn growth_under_an_unchanged_mtime_is_still_read() {
        let dir = TempDir::new().unwrap();
        let path = feed_in(&dir, "coarse");
        fs::write(&path, line("start", "t1", None) + "\n").unwrap();

        let mut reader = ThinkingFeedReader::new();
        assert!(reader.read("s", Some(&path)).is_empty());

        // Coarse timestamp granularity: the appended record leaves the
        // file's mtime as it was, so only the grown length betrays it.
        let old_mtime = fs::metadata(&path).unwrap().modified().unwrap();
        mtime_step();
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", line("delta", "t2", Some("sneaky"))).unwrap();
        drop(file);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old_mtime)
            .unwrap();

        let thought = reader.read("s", Some(&path));
        assert_eq!(thought.text, "sneaky");
    }
}
