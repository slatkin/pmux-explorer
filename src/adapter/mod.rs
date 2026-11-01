pub mod claude;
pub mod codex;
pub mod opencode;
pub mod pi;

use crate::event::{AgentEventKind, Reported};

pub(crate) fn json_str<'a>(val: &'a serde_json::Value, key: &str) -> &'a str {
    val.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// The fields the Claude-shaped adapters (claude, codex, opencode) report on
/// every event that carries session context, as those agents do today.
static SESSION_CONTEXT_FIELDS: &[&str] = &["session_id", "cwd", "worktree"];

/// The reported set of the declared `fields`, or nothing when the payload is
/// not valid JSON. An invalid payload arrives as `Null` and reports nothing,
/// so it clears nothing.
pub(crate) fn reported_fields(
    fields: &'static [&'static str],
    input: &serde_json::Value,
) -> Reported {
    if input.is_null() {
        Reported::NONE
    } else {
        Reported::new(fields)
    }
}

/// The reported set of an event that carries session context for one of the
/// Claude-shaped adapters.
pub(crate) fn session_context_reported(input: &serde_json::Value) -> Reported {
    reported_fields(SESSION_CONTEXT_FIELDS, input)
}

/// Parse the optional worktree object of a payload.
/// Returns None if the "worktree" field is missing or not an object.
pub(crate) fn parse_worktree(input: &serde_json::Value) -> Option<crate::event::WorktreeInfo> {
    use crate::event::WorktreeInfo;
    let obj = input.get("worktree")?;
    if !obj.is_object() {
        return None;
    }
    let name = json_str(obj, "name");
    let path = json_str(obj, "path");
    let branch = json_str(obj, "branch");
    let original = json_str(obj, "originalRepoDir");
    if name.is_empty() && path.is_empty() && branch.is_empty() && original.is_empty() {
        return None;
    }
    Some(WorktreeInfo {
        name: name.into(),
        path: path.into(),
        branch: branch.into(),
        original_repo_dir: original.into(),
    })
}

pub(crate) fn optional_str(val: &serde_json::Value, key: &str) -> Option<String> {
    let s = json_str(val, key);
    if s.is_empty() { None } else { Some(s.into()) }
}

pub(crate) fn json_value_or_null(val: &serde_json::Value, key: &str) -> serde_json::Value {
    val.get(key).cloned().unwrap_or(serde_json::Value::Null)
}

/// Binding between an upstream agent-side hook trigger (as it appears in the
/// agent's `settings.json`) and the internal `AgentEventKind` the sidebar
/// produces once the hook fires.
///
/// Each adapter exposes its full `HOOK_REGISTRATIONS` table so install
/// wizards, README snippets, setup commands, and docs can all be generated
/// from a single source of truth. The `kind` field is a compile-time enum,
/// not a string, so typos cannot creep in. Drift between the table and the
/// adapter's `parse()` match arms is caught by the tests in `claude.rs` /
/// `codex.rs` via [`assert_table_drift_free`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookRegistration {
    /// Trigger name in the agent's settings.json (e.g. `"SessionStart"`,
    /// `"PostToolUse"`).
    pub trigger: &'static str,
    /// Optional matcher value. `None` means "register with an empty matcher"
    /// (catches all). `Some("startup|resume")` etc. captures a specific filter.
    pub matcher: Option<&'static str>,
    /// Internal event this registration produces.
    pub kind: AgentEventKind,
}

#[cfg(test)]
pub(crate) fn minimal_payload(kind: AgentEventKind) -> serde_json::Value {
    use serde_json::json;
    match kind {
        AgentEventKind::ActivityLog => json!({"tool_name": "Read"}),
        AgentEventKind::SubagentStart | AgentEventKind::SubagentStop => {
            json!({"agent_type": "Explore"})
        }
        _ => json!({}),
    }
}

#[cfg(test)]
pub(crate) fn assert_table_drift_free(agent: &str, table: &[HookRegistration]) {
    use crate::event::resolve_adapter;
    let adapter = resolve_adapter(agent).expect("adapter should exist");

    // Table → parse: every registration must be accepted by `parse()` and
    // produce an `AgentEvent` whose kind matches the registration.
    for reg in table {
        let event_name = reg.kind.external_name();
        let payload = minimal_payload(reg.kind);
        let produced = adapter.parse(event_name, &payload).unwrap_or_else(|| {
            panic!(
                "{agent}: HOOK_REGISTRATIONS lists {:?} but parse() returned None — parse arm missing",
                reg.kind
            )
        });
        assert_eq!(
            produced.kind(),
            reg.kind,
            "{agent}: table declares {:?} but parse() produced {:?}",
            reg.kind,
            produced.kind()
        );
    }

    // Parse → table: every kind `parse()` accepts must appear in the table.
    // Catches "added parse arm, forgot to update HOOK_REGISTRATIONS".
    for kind in AgentEventKind::ALL {
        let accepted = adapter
            .parse(kind.external_name(), &minimal_payload(*kind))
            .is_some();
        let in_table = table.iter().any(|r| r.kind == *kind);
        assert!(
            !accepted || in_table,
            "{agent}: parse() accepts {:?} but HOOK_REGISTRATIONS does not list it — add it to the table",
            kind
        );
    }
}

