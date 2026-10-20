use serde_json::{Map, Value};

use crate::event::{AgentEvent, AgentEventKind, EventAdapter};
use crate::tmux::PI_AGENT;
use crate::tool_name::CanonicalTool;

use super::{json_str, json_value_or_null, optional_str, parse_worktree};

/// The protocol events the pi adapter accepts, with the kind each one
/// produces and the fields it reads.
///
/// This table is the contract the `setup pi` output prints and the one the
/// producer of a pi session is written against. The drift test below keeps
/// it and the `parse` arms in step.
pub const PI_EVENTS: &[(&str, AgentEventKind, &[&str])] = &[
    (
        "session-start",
        AgentEventKind::SessionStart,
        &[
            "cwd",
            "session_id",
            "source",
            "permission_mode",
            "agent_id",
            "worktree",
        ],
    ),
    ("session-end", AgentEventKind::SessionEnd, &["end_reason"]),
    (
        "user-prompt-submit",
        AgentEventKind::UserPromptSubmit,
        &[
            "cwd",
            "prompt",
            "session_id",
            "permission_mode",
            "agent_id",
            "worktree",
        ],
    ),
    (
        "notification",
        AgentEventKind::Notification,
        &["notification_type", "wait_reason", "cwd", "session_id"],
    ),
    (
        "stop",
        AgentEventKind::Stop,
        &[
            "last_assistant_message",
            "cwd",
            "session_id",
            "permission_mode",
        ],
    ),
    (
        "stop-failure",
        AgentEventKind::StopFailure,
        &["error", "error_type", "error_message", "cwd", "session_id"],
    ),
    (
        "activity-log",
        AgentEventKind::ActivityLog,
        &["tool_name", "tool_input", "tool_response"],
    ),
];

/// pi's tool names are lowercase (`read`, `edit`, …) but the internal label
/// extractor in `src/cli/label.rs` keys off Claude-style PascalCase names.
/// Normalise here so the activity log and its strategy table share a single
/// vocabulary across agents.
fn normalize_tool_name(raw: &str) -> String {
    let canonical = match raw {
        "read" => CanonicalTool::Read,
        "edit" | "multiedit" => CanonicalTool::Edit,
        "write" => CanonicalTool::Write,
        "bash" => CanonicalTool::Bash,
        "powershell" => CanonicalTool::PowerShell,
        "grep" => CanonicalTool::Grep,
        "find" | "ls" => CanonicalTool::Glob,
        other => return other.to_string(),
    };
    canonical.as_str().to_string()
}

/// pi names the file a tool works on `path`; the label extractor reads
/// `file_path`. Keys are added alongside the originals rather than replacing
/// them so downstream consumers that want the raw payload still see it.
fn normalize_tool_input(tool_name: &str, input: Value) -> Value {
    let Value::Object(mut map) = input else {
        return input;
    };
    if matches!(tool_name, "Read" | "Edit" | "Write") {
        copy_key(&mut map, "path", "file_path");
    }
    Value::Object(map)
}

fn copy_key(map: &mut Map<String, Value>, src: &str, dst: &str) {
    if map.contains_key(dst) {
        return;
    }
    if let Some(value) = map.get(src).cloned() {
        map.insert(dst.to_string(), value);
    }
}

/// The reason a pi session waits. `notification_type` is the name the
/// protocol gives; `wait_reason` is accepted as an alias, because that is
/// the name the OpenCode producer already uses.
fn notification_reason(input: &Value) -> &str {
    let reason = json_str(input, "notification_type");
    if reason.is_empty() {
        json_str(input, "wait_reason")
    } else {
        reason
    }
}

/// The error text of a failed turn. The protocol names it `error`; the
/// Claude-shaped names are accepted as alternatives.
fn failure_error(input: &Value) -> &str {
    for key in ["error", "error_type", "error_message", "error_details"] {
        let value = json_str(input, key);
        if !value.is_empty() {
            return value;
        }
    }
    ""
}

pub struct PiAdapter;

