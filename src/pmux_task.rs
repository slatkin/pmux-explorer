//! Parser for the pmux task prompt that pmux sends to a pi subagent as its
//! first user prompt.
//!
//! The prompt starts with `<file name="PATH">`, where PATH has the shape
//! `/artifacts/<id>/context/<stem>.md` for a spawned subagent or
//! `/artifacts/<id>/pmux-resume/<stem>.md` for a resumed one, and `<stem>` is
//! a hyphenated name plus a `DDDD-DD-DDTDD-DD-DD` timestamp. Only the `name`
//! attribute is read; the text inside the tag is never inspected. Anything
//! that does not match this shape returns `None`, so the caller stores the
//! prompt as before.

/// The length of the pmux timestamp suffix, `DDDD-DD-DDTDD-DD-DD`.
const TIMESTAMP_LEN: usize = 19;

/// Whether a recognised pmux task prompt started a new pi session or resumed
/// an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmuxTaskKind {
    Spawned,
    Resumed,
}

/// The subagent identity parsed from a recognised pmux task prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmuxTask {
    pub kind: PmuxTaskKind,
    pub parent_session_id: String,
    pub task: Option<String>,
}

/// The stem pmux writes when a subagent has no usable name.
fn fallback_stem(kind: PmuxTaskKind) -> &'static str {
    match kind {
        PmuxTaskKind::Spawned => "subagent",
        PmuxTaskKind::Resumed => "resume",
    }
}

/// Parses a pi user prompt into a subagent identity, or `None` when the prompt
/// is not a recognised pmux task prompt.
pub fn parse_pmux_task(prompt: &str) -> Option<PmuxTask> {
    let rest = prompt.strip_prefix("<file name=\"")?;
    let path = &rest[..rest.find('"')?];

    let (id, folder, stem) = split_path(path)?;
    let kind = match folder {
        "context" => PmuxTaskKind::Spawned,
        "pmux-resume" => PmuxTaskKind::Resumed,
        _ => return None,
    };

    let name = strip_timestamp(stem)?;
    let task = if name == fallback_stem(kind) {
        None
    } else {
        Some(name.replace('-', " "))
    };

    Some(PmuxTask {
        kind,
        parent_session_id: id.to_string(),
        task,
    })
}

/// Splits a pmux artifact path into its session id, folder and stem. The path
/// must end in `.md` and its last four segments must be `artifacts`, a
/// non-empty session id, the folder and the stem.
fn split_path(path: &str) -> Option<(&str, &str, &str)> {
    let stem = path.strip_suffix(".md")?;
    let mut segments = stem.rsplit('/');
    let stem = segments.next()?;
    let folder = segments.next()?;
    let id = segments.next()?;
    if id.is_empty() || segments.next() != Some("artifacts") {
        return None;
    }
    Some((id, folder, stem))
}

/// Splits a stem into its name and its hyphen-plus-timestamp suffix. The name
/// must be non-empty and the suffix must be a well-formed timestamp.
fn strip_timestamp(stem: &str) -> Option<&str> {
    let name_len = stem.len().checked_sub(TIMESTAMP_LEN + 1)?;
    if name_len == 0 {
        return None;
    }
    let (name, suffix) = stem.split_at(name_len);
    if !is_timestamp(&suffix[1..]) {
        return None;
    }
    Some(name)
}

