use unicode_width::UnicodeWidthStr;

use crate::shell_highlight::Class;

/// Display width of a string (CJK = 2 columns, ASCII = 1)
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Pad string with spaces to fill `target_width` display columns
pub fn pad_to(current_display_width: usize, target_width: usize) -> String {
    let pad = target_width.saturating_sub(current_display_width);
    " ".repeat(pad)
}

pub fn elapsed_label(started_at: Option<u64>, now: u64) -> String {
    let ts = match started_at {
        Some(t) if t > 0 => t,
        _ => return String::new(),
    };
    if now < ts {
        return String::new();
    }
    let elapsed = now - ts;
    let secs = elapsed % 60;
    let mins = (elapsed / 60) % 60;
    let hours = elapsed / 3600;
    if hours > 0 {
        format!("{}h{}m{}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m{}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// One entry of the shared wait-reason vocabulary. A single table answers
/// both questions the sidebar asks about a raw reason: the label to show,
/// and whether the reason demands the user (so a live background shell does
/// not lower the pane's status).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitReasonEntry {
    /// The label the pane row and the desktop notification body show.
    pub label: &'static str,
    /// The reason names a blocking user prompt or permission dialog.
    pub action_required: bool,
}

/// Look up a raw producer token in the shared wait-reason table.
///
/// `None` means the table does not name the reason. The caller then shows
/// the token unchanged, so a producer's new vocabulary still surfaces.
pub fn wait_reason_entry(reason: &str) -> Option<WaitReasonEntry> {
    let (label, action_required) = match reason {
        // Claude's notification vocabulary.
        "permission" => ("permission", true),
        "permission_prompt" => ("permission required", true),
        "permission_denied" => ("permission denied", true),
        "elicitation_dialog" => ("waiting for selection", true),
        "idle_prompt" => ("waiting for input", false),
        "auth_success" => ("auth success", false),
        "rate_limit" => ("rate limit", false),
        "session_resumed" => ("resumed", false),
        "session_resumed_compact" => ("resumed (compact)", false),
        // pi's `ui_prompt` kinds: each one blocks on a human.
        "select" => ("waiting for selection", true),
        "confirm" => ("confirmation", true),
        "input" => ("waiting for input", true),
        "editor" => ("editor", true),
        "custom" => ("prompt", true),
        // pi-ask-user's `herdr:blocked` wait. pi-ask-user emits it before the
        // ui_prompt opens, so an ask_user wait publishes this token first.
        "question" => ("prompt", true),
        _ => return None,
    };
    Some(WaitReasonEntry {
        label,
        action_required,
    })
}

pub fn wait_reason_label(reason: &str) -> String {
    if let Some(entry) = wait_reason_entry(reason) {
        return entry.label.into();
    }
    if reason.is_empty() {
        String::new()
    } else if let Some(rest) = reason.strip_prefix("teammate_idle:") {
        match rest.split_once(':') {
            Some((name, why)) if !why.is_empty() => format!("{name} idle ({why})"),
            _ => format!("{rest} idle"),
        }
    } else {
        reason.to_string()
    }
}

pub fn branch_label(git_info: &crate::group::PaneGitInfo) -> String {
    match &git_info.branch {
        Some(branch) => {
            if git_info.is_worktree {
                match &git_info.worktree_name {
                    Some(name) if name != branch => format!("+ {}: {}", name, branch),
                    _ => format!("+ {}", branch),
                }
            } else {
                branch.clone()
            }
        }
        None => String::new(),
    }
}

/// Truncate string to fit within max display width, adding … if needed
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let dw = display_width(text);
    if dw <= max_width {
        return text.to_string();
    }
    let mut result = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw + 1 > max_width {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result.push('\u{2026}');
    result
}

/// Wrap text by display width, breaking at word boundaries.
///
/// Words stay whole unless a single word is wider than the line, in which
/// case it is hard-split across lines. When the text does not fit within
/// `max_lines` lines, the final line keeps the last whole words that fit
/// and closes with an ellipsis; the agent label and elapsed label elsewhere
/// rely on `truncate_to_width` for the same treatment on single lines.
/// Returns at most `max_lines` lines; empty text returns no lines.
pub fn wrap_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    // Every char gets the plain argument class, so the shared layout
    // core produces exactly the plain-text lines this signature returns.
    let classes = vec![Class::Arg; text.chars().count()];
    wrap_classified(text, &classes, max_width, max_lines)
        .into_iter()
        .map(|runs| runs.into_iter().map(|(chunk, _)| chunk).collect::<String>())
        .collect()
}

/// Wrap classified text by display width with the same layout as
/// [`wrap_text`]: word boundaries, hard-split oversized words, at most
/// `max_lines` lines, and an ellipsis at the end of the last line when
/// text remains. Each line comes back as runs of consecutive characters
/// with the same class, ready to draw one span per run. The ellipsis is
/// class `Arg`; every other character keeps the class its position in
/// `classes` gives it (one entry per `char` of `text`; a missing tail
/// defaults to `Arg`). Empty or whitespace-only text returns no lines.
pub fn wrap_classified(
    text: &str,
    classes: &[Class],
    max_width: usize,
    max_lines: usize,
) -> Vec<Vec<(String, Class)>> {
    if max_width == 0 || max_lines == 0 {
        return Vec::new();
    }

    let chars: Vec<(char, Class)> = text
        .chars()
        .zip(classes.iter().copied().chain(std::iter::repeat(Class::Arg)))
        .collect();

    // Tokens keep words whole: a word wider than the line is pre-split
    // into width-fitting chunks, and every chunk after the first continues
    // the word with no space before it. Whitespace splits words but is not
    // part of any token; the join re-inserts one space between tokens.
    let mut tokens: Vec<(Vec<(char, Class)>, bool)> = Vec::new(); // (chars, space before)
    let mut word: Vec<(char, Class)> = Vec::new();
    let mut first_chunk = true;
    let flush_word = |tokens: &mut Vec<(Vec<(char, Class)>, bool)>,
                      word: &mut Vec<(char, Class)>,
                      first_chunk: &mut bool| {
        if word.is_empty() {
            return;
        }
        let mut chunk: Vec<(char, Class)> = Vec::new();
        let mut chunk_w = 0usize;
        for (ch, class) in word.drain(..) {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if chunk_w + cw > max_width {
                tokens.push((std::mem::take(&mut chunk), *first_chunk));
                *first_chunk = false;
                chunk_w = 0;
            }
            chunk.push((ch, class));
            chunk_w += cw;
        }
        if !chunk.is_empty() {
            tokens.push((chunk, *first_chunk));
        }
    };
    for (ch, class) in chars {
        if ch.is_whitespace() {
            flush_word(&mut tokens, &mut word, &mut first_chunk);
            first_chunk = true;
        } else {
            word.push((ch, class));
        }
    }
    flush_word(&mut tokens, &mut word, &mut first_chunk);

    // Join placed tokens into runs of equal class, one space (Arg) where
    // a token had a space before it.
    let join_runs = |placed: &[(Vec<(char, Class)>, bool)]| -> Vec<(String, Class)> {
        let mut runs: Vec<(String, Class)> = Vec::new();
        let push_char =
            |ch: char, class: Class, runs: &mut Vec<(String, Class)>| match runs.last_mut() {
                Some((text, last_class)) if *last_class == class => text.push(ch),
                _ => runs.push((ch.to_string(), class)),
            };
        for (i, (token, space_before)) in placed.iter().enumerate() {
            if i > 0 && *space_before {
                push_char(' ', Class::Arg, &mut runs);
            }
            for (ch, class) in token {
                push_char(*ch, *class, &mut runs);
            }
        }
        runs
    };

    let token_width = |token: &[(char, Class)]| -> usize {
        token
            .iter()
            .map(|(ch, _)| unicode_width::UnicodeWidthChar::width(*ch).unwrap_or(0))
            .sum()
    };

    let mut lines: Vec<Vec<(String, Class)>> = Vec::new();
    let mut next = 0usize;
    while next < tokens.len() {
        let last_allowed = lines.len() + 1 == max_lines;
        let mut placed: Vec<(Vec<(char, Class)>, bool)> = Vec::new();
        let mut line_w = 0usize;
        while next < tokens.len() {
            let (token, space_before) = &tokens[next];
            let sep = if placed.is_empty() {
                0
            } else {
                usize::from(*space_before)
            };
            let token_w = token_width(token);
            if line_w + sep + token_w > max_width {
                break;
            }
            placed.push((token.clone(), sep == 1));
            line_w += sep + token_w;
            next += 1;
        }

        if next < tokens.len() && last_allowed {
            // Text remains past the final line: close this line with an
            // ellipsis at a word boundary. The ellipsis is always Arg.
            let finish = |placed: &mut Vec<(Vec<(char, Class)>, bool)>,
                          lines: &mut Vec<Vec<(String, Class)>>| {
                let mut runs = join_runs(placed);
                match runs.last_mut() {
                    Some((text, Class::Arg)) => text.push('\u{2026}'),
                    _ => runs.push(("\u{2026}".to_string(), Class::Arg)),
                }
                lines.push(runs);
            };
            if line_w < max_width {
                finish(&mut placed, &mut lines);
            } else if placed.len() > 1 {
                // The line filled exactly; give back the last whole word
                // to make room for the ellipsis.
                placed.pop();
                finish(&mut placed, &mut lines);
            } else {
                // One full-width word: hard-cut it with the ellipsis.
                let mut cut_chars: Vec<(char, Class)> = Vec::new();
                let mut cut_w = 0usize;
                for (ch, class) in &placed[0].0 {
                    let cw = unicode_width::UnicodeWidthChar::width(*ch).unwrap_or(0);
                    if cut_w + cw > max_width - 1 {
                        break;
                    }
                    cut_chars.push((*ch, *class));
                    cut_w += cw;
                }
                let mut runs = join_runs(&[(cut_chars, false)]);
                match runs.last_mut() {
                    Some((text, Class::Arg)) => text.push('\u{2026}'),
                    _ => runs.push(("\u{2026}".to_string(), Class::Arg)),
                }
                lines.push(runs);
            }
            break;
        }
        lines.push(join_runs(&placed));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    // ─── display_width ─────────────────────────────────────────────

    #[test]
    fn display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn display_width_cjk() {
        assert_eq!(display_width("日本語"), 6); // 3 chars × 2 columns
    }

    #[test]
    fn display_width_mixed() {
        assert_eq!(display_width("hi日本"), 6); // 2 + 2×2
    }

    #[test]
    fn display_width_empty() {
        assert_eq!(display_width(""), 0);
    }

    // ─── pad_to ────────────────────────────────────────────────────

    #[test]
    fn pad_to_normal() {
        let p = pad_to(3, 10);
        assert_eq!(p.len(), 7);
        assert_eq!(p, "       ");
    }

    #[test]
    fn pad_to_overflow() {
        let p = pad_to(10, 5);
        assert_eq!(p, ""); // saturating_sub returns 0
    }

    #[test]
    fn pad_to_exact() {
        let p = pad_to(5, 5);
        assert_eq!(p, "");
    }

    // ─── truncate_to_width ─────────────────────────────────────────

    #[test]
    fn truncate_to_width_zero_width() {
        assert_eq!(truncate_to_width("hello", 0), "");
    }

    #[test]
    fn truncate_to_width_exact() {
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    // ─── wrap_text ─────────────────────────────────────────────────

    #[test]
    fn wrap_text_short() {
        let lines = wrap_text("hi", 10, 3);
        assert_eq!(lines, vec!["hi"]);
    }

    #[test]
    fn wrap_text_exact_width() {
        let lines = wrap_text("abcde", 5, 3);
        assert_eq!(lines, vec!["abcde"]);
    }

    #[test]
    fn wrap_text_word_wrap() {
        let lines = wrap_text("hello world foo", 10, 3);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world foo");
    }

    #[test]
    fn wrap_text_truncation_with_ellipsis() {
        // Force truncation on last line
        let lines = wrap_text("aaa bbb ccc ddd eee fff", 10, 2);
        assert_eq!(lines.len(), 2);
        assert!(
            lines[1].contains('\u{2026}'),
            "last line should have ellipsis"
        );
    }

    #[test]
    fn wrap_text_truncation_ends_at_word_boundary() {
        // The truncated last line keeps whole words and appends the
        // ellipsis instead of cutting mid-word.
        let lines = wrap_text("aaa bbb ccc ddd eee fff ggg", 10, 2);
        assert_eq!(lines, vec!["aaa bbb", "ccc ddd\u{2026}"]);
    }

    #[test]
    fn wrap_text_truncation_gives_back_word_when_line_exactly_full() {
        // "cccc dd" fills the 7-wide last line exactly, so the last word is
        // given back to make room for the ellipsis rather than overflowing.
        let lines = wrap_text("aaa bbb cccc dd ee", 7, 2);
        assert_eq!(lines, vec!["aaa bbb", "cccc\u{2026}"]);
    }

    #[test]
    fn wrap_text_full_line_no_ellipsis_when_text_fits() {
        // A line that fills the width exactly with no text remaining gets
        // no ellipsis.
        let lines = wrap_text("aaa bbbb cccc", 10, 3);
        assert_eq!(lines, vec!["aaa bbbb", "cccc"]);
    }

    #[test]
    fn wrap_text_oversized_word_hard_splits() {
        // A single word wider than the line is split across lines.
        let lines = wrap_text("abcdefghij", 4, 3);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_text_oversized_word_truncates_with_ellipsis() {
        let lines = wrap_text("abcdefghij", 4, 2);
        assert_eq!(lines, vec!["abcd", "efg\u{2026}"]);
    }

    #[test]
    fn wrap_text_whitespace_only_text_has_no_lines() {
        assert!(wrap_text("   ", 10, 3).is_empty());
    }

    #[test]
    fn wrap_text_collapses_consecutive_spaces() {
        let lines = wrap_text("hello    world", 10, 3);
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn wrap_text_cjk() {
        // Each CJK char is 2 columns wide; width=6 fits 3 CJK chars
        let lines = wrap_text("あいうえお", 6, 3);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "あいう");
        assert_eq!(lines[1], "えお");
    }

    #[test]
    fn wrap_text_zero_width() {
        let lines = wrap_text("hello", 0, 3);
        assert!(lines.is_empty());
    }

    #[test]
    fn wrap_text_zero_lines() {
        let lines = wrap_text("hello", 10, 0);
        assert!(lines.is_empty());
    }

    // ─── wrap_classified ──────────────────────────────────────────

    use crate::shell_highlight::{Class, classify};

    /// The plain text of one classified line.
    fn line_text(line: &[(String, Class)]) -> String {
        line.iter().map(|(text, _)| text.as_str()).collect()
    }

    /// All (text, class) runs of one classified line.
    fn runs(line: &[(String, Class)]) -> &[(String, Class)] {
        line
    }

    #[test]
    fn wrap_classified_string_token_keeps_class_across_lines() {
        // The quoted string wraps (the space inside it is a split point);
        // both halves keep the Str class.
        let label = "echo \"one two\"";
        let classes = classify(label);
        let lines = wrap_classified(label, &classes, 9, 3);
        assert_eq!(lines.len(), 2);
        assert_eq!(line_text(&lines[0]), "echo \"one");
        assert_eq!(line_text(&lines[1]), "two\"");
        // Line 1: `echo` is Command, the string half `"one` is Str.
        assert_eq!(lines[0][0], ("echo".to_string(), Class::Command));
        assert_eq!(lines[0][1], (" ".to_string(), Class::Arg));
        assert_eq!(lines[0][2], ("\"one".to_string(), Class::Str));
        // Line 2: the wrapped half stays a string.
        assert_eq!(lines[1][0], ("two\"".to_string(), Class::Str));
    }

    #[test]
    fn wrap_classified_ellipsis_is_arg() {
        let label = "aaa bbb ccc ddd";
        let classes = classify(label);
        let lines = wrap_classified(label, &classes, 10, 1);
        assert_eq!(lines.len(), 1);
        assert_eq!(line_text(&lines[0]), "aaa bbb\u{2026}");
        // The ellipsis merges into the trailing Arg run (`aaa` is Command).
        let (text, class) = lines[0].last().unwrap();
        assert_eq!(*class, Class::Arg);
        assert!(text.ends_with('\u{2026}'));
        assert!(
            lines[0]
                .iter()
                .all(|(_, c)| *c == Class::Arg || *c == Class::Command)
        );
    }

    #[test]
    fn wrap_classified_hard_split_word_keeps_its_classes() {
        // One oversized word made of two classes, hard-split across lines:
        // every chunk keeps the class of its characters.
        let label = "aaabbbccc";
        let mut classes = vec![Class::Arg; 9];
        classes[3..6].fill(Class::Str);
        let lines = wrap_classified(label, &classes, 4, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(line_text(&lines[0]), "aaab");
        assert_eq!(line_text(&lines[1]), "bbcc");
        assert_eq!(line_text(&lines[2]), "c");
        assert_eq!(
            lines[0],
            vec![
                ("aaa".to_string(), Class::Arg),
                ("b".to_string(), Class::Str)
            ]
        );
        assert_eq!(
            lines[1],
            vec![
                ("bb".to_string(), Class::Str),
                ("cc".to_string(), Class::Arg)
            ]
        );
        assert!(lines[2].iter().all(|(_, c)| *c == Class::Arg));
    }

    #[test]
    fn wrap_classified_line_texts_equal_wrap_text() {
        let label = "cargo build --release 2>&1 | tee \"out log\"";
        let classes = classify(label);
        let classified = wrap_classified(label, &classes, 12, 3);
        let plain = wrap_text(label, 12, 3);
        assert_eq!(classified.len(), plain.len());
        for (line, text) in classified.iter().zip(&plain) {
            assert_eq!(line_text(line), text.as_str());
        }
    }

    #[test]
    fn wrap_classified_runs_split_on_class_changes() {
        let label = "echo -n hi";
        let classes = classify(label);
        let lines = wrap_classified(label, &classes, 40, 3);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            vec![
                ("echo".to_string(), Class::Command),
                (" ".to_string(), Class::Arg),
                ("-n".to_string(), Class::Flag),
                (" hi".to_string(), Class::Arg),
            ]
        );
    }

    #[test]
    fn wrap_classified_empty_and_zero_limits() {
        let classes = classify("");
        assert!(wrap_classified("", &classes, 10, 3).is_empty());
        assert!(wrap_classified("   ", &classes, 10, 3).is_empty());
        assert!(wrap_classified("hi", &classes, 0, 3).is_empty());
        assert!(wrap_classified("hi", &classes, 10, 0).is_empty());
    }

    // ─── elapsed_label ─────────────────────────────────────────────

    #[test]
    fn elapsed_label_none() {
        assert_eq!(elapsed_label(None, 0), "");
    }

    #[test]
    fn elapsed_label_zero() {
        assert_eq!(elapsed_label(Some(0), 0), "");
    }

    #[test]
    fn elapsed_label_future() {
        const TEST_NOW: u64 = 1_000_000;
        assert_eq!(elapsed_label(Some(TEST_NOW + 9999), TEST_NOW), "");
    }

    #[test]
    fn elapsed_label_seconds() {
        const TEST_NOW: u64 = 1_000_000;
        let label = elapsed_label(Some(TEST_NOW - 5), TEST_NOW);
        assert!(label.ends_with('s'));
        assert!(!label.contains('m'));
    }

    #[test]
    fn elapsed_label_minutes() {
        const TEST_NOW: u64 = 1_000_000;
        let label = elapsed_label(Some(TEST_NOW - 125), TEST_NOW);
        assert!(label.contains('m'));
        assert!(label.contains('s'));
        assert!(!label.contains('h'));
    }

    #[test]
    fn elapsed_label_hours() {
        const TEST_NOW: u64 = 1_000_000;
        let label = elapsed_label(Some(TEST_NOW - 3661), TEST_NOW);
        assert!(label.contains('h'));
        assert!(label.contains('m'));
    }

    // ─── wait_reason_label ─────────────────────────────────────────

    #[test]
    fn wait_reason_known() {
        assert_eq!(
            wait_reason_label("permission_prompt"),
            "permission required"
        );
        assert_eq!(wait_reason_label("idle_prompt"), "waiting for input");
        assert_eq!(wait_reason_label("auth_success"), "auth success");
        assert_eq!(
            wait_reason_label("elicitation_dialog"),
            "waiting for selection"
        );
        assert_eq!(wait_reason_label("rate_limit"), "rate limit");
    }

    #[test]
    fn wait_reason_unknown() {
        assert_eq!(wait_reason_label("something_else"), "something_else");
        assert!(wait_reason_entry("something_else").is_none());
    }

    // ─── wait_reason_label: pi's ui_prompt kinds ───────────────────

    #[test]
    fn wait_reason_pi_prompt_kinds_are_labelled() {
        assert_eq!(wait_reason_label("select"), "waiting for selection");
        assert_eq!(wait_reason_label("confirm"), "confirmation");
        assert_eq!(wait_reason_label("input"), "waiting for input");
        assert_eq!(wait_reason_label("editor"), "editor");
        assert_eq!(wait_reason_label("custom"), "prompt");
        assert_eq!(wait_reason_label("question"), "prompt");
    }

    #[test]
    fn wait_reason_pi_prompt_kinds_are_action_required() {
        for reason in ["select", "confirm", "input", "editor", "custom", "question"] {
            assert_eq!(
                wait_reason_entry(reason).map(|entry| entry.action_required),
                Some(true),
                "{reason} should be action-required"
            );
        }
    }

    #[test]
    fn wait_reason_empty() {
        assert_eq!(wait_reason_label(""), "");
    }

    // ─── wait_reason_label: permission_denied ──────────────────────

    #[test]
    fn wait_reason_permission_denied() {
        assert_eq!(wait_reason_label("permission_denied"), "permission denied");
    }

    // ─── wait_reason_label: session resume ─────────────────────────

    #[test]
    fn wait_reason_session_resumed() {
        assert_eq!(wait_reason_label("session_resumed"), "resumed");
        assert_eq!(
            wait_reason_label("session_resumed_compact"),
            "resumed (compact)"
        );
    }

    // ─── wait_reason_label: teammate_idle ──────────────────────────

    #[test]
    fn wait_reason_teammate_idle_without_reason() {
        assert_eq!(wait_reason_label("teammate_idle:alice"), "alice idle");
    }

    #[test]
    fn wait_reason_teammate_idle_with_reason() {
        assert_eq!(
            wait_reason_label("teammate_idle:alice:tokens_exhausted"),
            "alice idle (tokens_exhausted)"
        );
    }

    // ─── branch_label ──────────────────────────────────────────────

    #[test]
    fn branch_label_with_branch() {
        use crate::group::PaneGitInfo;
        let info = PaneGitInfo {
            repo_root: Some("/repo".into()),
            branch: Some("main".into()),
            is_worktree: false,
            worktree_name: None,
        };
        assert_eq!(branch_label(&info), "main");
    }

    #[test]
    fn branch_label_worktree() {
        use crate::group::PaneGitInfo;
        let info = PaneGitInfo {
            repo_root: Some("/repo".into()),
            branch: Some("fix/typo".into()),
            is_worktree: true,
            worktree_name: None,
        };
        assert_eq!(branch_label(&info), "+ fix/typo");
    }

    #[test]
    fn branch_label_no_git() {
        use crate::group::PaneGitInfo;
        let info = PaneGitInfo::default();
        assert_eq!(branch_label(&info), "");
    }

    #[test]
    fn branch_label_worktree_with_name() {
        let info = crate::group::PaneGitInfo {
            branch: Some("feat/auth".into()),
            is_worktree: true,
            worktree_name: Some("auth-wt".into()),
            ..Default::default()
        };
        assert_eq!(branch_label(&info), "+ auth-wt: feat/auth");
    }

    #[test]
    fn branch_label_worktree_name_same_as_branch() {
        let info = crate::group::PaneGitInfo {
            branch: Some("feat/auth".into()),
            is_worktree: true,
            worktree_name: Some("feat/auth".into()),
            ..Default::default()
        };
        assert_eq!(branch_label(&info), "+ feat/auth");
    }

    #[test]
    fn branch_label_worktree_no_name() {
        let info = crate::group::PaneGitInfo {
            branch: Some("main".into()),
            is_worktree: true,
            worktree_name: None,
            ..Default::default()
        };
        assert_eq!(branch_label(&info), "+ main");
    }
}
