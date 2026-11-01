use serde_json::{Map, Value};

use crate::event::{AgentEvent, EventAdapter, Reported};
use crate::tmux::OPENCODE_AGENT;
use crate::tool_name::CanonicalTool;

use super::{json_str, json_value_or_null, optional_str, session_context_reported};

pub struct OpenCodeAdapter;

/// OpenCode tool IDs are lowercase (`bash`, `read`, …) but the internal
/// label extractor in `src/cli/label.rs` keys off Claude-style PascalCase
/// names. Normalise here so the activity log and its strategy table share
/// a single vocabulary across agents.
fn normalize_tool_name(raw: &str) -> String {
    let canonical = match raw {
        "bash" | "shell" => CanonicalTool::Bash,
        "read" => CanonicalTool::Read,
        "write" => CanonicalTool::Write,
        "edit" | "multiedit" => CanonicalTool::Edit,
        "glob" => CanonicalTool::Glob,
        "grep" => CanonicalTool::Grep,
        "webfetch" => CanonicalTool::WebFetch,
        "websearch" => CanonicalTool::WebSearch,
        "task" => CanonicalTool::Agent,
        "skill" => CanonicalTool::Skill,
        "lsp" => CanonicalTool::Lsp,
        "todowrite" => CanonicalTool::TodoWrite,
        other => return other.to_string(),
    };
    canonical.as_str().to_string()
}

/// Translate OpenCode's camelCase tool arguments into the snake_case keys
/// the Claude-style label extractor expects. Keys are added alongside the
/// originals rather than replacing them so downstream consumers that want
/// the raw payload still see it.
fn normalize_tool_input(tool_name: &str, input: Value) -> Value {
    let Value::Object(mut map) = input else {
        return input;
    };
    let rewrites: &[(&str, &str)] = match tool_name {
        "Read" | "Write" | "Edit" => &[("filePath", "file_path")],
        _ => &[],
    };
    copy_keys(&mut map, rewrites);
    Value::Object(map)
}

fn copy_keys(map: &mut Map<String, Value>, pairs: &[(&str, &str)]) {
    for (src, dst) in pairs {
        if map.contains_key(*dst) {
            continue;
        }
        if let Some(value) = map.get(*src).cloned() {
            map.insert((*dst).to_string(), value);
        }
    }
}

impl EventAdapter for OpenCodeAdapter {
    fn reported(&self, event_name: &str, input: &Value) -> Reported {
        // Every event OpenCode sends apart from `activity-log` carries
        // session context.
        match event_name {
            "session-start" | "user-prompt-submit" | "notification" | "stop" | "stop-failure" => {
                session_context_reported(input)
            }
            _ => Reported::NONE,
        }
    }

