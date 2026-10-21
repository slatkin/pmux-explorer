use unicode_width::UnicodeWidthStr;

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

pub fn wait_reason_label(reason: &str) -> String {
    match reason {
        "permission_prompt" => "permission required".into(),
        "idle_prompt" => "waiting for input".into(),
        "auth_success" => "auth success".into(),
        "elicitation_dialog" => "waiting for selection".into(),
        "rate_limit" => "rate limit".into(),
        "permission_denied" => "permission denied".into(),
        "session_resumed" => "resumed".into(),
        "session_resumed_compact" => "resumed (compact)".into(),
        _ => {
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
    if max_width == 0 || max_lines == 0 {
        return Vec::new();
    }

    // Tokens keep words whole: a word wider than the line is pre-split
    // into width-fitting chunks, and every chunk after the first continues
    // the word with no space before it.
    let mut tokens: Vec<(String, bool)> = Vec::new(); // (text, space before)
    for word in text.split_whitespace() {
        let mut first_chunk = true;
        let mut chunk = String::new();
        let mut chunk_w = 0usize;
        for ch in word.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if chunk_w + cw > max_width {
                tokens.push((std::mem::take(&mut chunk), first_chunk));
                first_chunk = false;
                chunk_w = 0;
            }
            chunk.push(ch);
            chunk_w += cw;
        }
        if !chunk.is_empty() {
            tokens.push((chunk, first_chunk));
        }
    }

    let join = |tokens: &[(String, bool)]| -> String {
        let mut line = String::new();
        for (i, (token, space_before)) in tokens.iter().enumerate() {
            if i > 0 && *space_before {
                line.push(' ');
            }
            line.push_str(token);
        }
        line
    };

    let mut lines: Vec<String> = Vec::new();
    let mut next = 0usize;
    while next < tokens.len() {
        let last_allowed = lines.len() + 1 == max_lines;
        let mut placed: Vec<(String, bool)> = Vec::new();
        let mut line_w = 0usize;
        while next < tokens.len() {
            let (token, space_before) = &tokens[next];
            let sep = if placed.is_empty() {
                0
            } else {
                usize::from(*space_before)
            };
            let token_w = display_width(token);
            if line_w + sep + token_w > max_width {
                break;
            }
            placed.push((token.clone(), sep == 1));
            line_w += sep + token_w;
            next += 1;
        }

        if next < tokens.len() && last_allowed {
            // Text remains past the final line: close this line with an
            // ellipsis at a word boundary.
            if line_w < max_width {
                lines.push(format!("{}\u{2026}", join(&placed)));
            } else if placed.len() > 1 {
                // The line filled exactly; give back the last whole word
                // to make room for the ellipsis.
                placed.pop();
                lines.push(format!("{}\u{2026}", join(&placed)));
            } else {
                // One full-width word: hard-cut it with the ellipsis.
                let mut cut = String::new();
                let mut cut_w = 0usize;
                for ch in placed[0].0.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if cut_w + cw > max_width - 1 {
                        break;
                    }
                    cut.push(ch);
                    cut_w += cw;
                }
                lines.push(format!("{cut}\u{2026}"));
            }
            break;
        }
        lines.push(join(&placed));
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