/// Checks that `s` is exactly 19 characters of the shape
/// `DDDD-DD-DDTDD-DD-DD`.
fn is_timestamp(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != TIMESTAMP_LEN {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        let expected = match index {
            4 | 7 | 13 | 16 => b'-',
            10 => b'T',
            _ => {
                if !byte.is_ascii_digit() {
                    return false;
                }
                continue;
            }
        };
        if *byte != expected {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_prompt(stem: &str, id: &str) -> String {
        format!("<file name=\"/s/artifacts/{id}/context/{stem}.md\">")
    }

    fn resume_prompt(stem: &str, id: &str) -> String {
        format!("<file name=\"/s/artifacts/{id}/pmux-resume/{stem}.md\">")
    }

    #[test]
    fn spawn_path_with_hyphenated_name() {
        let task = parse_pmux_task(&spawn_prompt(
            "unit1-idle-collapse-2026-09-07T15-18-30",
            "P1",
        ))
        .unwrap();
        assert_eq!(task.kind, PmuxTaskKind::Spawned);
        assert_eq!(task.parent_session_id, "P1");
        assert_eq!(task.task.as_deref(), Some("unit1 idle collapse"));
    }

    #[test]
    fn resume_path_with_hyphenated_name() {
        let task =
            parse_pmux_task(&resume_prompt("u1-placement-2026-10-21T05-41-15", "P1")).unwrap();
        assert_eq!(task.kind, PmuxTaskKind::Resumed);
        assert_eq!(task.parent_session_id, "P1");
        assert_eq!(task.task.as_deref(), Some("u1 placement"));
    }

    #[test]
    fn hyphenated_name_becomes_spaced_task() {
        let task =
            parse_pmux_task(&spawn_prompt("review-unit1-2026-09-07T15-26-50", "P1")).unwrap();
        assert_eq!(task.task.as_deref(), Some("review unit1"));
    }

    #[test]
    fn spawn_fallback_stem_has_no_task() {
        let task = parse_pmux_task(&spawn_prompt("subagent-2026-09-07T15-26-50", "P1")).unwrap();
        assert_eq!(task.kind, PmuxTaskKind::Spawned);
        assert_eq!(task.task, None);
    }

    #[test]
    fn resume_fallback_stem_has_no_task() {
        let task = parse_pmux_task(&resume_prompt("resume-2026-10-21T05-41-15", "P1")).unwrap();
        assert_eq!(task.kind, PmuxTaskKind::Resumed);
        assert_eq!(task.task, None);
    }

    #[test]
    fn non_pmux_file_path_is_rejected() {
        assert_eq!(parse_pmux_task("<file name=\"/home/me/notes.md\">"), None);
    }

    #[test]
    fn stem_without_timestamp_is_rejected() {
        assert_eq!(
            parse_pmux_task(&spawn_prompt("unit1-idle-collapse", "P1")),
            None
        );
    }

    #[test]
    fn malformed_timestamp_is_rejected() {
        // Wrong separators.
        assert_eq!(
            parse_pmux_task(&spawn_prompt("unit1-2026/09/07T15-18-30", "P1")),
            None
        );
        // Non-digit field.
        assert_eq!(
            parse_pmux_task(&spawn_prompt("unit1-2026-09-0XT15-18-30", "P1")),
            None
        );
        // Wrong separator before the time.
        assert_eq!(
            parse_pmux_task(&spawn_prompt("unit1-2026-09-07X15-18-30", "P1")),
            None
        );
    }

    #[test]
    fn empty_id_is_rejected() {
        assert_eq!(
            parse_pmux_task("<file name=\"/s/artifacts//context/unit1-2026-09-07T15-18-30.md\">"),
            None
        );
    }

    #[test]
    fn tag_not_at_start_is_rejected() {
        assert_eq!(
            parse_pmux_task(&format!(
                "please look at {}",
                spawn_prompt("unit1-idle-collapse-2026-09-07T15-18-30", "P1")
            )),
            None
        );
    }

    #[test]
    fn image_style_tag_matches_and_body_is_ignored() {
        let prompt = format!(
            "{}</file>",
            spawn_prompt("unit1-idle-collapse-2026-09-07T15-18-30", "P1")
        );
        let task = parse_pmux_task(&prompt).unwrap();
        assert_eq!(task.kind, PmuxTaskKind::Spawned);
        assert_eq!(task.task.as_deref(), Some("unit1 idle collapse"));
    }

    #[test]
    fn unterminated_tag_is_rejected() {
        assert_eq!(
            parse_pmux_task("<file name=\"/s/artifacts/P1/context/unit1-2026-09-07T15-18-30.md>"),
            None
        );
    }

    #[test]
    fn path_without_artifacts_segment_is_rejected() {
        assert_eq!(
            parse_pmux_task("<file name=\"/s/other/P1/context/unit1-2026-09-07T15-18-30.md\">"),
            None
        );
    }
}