/// The representative payload of one event kind: the minimal payload the
/// kind needs to parse, enriched with a full session context so a drift
/// check can see which context fields a parse arm really reads.
#[cfg(test)]
pub(crate) fn representative_payload(kind: AgentEventKind) -> serde_json::Value {
    let mut payload = minimal_payload(kind);
    let object = payload
        .as_object_mut()
        .expect("minimal payload is an object");
    object.insert("session_id".to_string(), serde_json::json!("sess-drift"));
    object.insert("cwd".to_string(), serde_json::json!("/repo"));
    object.insert(
        "worktree".to_string(),
        serde_json::json!({
            "name": "wt",
            "path": "/repo/wt",
            "branch": "feat",
            "originalRepoDir": "/repo"
        }),
    );
    payload
}

/// The session id a parsed event carries, for the drift checks. The
/// variants without a session id answer `None`.
#[cfg(test)]
pub(crate) fn event_session_id(event: &crate::event::AgentEvent) -> Option<&str> {
    use crate::event::AgentEvent;
    match event {
        AgentEvent::SessionStart { session_id, .. }
        | AgentEvent::UserPromptSubmit { session_id, .. }
        | AgentEvent::Notification { session_id, .. }
        | AgentEvent::Stop { session_id, .. }
        | AgentEvent::StopFailure { session_id, .. }
        | AgentEvent::PermissionDenied { session_id, .. }
        | AgentEvent::CwdChanged { session_id, .. } => session_id.as_deref(),
        _ => None,
    }
}

/// The worktree a parsed event carries, for the drift checks. A parse arm
/// that never reads the worktree object and a payload without one both
/// answer `None`.
#[cfg(test)]
pub(crate) fn event_worktree(
    event: &crate::event::AgentEvent,
) -> Option<&crate::event::WorktreeInfo> {
    use crate::event::AgentEvent;
    match event {
        AgentEvent::SessionStart { worktree, .. }
        | AgentEvent::UserPromptSubmit { worktree, .. }
        | AgentEvent::Notification { worktree, .. }
        | AgentEvent::Stop { worktree, .. }
        | AgentEvent::StopFailure { worktree, .. }
        | AgentEvent::PermissionDenied { worktree, .. }
        | AgentEvent::CwdChanged { worktree, .. } => worktree.as_ref(),
        _ => None,
    }
}

/// The reported-set drift pin. For every event an adapter accepts,
/// `reported()` answers `session_id` exactly when the parse arm carries
/// session context, and an event the adapter does not accept reports
/// nothing — so adding a session-context event without wiring `reported()`
/// fails here instead of silently dropping the identity writes.
///
/// `reads_worktree` marks the adapters whose session-context parse arms
/// read the worktree object (`claude`, `pi`); for them a reported
/// `worktree` must really be parsed from the payload, so dropping
/// `parse_worktree` from an arm fails here instead of clearing the
/// worktree options on every event. `session_fields` is the uniform
/// reported set every session-context event must carry; `None` for a
/// table-driven adapter whose per-event sets differ.
#[cfg(test)]
pub(crate) fn assert_reported_matches_parse(
    agent: &str,
    reads_worktree: bool,
    session_fields: Option<&'static [&'static str]>,
) {
    use crate::event::resolve_adapter;
    let adapter = resolve_adapter(agent).expect("adapter should exist");

    for kind in AgentEventKind::ALL {
        let event_name = kind.external_name();
        let payload = representative_payload(*kind);
        let reported = adapter.reported(event_name, &payload);
        let Some(event) = adapter.parse(event_name, &payload) else {
            assert!(
                reported.is_empty(),
                "{agent}: {event_name} is not accepted by parse() but reported() answers {reported:?}"
            );
            continue;
        };
        let carries_session = event_session_id(&event).is_some();
        assert_eq!(
            reported.contains("session_id"),
            carries_session,
            "{agent}: {event_name} reported() disagrees with its parse arm about session context"
        );
        if carries_session && let Some(fields) = session_fields {
            assert_eq!(
                reported.fields(),
                fields,
                "{agent}: {event_name} reports a set other than the session-context fields"
            );
        }
        if reads_worktree && reported.contains("worktree") {
            let worktree = event_worktree(&event).unwrap_or_else(|| {
                panic!(
                    "{agent}: {event_name} reports `worktree` but its parse arm does not read the worktree object"
                )
            });
            assert!(
                !worktree.name.is_empty()
                    || !worktree.path.is_empty()
                    || !worktree.branch.is_empty(),
                "{agent}: {event_name} reports `worktree` but its parse arm dropped the payload's worktree"
            );
        }
    }
}