impl EventAdapter for PiAdapter {
    fn parse(&self, event_name: &str, input: &Value) -> Option<AgentEvent> {
        match event_name {
            "session-start" => Some(AgentEvent::SessionStart {
                agent: PI_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: json_str(input, "permission_mode").into(),
                source: json_str(input, "source").into(),
                worktree: parse_worktree(input),
                agent_id: optional_str(input, "agent_id"),
                session_id: optional_str(input, "session_id"),
            }),
            "session-end" => Some(AgentEvent::SessionEnd {
                end_reason: json_str(input, "end_reason").into(),
            }),
            "user-prompt-submit" => Some(AgentEvent::UserPromptSubmit {
                agent: PI_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: json_str(input, "permission_mode").into(),
                prompt: json_str(input, "prompt").into(),
                worktree: parse_worktree(input),
                agent_id: optional_str(input, "agent_id"),
                session_id: optional_str(input, "session_id"),
            }),
            "notification" => {
                let wait_reason = notification_reason(input);
                let meta_only = wait_reason == "idle_prompt";
                Some(AgentEvent::Notification {
                    agent: PI_AGENT.into(),
                    cwd: json_str(input, "cwd").into(),
                    permission_mode: json_str(input, "permission_mode").into(),
                    wait_reason: wait_reason.into(),
                    meta_only,
                    worktree: parse_worktree(input),
                    agent_id: optional_str(input, "agent_id"),
                    session_id: optional_str(input, "session_id"),
                })
            }
            "stop" => Some(AgentEvent::Stop {
                agent: PI_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: json_str(input, "permission_mode").into(),
                last_message: json_str(input, "last_assistant_message").into(),
                response: None,
                worktree: parse_worktree(input),
                agent_id: optional_str(input, "agent_id"),
                session_id: optional_str(input, "session_id"),
            }),
            "stop-failure" => Some(AgentEvent::StopFailure {
                agent: PI_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: json_str(input, "permission_mode").into(),
                error: failure_error(input).into(),
                worktree: parse_worktree(input),
                agent_id: optional_str(input, "agent_id"),
                session_id: optional_str(input, "session_id"),
            }),
            "activity-log" => {
                let raw_name = json_str(input, "tool_name");
                if raw_name.is_empty() {
                    return None;
                }
                let tool_name = normalize_tool_name(raw_name);
                let tool_input =
                    normalize_tool_input(&tool_name, json_value_or_null(input, "tool_input"));
                Some(AgentEvent::ActivityLog {
                    tool_name,
                    tool_input,
                    tool_response: json_value_or_null(input, "tool_response"),
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_start() {
        let adapter = PiAdapter;
        let event = adapter
            .parse(
                "session-start",
                &json!({"cwd": "/tmp", "session_id": "ses-1", "source": "startup"}),
            )
            .unwrap();
        assert_eq!(
            event,
            AgentEvent::SessionStart {
                agent: PI_AGENT.into(),
                cwd: "/tmp".into(),
                permission_mode: "".into(),
                source: "startup".into(),
                worktree: None,
                agent_id: None,
                session_id: Some("ses-1".into()),
            }
        );
    }

    #[test]
    fn session_start_reads_the_worktree() {
        let adapter = PiAdapter;
        let event = adapter
            .parse(
                "session-start",
                &json!({
                    "cwd": "/tmp",
                    "worktree": {"name": "feature", "path": "/tmp/feature", "branch": "feat/x"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::SessionStart { worktree, .. } => {
                let worktree = worktree.expect("worktree is read");
                assert_eq!(worktree.name, "feature");
                assert_eq!(worktree.branch, "feat/x");
            }
            other => panic!("expected SessionStart, got {:?}", other),
        }
    }

    #[test]
    fn user_prompt_submit() {
        let adapter = PiAdapter;
        let event = adapter
            .parse(
                "user-prompt-submit",
                &json!({"cwd": "/tmp", "prompt": "hello"}),
            )
            .unwrap();
        assert_eq!(
            event,
            AgentEvent::UserPromptSubmit {
                agent: PI_AGENT.into(),
                cwd: "/tmp".into(),
                permission_mode: "".into(),
                prompt: "hello".into(),
                worktree: None,
                agent_id: None,
                session_id: None,
            }
        );
    }

    #[test]
    fn notification_reads_the_protocol_name() {
        let adapter = PiAdapter;
        let event = adapter
            .parse("notification", &json!({"notification_type": "permission"}))
            .unwrap();
        match event {
            AgentEvent::Notification {
                wait_reason,
                meta_only,
                ..
            } => {
                assert_eq!(wait_reason, "permission");
                assert!(!meta_only);
            }
            other => panic!("expected Notification, got {:?}", other),
        }
    }

    #[test]
    fn notification_accepts_the_alias() {
        let adapter = PiAdapter;
        let event = adapter
            .parse("notification", &json!({"wait_reason": "permission"}))
            .unwrap();
        match event {
            AgentEvent::Notification { wait_reason, .. } => assert_eq!(wait_reason, "permission"),
            other => panic!("expected Notification, got {:?}", other),
        }
    }

    #[test]
    fn stop_reads_the_last_assistant_message() {
        let adapter = PiAdapter;
        let event = adapter
            .parse("stop", &json!({"last_assistant_message": "done"}))
            .unwrap();
        match event {
            AgentEvent::Stop { last_message, .. } => assert_eq!(last_message, "done"),
            other => panic!("expected Stop, got {:?}", other),
        }
    }

    #[test]
    fn stop_failure_reads_the_error() {
        let adapter = PiAdapter;
        let event = adapter
            .parse("stop-failure", &json!({"error": "provider exploded"}))
            .unwrap();
        match event {
            AgentEvent::StopFailure { error, .. } => assert_eq!(error, "provider exploded"),
            other => panic!("expected StopFailure, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_normalizes_a_read_tool() {
        let adapter = PiAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "read",
                    "tool_input": {"path": "/home/user/project/src/main.rs"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                ..
            } => {
                assert_eq!(tool_name, "Read");
                assert_eq!(tool_input["file_path"], "/home/user/project/src/main.rs");
                assert_eq!(tool_input["path"], "/home/user/project/src/main.rs");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_normalizes_a_bash_tool() {
        let adapter = PiAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({"tool_name": "bash", "tool_input": {"command": "ls"}}),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                ..
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "ls");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_keeps_an_unknown_tool_name() {
        let adapter = PiAdapter;
        let event = adapter
            .parse("activity-log", &json!({"tool_name": "mcp__server__do"}))
            .unwrap();
        match event {
            AgentEvent::ActivityLog { tool_name, .. } => assert_eq!(tool_name, "mcp__server__do"),
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_without_a_tool_name_is_ignored() {
        let adapter = PiAdapter;
        assert!(adapter.parse("activity-log", &json!({})).is_none());
    }

    #[test]
    fn an_unknown_event_is_ignored() {
        let adapter = PiAdapter;
        assert!(adapter.parse("teleport", &json!({})).is_none());
    }

    // ─── the table and the parse arms agree ──────────────────────────

    #[test]
    fn every_table_event_parses_to_its_kind() {
        let adapter = PiAdapter;
        for (event_name, kind, fields) in PI_EVENTS {
            let mut payload = serde_json::Map::new();
            for field in *fields {
                payload.insert((*field).to_string(), Value::String("x".into()));
            }
            if *kind == AgentEventKind::ActivityLog {
                payload.insert("tool_name".into(), Value::String("read".into()));
            }
            let produced = adapter
                .parse(event_name, &Value::Object(payload))
                .unwrap_or_else(|| panic!("pi: parse() returned None for {:?}", event_name));
            assert_eq!(
                produced.kind(),
                *kind,
                "pi: {:?} produced the wrong kind",
                event_name
            );
        }
    }

    #[test]
    fn no_event_outside_the_table_is_accepted() {
        let adapter = PiAdapter;
        for kind in AgentEventKind::ALL {
            let name = kind.external_name();
            let in_table = PI_EVENTS.iter().any(|(event, _, _)| *event == name);
            let accepted = adapter
                .parse(name, &super::super::minimal_payload(*kind))
                .is_some();
            assert_eq!(
                accepted,
                in_table,
                "pi: {:?} is accepted by parse() but {} the table",
                name,
                if in_table { "is in" } else { "is not in" }
            );
        }
    }
}
