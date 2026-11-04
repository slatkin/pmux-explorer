//! The tmux adapter of the pane-write seam: a [`PaneStore`] backed by the
//! tmux server. One event's write list becomes one `tmux` invocation of
//! `;`-joined `set` commands, and the snapshot options are read with one
//! `display-message`, so a pane never reads mid-batch.

use super::commands::run_tmux;
use super::options::{PANE_BG_CMD, PANE_SESSION_ID, PANE_STARTED_AT, PANE_STATUS, PANE_SUBAGENTS};
use super::query::{q, split_tmux_fields};
use crate::cli::sanitize_tmux_value;
use crate::pane_store::{PaneSnapshot, PaneStore, PaneWrite, PaneWrites};

/// The production store: every call goes to the tmux server.
#[derive(Debug, Clone, Copy, Default)]
pub struct TmuxPaneStore;

/// Build the `tmux` argument list that applies `writes` to `pane` as ONE
/// invocation — `set -t P -p K V ; set -t P -p -u K2 ; …` — mirroring the
/// flag shape `set_pane_option` / `unset_pane_option` invoke tmux with
/// today. `Set` values are encoded with [`sanitize_tmux_value`] here, the
/// one place every written value passes through, so a value is always one
/// field and no caller has to remember the encoder. An empty list yields no
/// arguments; `apply` turns that into a no-op. Pure, so tests can pin the
/// exact invocation without a tmux server.
fn apply_args(pane: &str, writes: &PaneWrites) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    for (index, write) in writes.iter().enumerate() {
        if index > 0 {
            args.push(";".to_string());
        }
        match write {
            PaneWrite::Set(key, value) => args.extend([
                "set".to_string(),
                "-t".to_string(),
                pane.to_string(),
                "-p".to_string(),
                key.to_string(),
                sanitize_tmux_value(value),
            ]),
            PaneWrite::Unset(key) => args.extend([
                "set".to_string(),
                "-t".to_string(),
                pane.to_string(),
                "-p".to_string(),
                "-u".to_string(),
                key.to_string(),
            ]),
        }
    }
    args
}

/// The `display-message -p` format that reads the five snapshot options from
/// one pane in one call, quoted with the same `#{q:...}` convention the
/// `list-panes` read uses so an embedded delimiter stays in its field.
fn snapshot_format() -> String {
    [
        PANE_STATUS,
        PANE_BG_CMD,
        PANE_SUBAGENTS,
        PANE_STARTED_AT,
        PANE_SESSION_ID,
    ]
    .map(q)
    .join("|")
}

/// Decode one snapshot line into the snapshot: an empty field is an unset
/// option, and a field that tmux escaped stays decoded because
/// [`split_tmux_fields`] honors the same escapes the `list-panes` decoder
/// does.
fn snapshot_from(output: &str) -> PaneSnapshot {
    let Some(line) = output.lines().next() else {
        return PaneSnapshot::default();
    };
    let fields = split_tmux_fields(line, '|');
    let field = |index: usize| -> Option<String> {
        fields.get(index).filter(|value| !value.is_empty()).cloned()
    };
    PaneSnapshot {
        status: field(0),
        bg_cmd: field(1),
        subagents: field(2),
        started_at: field(3),
        session_id: field(4),
    }
}

impl PaneStore for TmuxPaneStore {
    fn snapshot(&self, pane: &str) -> PaneSnapshot {
        let format = snapshot_format();
        match run_tmux(&["display-message", "-t", pane, "-p", &format]) {
            Some(output) => snapshot_from(&output),
            // tmux unreachable or the pane gone: the snapshot is empty, the
            // same convention the `list-panes` read uses for a dead server.
            None => PaneSnapshot::default(),
        }
    }

    fn get(&self, pane: &str, key: &str) -> String {
        run_tmux(&["show", "-t", pane, "-pv", key])
            .map(|value| value.trim().to_string())
            .unwrap_or_default()
    }

    fn apply(&self, pane: &str, writes: &PaneWrites) -> Result<(), String> {
        if writes.is_empty() {
            return Ok(());
        }
        let args = apply_args(pane, writes);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_tmux(&arg_refs)
            .map(|_| ())
            .ok_or_else(|| format!("tmux rejected the write batch for pane {pane}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::{PANE_ATTENTION, PANE_WAIT_REASON};

    #[test]
    fn apply_args_pins_the_mixed_batch_argument_list() {
        let mut writes = PaneWrites::new();
        writes.set(PANE_STATUS, "running");
        writes.unset(PANE_STARTED_AT);
        writes.set(PANE_WAIT_REASON, "permission");
        writes.unset(PANE_ATTENTION);

        assert_eq!(
            apply_args("%5", &writes),
            vec![
                "set",
                "-t",
                "%5",
                "-p",
                PANE_STATUS,
                "running",
                ";",
                "set",
                "-t",
                "%5",
                "-p",
                "-u",
                PANE_STARTED_AT,
                ";",
                "set",
                "-t",
                "%5",
                "-p",
                PANE_WAIT_REASON,
                "permission",
                ";",
                "set",
                "-t",
                "%5",
                "-p",
                "-u",
                PANE_ATTENTION,
            ]
        );
    }

    #[test]
    fn apply_args_is_empty_for_an_empty_batch() {
        assert!(apply_args("%5", &PaneWrites::new()).is_empty());
    }

    #[test]
    fn apply_args_encodes_newline_carriage_return_and_pipe_into_one_field() {
        let mut writes = PaneWrites::new();
        writes.set(PANE_WAIT_REASON, "line1\nline2|a\rb");

        let args = apply_args("%5", &writes);

        // One `set` command, one value argument, none of the three
        // separators left in it.
        assert_eq!(
            args,
            vec!["set", "-t", "%5", "-p", PANE_WAIT_REASON, "line1 line2 a b"]
        );
    }

    #[test]
    fn snapshot_format_names_the_five_rule_options_quoted() {
        assert_eq!(
            snapshot_format(),
            "#{q:@pane_status}|#{q:@pane_bg_cmd}|#{q:@pane_subagents}|#{q:@pane_started_at}|#{q:@pane_session_id}"
        );
    }

    #[test]
    fn snapshot_from_decodes_the_four_fields() {
        assert_eq!(
            snapshot_from("waiting|npm run dev|Explore:sub-1|1700|sess-1\n"),
            PaneSnapshot {
                status: Some("waiting".into()),
                bg_cmd: Some("npm run dev".into()),
                subagents: Some("Explore:sub-1".into()),
                started_at: Some("1700".into()),
                session_id: Some("sess-1".into()),
            }
        );
    }

    #[test]
    fn snapshot_from_treats_empty_fields_and_missing_lines_as_unset() {
        assert_eq!(snapshot_from("||||\n"), PaneSnapshot::default());
        assert_eq!(snapshot_from(""), PaneSnapshot::default());
    }

    #[test]
    fn snapshot_from_keeps_an_escaped_delimiter_inside_its_field() {
        // The `#{q:...}` quoting escapes a delimiter in the value with a
        // backslash; the decoder must undo it, the way the `list-panes`
        // decoder does.
        assert_eq!(
            snapshot_from("running|npm\\|run|sub|1700\n")
                .bg_cmd
                .as_deref(),
            Some("npm|run")
        );
    }
}
