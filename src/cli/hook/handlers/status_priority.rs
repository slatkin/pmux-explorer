//! Central resolver for the sidebar's pane status priority. Hook handlers
//! consult these helpers rather than hard-coding the precedence, so the
//! ordering lives in exactly one place.
//!
//! Priority (highest → lowest):
//!
//! ```text
//! running > permission > background > waiting > idle
//! ```
//!
//! Non-obvious rule: action-required wait reasons (the permission class and
//! a blocking user prompt) must stay as `waiting` even when a background
//! shell is alive — the user still has to act on the prompt, so a live bg
//! shell does not lower the signal.

use crate::ui::text::wait_reason_entry;

/// Wait reasons that demand direct user action and must stay visible as
/// `waiting` even with a live background shell.
///
/// The class comes from the shared wait-reason table in `ui::text`, so a
/// reason mapped for display cannot silently stay demotable.
pub(in crate::cli::hook) fn is_action_required_wait_reason(wait_reason: &str) -> bool {
    wait_reason_entry(wait_reason).is_some_and(|entry| entry.action_required)
}

/// Status `Stop` should land in.
pub(in crate::cli::hook) fn resolve_stop_status(bg_shell_live: bool) -> &'static str {
    if bg_shell_live { "background" } else { "idle" }
}

/// Status `Notification` should land in.
pub(in crate::cli::hook) fn resolve_notification_status(
    wait_reason: &str,
    bg_shell_live: bool,
) -> &'static str {
    if bg_shell_live && !is_action_required_wait_reason(wait_reason) {
        "background"
    } else {
        "waiting"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_action_required_wait_reason_allowlist() {
        assert!(is_action_required_wait_reason("permission"));
        assert!(is_action_required_wait_reason("permission_prompt"));
        assert!(is_action_required_wait_reason("permission_denied"));
        assert!(is_action_required_wait_reason("elicitation_dialog"));

        assert!(!is_action_required_wait_reason("auth_success"));
        assert!(!is_action_required_wait_reason("rate_limit"));
        assert!(!is_action_required_wait_reason("session_resumed"));
        assert!(!is_action_required_wait_reason("teammate_idle:alice"));
        assert!(!is_action_required_wait_reason(""));
    }

    #[test]
    fn pi_prompt_kinds_keep_waiting_under_live_bg_shell() {
        for reason in ["select", "confirm", "input", "editor", "custom"] {
            assert!(
                is_action_required_wait_reason(reason),
                "{reason} should be action-required"
            );
            assert_eq!(
                resolve_notification_status(reason, true),
                "waiting",
                "{reason} should stay waiting under a live bg shell"
            );
            assert_eq!(resolve_notification_status(reason, false), "waiting");
        }
    }

    #[test]
    fn resolve_stop_status_prefers_background_when_bg_shell_live() {
        assert_eq!(resolve_stop_status(true), "background");
        assert_eq!(resolve_stop_status(false), "idle");
    }

    #[test]
    fn resolve_notification_status_permission_always_waiting() {
        // The action-required class survives regardless of bg shell state.
        assert_eq!(resolve_notification_status("permission", true), "waiting");
        assert_eq!(resolve_notification_status("permission", false), "waiting");
        assert_eq!(
            resolve_notification_status("permission_prompt", true),
            "waiting"
        );
    }

    #[test]
    fn resolve_notification_status_soft_reason_downgrades_to_background_when_bg_live() {
        assert_eq!(
            resolve_notification_status("auth_success", true),
            "background"
        );
        assert_eq!(
            resolve_notification_status("rate_limit", true),
            "background"
        );
        assert_eq!(resolve_notification_status("", true), "background");
    }

    #[test]
    fn resolve_notification_status_soft_reason_without_bg_stays_waiting() {
        assert_eq!(
            resolve_notification_status("auth_success", false),
            "waiting"
        );
        assert_eq!(resolve_notification_status("", false), "waiting");
    }
}