    fn parse(&self, event_name: &str, input: &Value) -> Option<AgentEvent> {
        match event_name {
            "session-start" => Some(AgentEvent::SessionStart {
                agent: OPENCODE_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: String::new(),
                source: json_str(input, "source").into(),
                worktree: None,
                agent_id: None,
                session_id: optional_str(input, "session_id"),
            }),
            "user-prompt-submit" => Some(AgentEvent::UserPromptSubmit {
                agent: OPENCODE_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: String::new(),
                prompt: json_str(input, "prompt").into(),
                worktree: None,
                agent_id: None,
                session_id: optional_str(input, "session_id"),
                pmux_task: None,
            }),
            "notification" => Some(AgentEvent::Notification {
                agent: OPENCODE_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: String::new(),
                wait_reason: json_str(input, "wait_reason").into(),
                meta_only: false,
                worktree: None,
                agent_id: None,
                session_id: optional_str(input, "session_id"),
            }),
            "stop" => Some(AgentEvent::Stop {
                agent: OPENCODE_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: String::new(),
                last_message: json_str(input, "last_message").into(),
                response: None,
                worktree: None,
                agent_id: None,
                session_id: optional_str(input, "session_id"),
            }),
            "stop-failure" => Some(AgentEvent::StopFailure {
                agent: OPENCODE_AGENT.into(),
                cwd: json_str(input, "cwd").into(),
                permission_mode: String::new(),
                error: json_str(input, "error").into(),
                worktree: None,
                agent_id: None,
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
    fn reported_matches_the_parse_arms() {
        // OpenCode's session-context arms hardcode `worktree: None`, so the
        // worktree leg of the pin does not apply.
        super::super::assert_reported_matches_parse(
            "opencode",
            false,
            Some(super::super::SESSION_CONTEXT_FIELDS),
        );
    }

    #[test]
    fn session_start() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "session-start",
                &json!({"cwd": "/tmp", "session_id": "ses-1", "source": "startup"}),
            )
            .unwrap();
        assert_eq!(
            event,
            AgentEvent::SessionStart {
                agent: OPENCODE_AGENT.into(),
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
    fn user_prompt_submit() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "user-prompt-submit",
                &json!({"cwd": "/tmp", "prompt": "hello"}),
            )
            .unwrap();
        assert_eq!(
            event,
            AgentEvent::UserPromptSubmit {
                agent: OPENCODE_AGENT.into(),
                cwd: "/tmp".into(),
                permission_mode: "".into(),
                prompt: "hello".into(),
                worktree: None,
                agent_id: None,
                session_id: None,
                pmux_task: None,
            }
        );
    }

    #[test]
    fn activity_log_normalizes_lowercase_bash() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "bash",
                    "tool_input": {"command": "ls"},
                    "tool_response": {"stdout": "file.txt"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                tool_response,
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "ls");
                assert_eq!(tool_response["stdout"], "file.txt");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_normalizes_shell_tool_to_bash() {
        // OpenCode 2 ships both `bash` and `shell` (its persistent shell
        // tool); both are command execution for the sidebar.
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "shell",
                    "tool_input": {"command": "ls -la", "workdir": "/tmp"},
                    "tool_response": {"output": "file.txt"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                ..
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "ls -la");
                assert_eq!(tool_input["workdir"], "/tmp");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_preserves_bash_background_flag() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "bash",
                    "tool_input": {
                        "command": "npm run dev",
                        "runInBackground": true
                    }
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                ..
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_input["command"], "npm run dev");
                assert_eq!(tool_input["runInBackground"], true);
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_normalizes_read_filepath_key() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "read",
                    "tool_input": {"filePath": "/home/user/src/main.rs"}
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
                assert_eq!(tool_input["file_path"], "/home/user/src/main.rs");
                assert_eq!(tool_input["filePath"], "/home/user/src/main.rs");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_unknown_tool_passes_through() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "custom-mcp-tool",
                    "tool_input": {"foo": "bar"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog { tool_name, .. } => {
                assert_eq!(tool_name, "custom-mcp-tool");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn activity_log_multiedit_maps_to_edit() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "activity-log",
                &json!({
                    "tool_name": "multiedit",
                    "tool_input": {"filePath": "/a/b.rs"}
                }),
            )
            .unwrap();
        match event {
            AgentEvent::ActivityLog {
                tool_name,
                tool_input,
                ..
            } => {
                assert_eq!(tool_name, "Edit");
                assert_eq!(tool_input["file_path"], "/a/b.rs");
            }
            other => panic!("expected ActivityLog, got {:?}", other),
        }
    }

    #[test]
    fn stop_failure() {
        let adapter = OpenCodeAdapter;
        let event = adapter
            .parse(
                "stop-failure",
                &json!({"cwd": "/tmp", "error": "boom", "session_id": "ses-1"}),
            )
            .unwrap();
        assert_eq!(
            event,
            AgentEvent::StopFailure {
                agent: OPENCODE_AGENT.into(),
                cwd: "/tmp".into(),
                permission_mode: "".into(),
                error: "boom".into(),
                worktree: None,
                agent_id: None,
                session_id: Some("ses-1".into()),
            }
        );
    }

    // ─── the reported set per event ──────────────────────────────

    #[test]
    fn the_session_context_events_report_the_session_fields() {
        let adapter = OpenCodeAdapter;
        for event_name in [
            "session-start",
            "user-prompt-submit",
            "notification",
            "stop",
            "stop-failure",
        ] {
            let reported = adapter.reported(event_name, &json!({}));
            assert_eq!(
                reported.fields(),
                &["session_id", "cwd", "worktree"],
                "{event_name} should report the session-context fields"
            );
        }
    }

    #[test]
    fn the_events_without_session_context_report_nothing() {
        let adapter = OpenCodeAdapter;
        for event_name in [
            "session-end",
            "activity-log",
            "permission-denied",
            "cwd-changed",
            "subagent-start",
            "subagent-stop",
            "task-created",
            "task-completed",
            "teammate-idle",
            "worktree-create",
            "worktree-remove",
        ] {
            let reported = adapter.reported(event_name, &json!({}));
            assert!(reported.is_empty(), "{event_name} should report nothing");
        }
    }

    #[test]
    fn an_invalid_payload_reports_nothing() {
        let adapter = OpenCodeAdapter;
        // A payload that is not valid JSON reaches the adapter as `Null`;
        // it reports nothing, so it clears nothing.
        let invalid = serde_json::from_str::<serde_json::Value>("{\"cwd\": ")
            .unwrap_or(serde_json::Value::Null);
        assert!(invalid.is_null());
        for event_name in ["session-start", "stop", "stop-failure"] {
            assert_eq!(
                adapter.reported(event_name, &invalid),
                Reported::NONE,
                "{event_name} reported something for an invalid payload"
            );
        }
    }
}
