use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use crate::pmux_task::PmuxTaskKind;
use crate::process::{ProcessSnapshot, command_basename};

use super::commands::run_tmux;
use super::options::{
    AGENT_SESSION_PANE_OPTIONS, PANE_AGENT, PANE_ATTENTION, PANE_BG_CMD, PANE_CWD, PANE_NAME,
    PANE_PERMISSION_MODE, PANE_PROMPT, PANE_PROMPT_SOURCE, PANE_ROLE, PANE_SESSION_ID,
    PANE_STARTED_AT, PANE_STATUS, PANE_SUBAGENT_KIND, PANE_SUBAGENT_PARENT, PANE_SUBAGENT_TASK,
    PANE_SUBAGENTS, PANE_WAIT_REASON, PANE_WORKTREE_BRANCH, PANE_WORKTREE_NAME, unset_pane_option,
};
use super::types::{
    AgentType, CODEX_AGENT, PaneInfo, PaneStatus, PermissionMode, SessionInfo, SubagentInfo,
    SubagentParent, WindowInfo, WorktreeMetadata,
};
use crate::worktree::SPAWNED_OPTION;

// Field indices in `tmux list-panes -F` output. Keep in lock-step with
// the `pane_format()` field list. When adding a new field, update both
// this module and the format string together.
mod session_line_field {
    pub const SESSION_NAME: usize = 0;
    pub const WINDOW_ID: usize = 1;
    pub const WINDOW_NAME: usize = 3;
    pub const WINDOW_ACTIVE: usize = 4;
    pub const AUTOMATIC_RENAME: usize = 5;
    /// Index where the per-pane field suffix consumed by `parse_pane_line` begins.
    pub const PANE_LINE_OFFSET: usize = 6;
    /// Minimum number of fields a valid `pane_format()` line must contain.
    pub const MIN_FIELDS: usize = 33;
}

// Indices into the pane-line suffix that `parse_pane_line` operates on.
// Each value = (absolute index in the full format string) - 6, because
// `build_session_hierarchy` strips the leading 6 window-level fields
// before joining the remainder back into `pane_line`.
pub(super) mod pane_line_field {
    pub const PANE_ACTIVE: usize = 0; // absolute 6
    pub const PANE_STATUS: usize = 1; // absolute 7  (@pane_status)
    pub const PANE_ATTENTION: usize = 2; // absolute 8  (@pane_attention)
    pub const AGENT: usize = 3; // absolute 9  (@pane_agent)
    pub const PANE_CURRENT_PATH: usize = 5; // absolute 11 (pane_current_path)
    pub const PANE_CURRENT_COMMAND: usize = 6; // absolute 12
    pub const PANE_ROLE: usize = 7; // absolute 13 (@pane_role)
    pub const PANE_ID: usize = 8; // absolute 14
    pub const PROMPT: usize = 9; // absolute 15 (@pane_prompt)
    pub const PROMPT_SOURCE: usize = 10; // absolute 16 (@pane_prompt_source)
    pub const STARTED_AT: usize = 11; // absolute 17 (@pane_started_at)
    pub const WAIT_REASON: usize = 12; // absolute 18 (@pane_wait_reason)
    pub const PANE_PID: usize = 13; // absolute 19
    pub const SUBAGENTS: usize = 14; // absolute 20 (@pane_subagents)
    pub const PANE_CWD: usize = 15; // absolute 21 (@pane_cwd)
    pub const PERMISSION_MODE: usize = 16; // absolute 22 (@pane_permission_mode)
    pub const WORKTREE_NAME: usize = 17; // absolute 23 (@pane_worktree_name)
    pub const WORKTREE_BRANCH: usize = 18; // absolute 24 (@pane_worktree_branch)
    pub const SESSION_ID: usize = 19; // absolute 25 (@pane_session_id)
    pub const SIDEBAR_SPAWNED: usize = 20; // absolute 26 (@pmux-explorer-spawned)
    pub const BG_CMD: usize = 21; // absolute 27 (@pane_bg_cmd)
    pub const SUBAGENT_KIND: usize = 22; // absolute 28 (@pane_subagent_kind)
    pub const SUBAGENT_PARENT: usize = 23; // absolute 29 (@pane_subagent_parent)
    pub const SUBAGENT_TASK: usize = 24; // absolute 30 (@pane_subagent_task)
    pub const PANE_TITLE: usize = 25; // absolute 31 (pane_title)
    pub const HOST: usize = 26; // absolute 32 (host)
    /// Minimum number of fields the pane-line suffix must contain.
    /// Equals `session_line_field::MIN_FIELDS - PANE_LINE_OFFSET`.
    pub const MIN_FIELDS: usize = 27;
}

/// Build the tmux `list-panes -F` format used by [`query_sessions`].
/// Every field is quoted with `#{q:...}` so embedded pipes in user content
/// survive the split.
fn pane_format() -> String {
    [
        q("session_name"),
        q("window_id"),
        q("window_index"),
        q("window_name"),
        q("window_active"),
        q("automatic-rename"),
        q("pane_active"),
        q(PANE_STATUS),
        q(PANE_ATTENTION),
        q(PANE_AGENT),
        q(PANE_NAME),
        q("pane_current_path"),
        q("pane_current_command"),
        q(PANE_ROLE),
        q("pane_id"),
        q(PANE_PROMPT),
        q(PANE_PROMPT_SOURCE),
        q(PANE_STARTED_AT),
        q(PANE_WAIT_REASON),
        q("pane_pid"),
        q(PANE_SUBAGENTS),
        q(PANE_CWD),
        q(PANE_PERMISSION_MODE),
        q(PANE_WORKTREE_NAME),
        q(PANE_WORKTREE_BRANCH),
        q(PANE_SESSION_ID),
        q(SPAWNED_OPTION),
        q(PANE_BG_CMD),
        q(PANE_SUBAGENT_KIND),
        q(PANE_SUBAGENT_PARENT),
        q(PANE_SUBAGENT_TASK),
        q("pane_title"),
        q("host"),
    ]
    .join("|")
}

fn q(field: &str) -> String {
    format!("#{{q:{field}}}")
}

type SessionMap = indexmap::IndexMap<String, indexmap::IndexMap<String, WindowInfo>>;

/// (window_id, pane_index_in_window, pane_pid) — the minimum info needed to
/// later retarget a permission-mode update at the right pane.
type CodexPidEntry = (String, usize, u32);

/// Parent-resolution data collected while the pane lines are parsed, and
/// applied by [`resolve_subagent_parents`] once every line is seen.
#[derive(Default)]
struct SubagentResolution {
    /// `@pane_session_id` -> parent label (`pane_title`, or the pane id when
    /// the title is empty) for every parsed agent pane. The first pane that
    /// reports a session id wins.
    parent_labels: HashMap<String, String>,
    /// pane_id -> recorded parent session id for every parsed subagent pane.
    pending: HashMap<String, String>,
}

impl SubagentResolution {
    /// Records one parsed pane: its label when it reports a session id, and
    /// its pending parent lookup when it is a subagent.
    fn observe(&mut self, pane: &PaneInfo, pane_fields: &[String]) {
        if let Some(session_id) = &pane.session_id {
            let title = &pane_fields[pane_line_field::PANE_TITLE];
            // Use the same trimmed pane name the parent's own status row
            // shows, so the parent label is consistent across rows; fall
            // back to the raw title, then the pane id, as before.
            let label = match &pane.pane_name {
                Some(name) => name.clone(),
                None if title.is_empty() => pane.pane_id.clone(),
                None => title.clone(),
            };
            self.parent_labels
                .entry(session_id.clone())
                .or_insert(label);
        }
        if pane.subagent.is_some() {
            self.pending.insert(
                pane.pane_id.clone(),
                pane_fields[pane_line_field::SUBAGENT_PARENT].clone(),
            );
        }
    }
}

/// Query all sessions, windows, and panes in a single `tmux list-panes -a` call
/// (plus one optional `ps` call for process-backed agent checks), instead of
/// N+1 subprocess invocations.
pub fn query_sessions() -> Vec<SessionInfo> {
    query_sessions_with_process_snapshot().0
}

pub(crate) fn query_sessions_with_process_snapshot() -> (Vec<SessionInfo>, Option<ProcessSnapshot>)
{
    let pane_format = pane_format();
    let all_panes_output = match run_tmux(&["list-panes", "-a", "-F", &pane_format]) {
        Some(s) => s,
        None => return (vec![], None),
    };

    let process_snapshot = process_snapshot_for_panes(&all_panes_output);
    let (mut sessions_map, codex_pids, subagent_resolution) =
        build_session_hierarchy(&all_panes_output, process_snapshot.as_ref());
    resolve_subagent_parents(&mut sessions_map, &subagent_resolution);
    if !codex_pids.is_empty()
        && let Some(snapshot) = &process_snapshot
    {
        resolve_codex_permission_modes(&mut sessions_map, &codex_pids, snapshot);
    }
    (finalize_sessions(sessions_map), process_snapshot)
}

/// Parse the raw `tmux list-panes` output into an indexed session→window→pane
/// hierarchy. Also returns every Codex pane's pid so the caller can resolve
/// permission modes in a single `ps` pass.
fn build_session_hierarchy(
    all_panes_output: &str,
    process_snapshot: Option<&ProcessSnapshot>,
) -> (SessionMap, Vec<CodexPidEntry>, SubagentResolution) {
    let mut sessions_map: SessionMap = indexmap::IndexMap::new();
    let mut codex_pids: Vec<CodexPidEntry> = Vec::new();
    let mut seen_pids: HashSet<u32> = HashSet::new();
    let mut subagent_resolution = SubagentResolution::default();

    for line in all_panes_output.lines() {
        let parts = split_tmux_fields(line, '|');
        if parts.len() < session_line_field::MIN_FIELDS {
            continue;
        }

        let session_name = parts[session_line_field::SESSION_NAME].as_str();
        let window_id = parts[session_line_field::WINDOW_ID].as_str();
        // Pass the unescaped pane fields directly instead of re-joining
        // with `|` and re-splitting, which would turn any literal pipe
        // inside a pane field (cwd, prompt, branch) back into a field
        // separator and shift every downstream index.
        let pane_fields = &parts[session_line_field::PANE_LINE_OFFSET..];

        // Deduplicate panes shared across grouped sessions:
        // same pane_pid may appear in multiple sessions, keep only
        // the first occurrence. pane_pid is at index 13 in pane_fields.
        if let Some(pid_str) = pane_fields.get(pane_line_field::PANE_PID)
            && let Ok(pid) = pid_str.parse::<u32>()
            && pid != 0
            && !seen_pids.insert(pid)
        {
            continue;
        }

        let sessions_entry = sessions_map.entry(session_name.to_string()).or_default();

        let window = sessions_entry
            .entry(window_id.to_string())
            .or_insert_with(|| WindowInfo {
                window_id: window_id.to_string(),
                window_name: parts[session_line_field::WINDOW_NAME].to_string(),
                window_active: parts[session_line_field::WINDOW_ACTIVE] == "1",
                auto_rename: parts[session_line_field::AUTOMATIC_RENAME] == "1",
                panes: Vec::new(),
            });

        if let Some(pane) = parse_pane_fields_with_processes(pane_fields, process_snapshot) {
            if pane.agent == AgentType::Codex
                && let Some(pid) = pane.pane_pid
            {
                codex_pids.push((window_id.to_string(), window.panes.len(), pid));
            }
            subagent_resolution.observe(&pane, pane_fields);
            window.panes.push(pane);
        }
    }

    (sessions_map, codex_pids, subagent_resolution)
}

/// Set each subagent pane's parent from the collected session-id -> label
/// map, or `Orphaned` when no live pane reports the recorded session id.
/// Runs before any repository filter, so a parent row the filter hides still
/// resolves.
fn resolve_subagent_parents(sessions_map: &mut SessionMap, resolution: &SubagentResolution) {
    for windows in sessions_map.values_mut() {
        for window in windows.values_mut() {
            for pane in &mut window.panes {
                let Some(parent_session_id) = resolution.pending.get(&pane.pane_id) else {
                    continue;
                };
                if let Some(info) = &mut pane.subagent {
                    info.parent = resolution
                        .parent_labels
                        .get(parent_session_id)
                        .cloned()
                        .map_or(SubagentParent::Orphaned, SubagentParent::Pane);
                }
            }
        }
    }
}

/// Fan out Codex permission mode updates to every Codex pane across every
/// window using the same single process snapshot used for shell-fallback checks.
fn resolve_codex_permission_modes(
    sessions_map: &mut SessionMap,
    codex_pids: &[CodexPidEntry],
    process_snapshot: &ProcessSnapshot,
) {
    for windows in sessions_map.values_mut() {
        for (window_id, window) in windows.iter_mut() {
            let window_pids: Vec<(usize, u32)> = codex_pids
                .iter()
                .filter(|(wid, _, _)| wid == window_id)
                .map(|(_, idx, pid)| (*idx, *pid))
                .collect();
            if window_pids.is_empty() {
                continue;
            }
            apply_codex_permission_modes(&mut window.panes, &window_pids, process_snapshot);
        }
    }
}

/// Flatten the session→window hierarchy into a `Vec<SessionInfo>`, dropping
/// any windows whose `parse_pane_line` filtering left them empty, and any
/// sessions whose windows are all empty as a result.
fn finalize_sessions(sessions_map: SessionMap) -> Vec<SessionInfo> {
    let mut sessions = Vec::new();
    for (session_name, windows) in sessions_map {
        let windows: Vec<WindowInfo> = windows
            .into_values()
            .filter(|w| !w.panes.is_empty())
            .collect();
        if !windows.is_empty() {
            sessions.push(SessionInfo {
                session_name,
                windows,
            });
        }
    }
    sessions
}

/// Parse a single pane line from `tmux list-panes -F`.
/// Returns None if the line has too few fields, is a sidebar, or has no agent.
/// Thin wrapper used by the unit tests, which still construct a raw
/// `|`-joined fixture line. Production callers go through
/// `parse_pane_fields_with_processes` directly to avoid re-joining and re-splitting
/// fields that may themselves contain literal `|` characters (cwd,
/// prompt, branch) — see `build_session_hierarchy`.
#[cfg(test)]
pub(crate) fn parse_pane_line(line: &str) -> Option<PaneInfo> {
    let parts = split_tmux_fields(line, '|');
    parse_pane_fields_with_processes(&parts, None)
}

fn parse_pane_fields_with_processes(
    parts: &[String],
    process_snapshot: Option<&ProcessSnapshot>,
) -> Option<PaneInfo> {
    if parts.len() < pane_line_field::MIN_FIELDS {
        return None;
    }

    if parts[pane_line_field::PANE_ROLE] == "sidebar" {
        return None;
    }

    let agent = AgentType::from_label(&parts[pane_line_field::AGENT])?;
    let current_command = parts[pane_line_field::PANE_CURRENT_COMMAND].as_str();
    let pane_pid: Option<u32> = parts[pane_line_field::PANE_PID].parse().ok();

    // Codex / OpenCode / pi panes can leave stale tmux metadata behind after
    // the agent exits and the pane falls back to the user's shell. None of
    // them guarantees a "process exit" event reaches the sidebar (Codex has
    // no such hook, OpenCode runs under Bun where `process.on("exit")` does
    // not fire our handlers, and a pi process can be killed), so the Rust
    // polling side must own teardown: wipe pane options + activity log the
    // first poll after the agent is gone. Subsequent polls short-circuit at
    // the `AgentType::from_label` check above once `@pane_agent` has been
    // cleared. Claude is excluded because its SessionEnd hook drives cleanup
    // instead.
    if matches!(
        agent,
        AgentType::Codex | AgentType::OpenCode | AgentType::Pi
    ) && is_shell_command(current_command)
    {
        let agent_still_alive = pane_pid
            .and_then(|pid| {
                process_snapshot.map(|snapshot| snapshot.tree_has_agent(&[pid], &agent))
            })
            .unwrap_or(false);
        if !agent_still_alive {
            clear_agent_pane_state(&parts[pane_line_field::PANE_ID]);
            return None;
        }
    }

    // Prefer @pane_cwd (set by hook from agent's cwd) over pane_current_path
    let pane_cwd = &parts[pane_line_field::PANE_CWD];
    let path = if !pane_cwd.is_empty() {
        pane_cwd.to_string()
    } else {
        parts[pane_line_field::PANE_CURRENT_PATH].to_string()
    };

    // Claude: read permission_mode from hook-set tmux variable.
    // Codex / OpenCode / pi: no permission_mode in the protocol, keep the
    // default.
    let permission_mode = if agent == AgentType::Claude {
        PermissionMode::from_label(&parts[pane_line_field::PERMISSION_MODE])
    } else {
        PermissionMode::Default
    };

    let prompt_source = &parts[pane_line_field::PROMPT_SOURCE];
    let prompt_is_response = prompt_source == "response";

    // Sanitize prompt: replace pipes/newlines, filter system-injected messages, truncate
    let prompt = sanitize_prompt(&parts[pane_line_field::PROMPT]);

    let session_id = if parts[pane_line_field::SESSION_ID].is_empty() {
        None
    } else {
        Some(parts[pane_line_field::SESSION_ID].to_string())
    };

    let subagent = parse_subagent_identity(
        &agent,
        &parts[pane_line_field::SUBAGENT_KIND],
        &parts[pane_line_field::SUBAGENT_TASK],
    );

    // The pane's own name: pmux names a pane with `select-pane -T`, which
    // shows up as `pane_title`. tmux defaults the title to the host name
    // and pi / Claude Code overwrite it with OSC titles, so a title that
    // is empty, equals the host, or matches an agent's OSC prefix is not
    // a name. `automatic-rename` only affects the window name, so it does
    // not enter the gate. Known gap: agent-set titles other than pi's and
    // Claude's (Codex, opencode) are not filtered and show as pane names.
    // A title that carries an em dash keeps only the text before it: the
    // suffix (a branch, a status, a cwd) is decoration, not the name.
    let pane_title = &parts[pane_line_field::PANE_TITLE];
    let pane_name = if !pane_title.is_empty()
        && pane_title != parts[pane_line_field::HOST].as_str()
        && !pane_title.starts_with("π - ")
        && !pane_title.starts_with("✳ ")
    {
        let before_dash = pane_title.split('—').next().unwrap_or(pane_title);
        Some(before_dash.trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    };

    Some(PaneInfo {
        pane_active: parts[pane_line_field::PANE_ACTIVE] == "1",
        status: PaneStatus::from_label(&parts[pane_line_field::PANE_STATUS]),
        attention: !parts[pane_line_field::PANE_ATTENTION].is_empty(),
        agent,
        path,
        current_command: parts[pane_line_field::PANE_CURRENT_COMMAND].to_string(),
        pane_id: parts[pane_line_field::PANE_ID].to_string(),
        prompt,
        prompt_is_response,
        started_at: parts[pane_line_field::STARTED_AT].parse().ok(),
        wait_reason: parts[pane_line_field::WAIT_REASON].to_string(),
        permission_mode,
        subagents: parse_subagents(&parts[pane_line_field::SUBAGENTS]),
        pane_pid,
        worktree: WorktreeMetadata {
            name: parts[pane_line_field::WORKTREE_NAME].to_string(),
            branch: parts[pane_line_field::WORKTREE_BRANCH].to_string(),
        },
        session_id,
        pane_name,
        sidebar_spawned: parts[pane_line_field::SIDEBAR_SPAWNED] == "1",
        bg_shell_cmd: {
            let raw = &parts[pane_line_field::BG_CMD];
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        },
        subagent,
    })
}

/// Parse the pmux subagent identity from the `@pane_subagent_*` fields.
/// The three options are pi-only (`docs/agent-protocol.md`), so a pane
/// whose agent is not pi is never a subagent, mirroring the agent-gated
/// `permission_mode` read — a pi pane that flipped to another agent
/// without an intervening shell keeps the stale options (no sweep runs
/// and SessionStart preserves them). Within a pi pane, a subagent
/// identity exists only when `@pane_subagent_kind` holds a known kind
/// value; the parent starts `Orphaned` and is resolved by the post-pass
/// [`resolve_subagent_parents`] once the whole `list-panes` output is seen.
fn parse_subagent_identity(
    agent: &AgentType,
    kind_raw: &str,
    task_raw: &str,
) -> Option<SubagentInfo> {
    if *agent != AgentType::Pi {
        return None;
    }
    let task = if task_raw.is_empty() {
        None
    } else {
        Some(task_raw.to_string())
    };
    Some(SubagentInfo {
        kind: PmuxTaskKind::from_label(kind_raw)?,
        task,
        parent: SubagentParent::Orphaned,
    })
}

/// Wipe all agent-tracked tmux pane options and the activity log file for
/// `pane_id`. Triggered by `parse_pane_fields` when it detects a Codex or
/// OpenCode pane that has dropped back to the user's shell, since neither
/// CLI fires a reliable process-exit hook. Claude panes are never routed
/// here because Claude has its own SessionEnd hook. The key set is the
/// full [`AGENT_SESSION_PANE_OPTIONS`] lifecycle list from
/// `tmux/options.rs` — the single enumeration shared by every teardown
/// sweep, each of which skips only the keys it deliberately keeps.
fn clear_agent_pane_state(pane_id: &str) {
    for key in AGENT_SESSION_PANE_OPTIONS {
        unset_pane_option(pane_id, key);
    }
    let log_path = crate::activity::log_file_path(pane_id);
    let _ = std::fs::remove_file(log_path);
}

fn is_shell_command(command: &str) -> bool {
    const SHELL_COMMANDS: &[&str] = &[
        "ash",
        "bash",
        "csh",
        "dash",
        "elvish",
        "fish",
        "ksh",
        "mksh",
        "nu",
        "oksh",
        "pdksh",
        "posh",
        "powershell",
        "powershell.exe",
        "pwsh",
        "sh",
        "tcsh",
        "xonsh",
        "zsh",
    ];

    let Some(token) = command.split_whitespace().next() else {
        return false;
    };
    let executable = Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(token)
        .to_ascii_lowercase();

    SHELL_COMMANDS.contains(&executable.as_str())
}

/// Detect Codex permission mode from process args (--full-auto, --yolo, etc.)
fn detect_codex_permission_mode(args: &str) -> PermissionMode {
    if args.contains("dangerously-bypass-approvals-and-sandbox") || args.contains("--yolo") {
        return PermissionMode::BypassPermissions;
    }
    if args.contains("--full-auto") {
        return PermissionMode::Auto;
    }
    PermissionMode::Default
}

fn process_snapshot_for_panes(all_panes_output: &str) -> Option<ProcessSnapshot> {
    if !pane_output_needs_process_snapshot(all_panes_output) {
        return None;
    }
    ProcessSnapshot::scan()
}

fn pane_output_needs_process_snapshot(all_panes_output: &str) -> bool {
    all_panes_output.lines().any(|line| {
        let parts = split_tmux_fields(line, '|');
        if parts.len() < session_line_field::MIN_FIELDS {
            return false;
        }
        let pane_fields = &parts[session_line_field::PANE_LINE_OFFSET..];
        AgentType::from_label(&pane_fields[pane_line_field::AGENT]).is_some_and(|agent| {
            matches!(
                agent,
                AgentType::Codex | AgentType::OpenCode | AgentType::Pi
            )
        })
    })
}

fn apply_codex_permission_modes(
    panes: &mut [PaneInfo],
    pids_to_check: &[(usize, u32)],
    process_snapshot: &ProcessSnapshot,
) {
    for (idx, pid) in pids_to_check {
        let descendants = process_snapshot.descendants(&[*pid]);
        for descendant in descendants {
            let Some(info) = process_snapshot.info_by_pid.get(&descendant) else {
                continue;
            };
            if command_basename(&info.comm) != CODEX_AGENT {
                continue;
            }
            if let Some(pane) = panes.get_mut(*idx) {
                pane.permission_mode = detect_codex_permission_mode(&info.args);
                if pane.permission_mode != PermissionMode::Default {
                    break;
                }
            }
        }
    }
}

/// Sanitize prompt text from tmux variable so it's safe for display.
fn sanitize_prompt(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    // Filter known system-injected messages. Avoid the old broad angle-bracket
    // check so legitimate prompts containing comparisons or code snippets
    // still render.
    if raw.contains("<task-notification>")
        || raw.contains("<system-reminder>")
        || raw.contains("<task-status>")
    {
        return String::new();
    }
    if raw.chars().count() > 200 {
        raw.chars().take(200).collect()
    } else {
        raw.to_string()
    }
}

/// Parse subagent list from tmux variable.
/// Format: comma-separated "type" entries, e.g. "Explore,Explore,Plan"
/// Parse the comma-separated `@pane_subagents` value into display strings.
///
/// Each entry is either `agent_type` (legacy) or `agent_type:agent_id`
/// (current). When an `agent_id` is present, the entry is rendered as
/// `"agent_type #<id-prefix>"` where `<id-prefix>` is the first 4 characters
/// of the id — stable per instance, so the UI label does not shift when
/// sibling subagents stop. The `#` embedding is recognized by the `#`-based
/// numbering branch in `subagent_rows`, which keeps it verbatim.
fn parse_subagents(raw: &str) -> Vec<String> {
    const ID_PREFIX_LEN: usize = 4;
    if raw.is_empty() {
        return vec![];
    }
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|entry| match entry.split_once(':') {
            Some((ty, id)) if !id.is_empty() => {
                let prefix: String = id.chars().take(ID_PREFIX_LEN).collect();
                format!("{} #{}", ty, prefix)
            }
            _ => entry.to_string(),
        })
        .collect()
}

/// Split a tmux format line while honoring tmux `#{q:...}` backslash escapes.
fn split_tmux_fields(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == delimiter {
            fields.push(current);
            current = String::new();
            continue;
        }

        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }

    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::super::options::test_mock;
    use super::*;

    #[test]
    fn detect_codex_permission_mode_variants() {
        assert_eq!(
            detect_codex_permission_mode("codex"),
            PermissionMode::Default
        );
        assert_eq!(
            detect_codex_permission_mode("codex --full-auto"),
            PermissionMode::Auto
        );
        assert_eq!(
            detect_codex_permission_mode("codex --dangerously-bypass-approvals-and-sandbox"),
            PermissionMode::BypassPermissions
        );
        assert_eq!(
            detect_codex_permission_mode("codex --full-auto --yolo"),
            PermissionMode::BypassPermissions
        );
    }

    fn test_pane_codex(id: &str) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            pane_active: false,
            status: PaneStatus::Idle,
            attention: false,
            agent: AgentType::Codex,
            path: "/tmp".into(),
            current_command: String::new(),
            prompt: String::new(),
            prompt_is_response: false,
            started_at: None,
            wait_reason: String::new(),
            permission_mode: PermissionMode::Default,
            subagents: vec![],
            pane_pid: None,
            worktree: WorktreeMetadata::default(),
            session_id: None,
            sidebar_spawned: false,
            pane_name: None,
            bg_shell_cmd: None,
            subagent: None,
        }
    }

    #[test]
    fn apply_codex_permission_modes_from_ps() {
        let mut panes = vec![test_pane_codex("%1")];
        let pids = vec![(0, 101)];
        let ps_out = "101 1 bash /bin/bash\n102 101 codex /bin/codex --full-auto\n";
        let snapshot = ProcessSnapshot::from_ps_output(ps_out);

        apply_codex_permission_modes(&mut panes, &pids, &snapshot);
        assert_eq!(panes[0].permission_mode, PermissionMode::Auto);
    }

    #[test]
    fn apply_codex_permission_modes_follows_shell_wrappers() {
        let mut panes = vec![test_pane_codex("%1")];
        let pids = vec![(0, 101)];
        let ps_out = "101 1 bash /bin/bash\n102 101 sh -c wrapper\n103 102 codex /usr/local/bin/codex --yolo\n";
        let snapshot = ProcessSnapshot::from_ps_output(ps_out);

        apply_codex_permission_modes(&mut panes, &pids, &snapshot);
        assert_eq!(panes[0].permission_mode, PermissionMode::BypassPermissions);
    }

    #[test]
    fn apply_codex_permission_modes_matches_path_comm() {
        let mut panes = vec![test_pane_codex("%1")];
        let pids = vec![(0, 101)];
        let ps_out = "101 1 /bin/zsh /bin/zsh\n102 101 /opt/homebrew/bin/codex /opt/homebrew/bin/codex --full-auto\n";
        let snapshot = ProcessSnapshot::from_ps_output(ps_out);

        apply_codex_permission_modes(&mut panes, &pids, &snapshot);
        assert_eq!(panes[0].permission_mode, PermissionMode::Auto);
    }

    #[test]
    fn parse_ps_processes_preserves_spaced_args() {
        let snapshot = ProcessSnapshot::from_ps_output(
            "100 1 codex /Applications/Codex App/bin/codex --full-auto\n101 100 sh sh -c wrapper\n",
        );

        assert_eq!(snapshot.children_of.get(&1).cloned(), Some(vec![100]));
        let info = snapshot.info_by_pid.get(&100).expect("process info");
        assert_eq!(info.comm, "codex");
        assert_eq!(info.args, "/Applications/Codex App/bin/codex --full-auto");
    }

    #[test]
    fn process_tree_has_agent_matches_descendant_process_name() {
        let snapshot = ProcessSnapshot::from_ps_output(
            "100 1 fish fish -c opencode\n101 100 opencode opencode\n",
        );

        assert!(snapshot.tree_has_agent(&[100], &AgentType::OpenCode));
        assert!(!snapshot.tree_has_agent(&[100], &AgentType::Codex));
    }

    // ─── sanitize_prompt tests ──────────────────────────────────────

    #[test]
    fn sanitize_prompt_filters_system_injected() {
        assert_eq!(
            sanitize_prompt("<system-reminder>noise</system-reminder>"),
            ""
        );
        assert_eq!(
            sanitize_prompt("hello <task-notification>abc</task-notification> world"),
            ""
        );
    }

    #[test]
    fn sanitize_prompt_passes_normal_text() {
        assert_eq!(sanitize_prompt("fix the bug"), "fix the bug");
    }

    #[test]
    fn sanitize_prompt_keeps_legitimate_angle_brackets() {
        assert_eq!(sanitize_prompt("1 < 2 and 3 > 1"), "1 < 2 and 3 > 1");
    }

    #[test]
    fn sanitize_prompt_truncates_long_text() {
        let long = "a".repeat(300);
        let result = sanitize_prompt(&long);
        assert_eq!(result.chars().count(), 200);
    }

    #[test]
    fn sanitize_prompt_empty() {
        assert_eq!(sanitize_prompt(""), "");
    }

    // ─── parse_subagents tests ──────────────────────────────────────

    #[test]
    fn parse_subagents_empty() {
        assert_eq!(parse_subagents(""), Vec::<String>::new());
    }

    #[test]
    fn parse_subagents_single() {
        assert_eq!(parse_subagents("Explore"), vec!["Explore"]);
    }

    #[test]
    fn parse_subagents_multiple() {
        assert_eq!(
            parse_subagents("Explore,Plan,Bash"),
            vec!["Explore", "Plan", "Bash"]
        );
    }

    #[test]
    fn parse_subagents_duplicates() {
        assert_eq!(
            parse_subagents("Explore,Explore,Plan"),
            vec!["Explore", "Explore", "Plan"]
        );
    }

    #[test]
    fn parse_subagents_renders_id_prefix() {
        // Current format: `type:id`. The id prefix is used as a stable
        // `#<prefix>` label so surviving siblings do not renumber when
        // another subagent stops.
        assert_eq!(
            parse_subagents("Explore:sub123456,Plan:abc987654"),
            vec!["Explore #sub1", "Plan #abc9"]
        );
    }

    #[test]
    fn parse_subagents_id_prefix_distinguishes_parallel_same_type() {
        // Two subagents of the same type get distinct labels from their ids,
        // which is the whole point of id-based tagging.
        assert_eq!(
            parse_subagents("Explore:aaaa1111,Explore:bbbb2222"),
            vec!["Explore #aaaa", "Explore #bbbb"]
        );
    }

    #[test]
    fn parse_subagents_id_shorter_than_prefix_len_uses_full_id() {
        // Short ids (e.g. test fixtures like "s1") render in full rather
        // than being padded or truncated to nothing.
        assert_eq!(parse_subagents("Plan:s1"), vec!["Plan #s1"]);
    }

    #[test]
    fn parse_subagents_legacy_without_id_renders_type_only() {
        // Stale entry written before id tracking (or by an older build)
        // falls back to the bare type name.
        assert_eq!(
            parse_subagents("Explore,Plan:sub-999"),
            vec!["Explore", "Plan #sub-"]
        );
    }

    // ─── parse_pane_line tests ──────────────────────────────────────

    fn make_pane_line(fields: &[&str]) -> String {
        fields.join("|")
    }

    fn full_fields() -> Vec<&'static str> {
        vec![
            "1",                  // 0: pane_active
            "running",            // 1: @pane_status
            "",                   // 2: @pane_attention
            "claude",             // 3: @pane_agent
            "my-agent",           // 4: @pane_name
            "/home/user/project", // 5: pane_current_path
            "fish",               // 6: pane_current_command
            "",                   // 7: @pane_role
            "%1",                 // 8: pane_id
            "fix the bug",        // 9: @pane_prompt
            "user",               // 10: @pane_prompt_source
            "1700000000",         // 11: @pane_started_at
            "",                   // 12: @pane_wait_reason
            "12345",              // 13: pane_pid
            "Explore,Plan",       // 14: @pane_subagents
            "/custom/cwd",        // 15: @pane_cwd
            "auto",               // 16: @pane_permission_mode
            "",                   // 17: @pane_worktree_name
            "",                   // 18: @pane_worktree_branch
            "",                   // 19: @pane_session_id
            "",                   // 20: @pmux-explorer-spawned
            "",                   // 21: @pane_bg_cmd
            "",                   // 22: @pane_subagent_kind
            "",                   // 23: @pane_subagent_parent
            "",                   // 24: @pane_subagent_task
            "",                   // 25: pane_title
            "myhost",             // 26: host
        ]
    }

    fn process_snapshot(ps_out: &str) -> ProcessSnapshot {
        ProcessSnapshot::from_ps_output(ps_out)
    }

    fn field_strings(fields: &[&str]) -> Vec<String> {
        fields.iter().map(|field| (*field).to_string()).collect()
    }

    #[test]
    fn parse_pane_line_full_fields() {
        let line = make_pane_line(&full_fields());
        let pane = parse_pane_line(&line).expect("should parse 27 fields");
        assert!(pane.pane_active);
        assert_eq!(pane.status, PaneStatus::Running);
        assert_eq!(pane.agent, AgentType::Claude);
        assert_eq!(pane.path, "/custom/cwd"); // pane_cwd preferred
        assert_eq!(pane.current_command, "fish");
        assert_eq!(pane.pane_id, "%1");
        assert_eq!(pane.prompt, "fix the bug");
        assert!(!pane.prompt_is_response);
        assert_eq!(pane.started_at, Some(1700000000));
        assert_eq!(pane.pane_pid, Some(12345));
        assert_eq!(pane.subagents, vec!["Explore", "Plan"]);
        assert_eq!(pane.permission_mode, PermissionMode::Auto);
        assert!(
            pane.subagent.is_none(),
            "no subagent options -> no identity"
        );
    }

    #[test]
    fn parse_pane_line_sidebar_spawned_field() {
        let mut fields = full_fields();
        fields[20] = "1";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
        assert!(pane.sidebar_spawned);

        fields[20] = "";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
        assert!(!pane.sidebar_spawned);

        fields[20] = "0";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
        assert!(
            !pane.sidebar_spawned,
            "any value other than `1` is treated as false"
        );
    }

    #[test]
    fn parse_pane_line_response_prompt_source() {
        let mut fields = full_fields();
        fields[10] = "response"; // @pane_prompt_source
        let line = make_pane_line(&fields);
        let pane = parse_pane_line(&line).unwrap();
        assert!(pane.prompt_is_response);
    }

    #[test]
    fn parse_pane_line_rejects_fewer_than_min_fields() {
        // Only 15 fields — should be rejected
        let fields_15 =
            "1|running||claude|name|/path|fish||%1|prompt|1700000000||12345|Explore|/cwd";
        assert!(
            parse_pane_line(fields_15).is_none(),
            "15 fields should be rejected"
        );

        // 21 fields — still rejected (need 27 including the five tail
        // fields: the three `@pane_subagent_*` options, `pane_title`, and
        // `host`).
        let fields_21 = "1|running||claude|name|/path|fish||%1|prompt|user|1700000000||12345|Explore|/cwd|auto||||";
        assert!(
            parse_pane_line(fields_21).is_none(),
            "21 fields should be rejected"
        );
    }

    #[test]
    fn parse_pane_line_reads_bg_cmd_field() {
        let mut fields = full_fields();
        fields[pane_line_field::BG_CMD] = "cargo build --release";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
        assert_eq!(
            pane.bg_shell_cmd.as_deref(),
            Some("cargo build --release"),
            "bg_shell_cmd should surface the @pane_bg_cmd value"
        );

        fields[pane_line_field::BG_CMD] = "";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
        assert!(
            pane.bg_shell_cmd.is_none(),
            "empty @pane_bg_cmd should parse as None"
        );
    }

    // ─── pane_name resolution tests ────────────────────────────

    #[test]
    fn parse_pane_line_pane_name_gate() {
        // A title that is non-empty, different from the host name, and
        // not an agent OSC title is a pane name; anything else is None.
        let cases = [
            ("my-task", Some("my-task")),      // pmux `select-pane -T my-task`
            ("myhost", None),                  // tmux default: the host name
            ("π - send-report-channel", None), // pi's OSC title
            ("✳ Claude Code", None),           // Claude Code's OSC title
            ("", None),                        // empty title
            ("edit — fix bug", Some("edit")),  // em dash: keep only the text before it
            ("edit—fix bug", Some("edit")),    // em dash with no surrounding spaces
            ("edit — ", Some("edit")),         // em dash with nothing after it
            ("— fix bug", None),               // leading em dash trims to empty: not a name
            ("—", None),                       // a lone em dash is not a name
        ];
        for (title, expected) in cases {
            let mut fields = full_fields();
            fields[pane_line_field::PANE_TITLE] = title;
            let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();
            assert_eq!(pane.pane_name.as_deref(), expected, "title: {title:?}");
        }
    }

    // ─── parse_subagent_identity tests ────────────────────────────

    #[test]
    fn parse_pane_line_subagent_options_parse_into_identity() {
        let mut fields = full_fields();
        fields[pane_line_field::AGENT] = "pi";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "node";
        fields[pane_line_field::SUBAGENT_KIND] = "spawned";
        fields[pane_line_field::SUBAGENT_PARENT] = "P1";
        fields[pane_line_field::SUBAGENT_TASK] = "unit one";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();

        let info = pane
            .subagent
            .expect("the three options should mark the pane as a subagent");
        assert_eq!(info.kind, PmuxTaskKind::Spawned);
        assert_eq!(info.task.as_deref(), Some("unit one"));
        assert_eq!(info.parent, SubagentParent::Orphaned);
    }

    #[test]
    fn parse_pane_line_subagent_empty_task_means_no_task() {
        let mut fields = full_fields();
        fields[pane_line_field::AGENT] = "pi";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "node";
        fields[pane_line_field::SUBAGENT_KIND] = "resumed";
        fields[pane_line_field::SUBAGENT_PARENT] = "P1";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();

        let info = pane.subagent.expect("a known kind value marks the pane");
        assert_eq!(info.kind, PmuxTaskKind::Resumed);
        assert_eq!(info.task, None, "an empty task option means no task");
    }

    #[test]
    fn parse_pane_line_unknown_subagent_kind_is_not_a_subagent() {
        let mut fields = full_fields();
        fields[pane_line_field::AGENT] = "pi";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "node";
        fields[pane_line_field::SUBAGENT_KIND] = "maybe";
        fields[pane_line_field::SUBAGENT_PARENT] = "P1";
        fields[pane_line_field::SUBAGENT_TASK] = "unit one";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();

        assert!(
            pane.subagent.is_none(),
            "an unknown kind value must not mark the pane"
        );
    }

    #[test]
    fn parse_pane_line_ignores_subagent_options_for_non_pi_agents() {
        // The `@pane_subagent_*` options are pi-only. A pane that flipped
        // from pi to another agent without an intervening shell keeps the
        // stale options (no sweep runs and SessionStart preserves them),
        // so the reader must gate on the pane's agent the way the
        // neighbouring `permission_mode` read does.
        let mut fields = full_fields();
        fields[pane_line_field::SUBAGENT_KIND] = "spawned";
        fields[pane_line_field::SUBAGENT_PARENT] = "P1";
        fields[pane_line_field::SUBAGENT_TASK] = "unit one";
        let pane = parse_pane_line(&make_pane_line(&fields)).unwrap();

        assert!(
            pane.subagent.is_none(),
            "a non-pi pane must not be reported as a subagent even with the options set"
        );
    }

    #[test]
    fn clear_agent_pane_state_clears_every_agent_session_option() {
        let _guard = test_mock::install();
        let pane = "%SWEEP_ALL";
        for key in AGENT_SESSION_PANE_OPTIONS {
            test_mock::set(pane, key, "x");
        }

        clear_agent_pane_state(pane);

        for key in AGENT_SESSION_PANE_OPTIONS {
            assert!(
                !test_mock::contains(pane, key),
                "{key} must be cleared by the shell-fallback sweep"
            );
        }
    }

    #[test]
    fn parse_pane_line_rejects_sidebar_role() {
        let mut fields = full_fields();
        fields[7] = "sidebar";
        let line = make_pane_line(&fields);
        assert!(
            parse_pane_line(&line).is_none(),
            "sidebar role should be filtered out"
        );
    }

    #[test]
    fn parse_pane_line_rejects_unknown_agent() {
        let mut fields = full_fields();
        fields[3] = ""; // no agent type
        let line = make_pane_line(&fields);
        assert!(
            parse_pane_line(&line).is_none(),
            "empty agent should be rejected"
        );
    }

    #[test]
    fn parse_pane_line_falls_back_to_pane_current_path() {
        let mut fields = full_fields();
        fields[15] = ""; // empty pane_cwd
        let line = make_pane_line(&fields);
        let pane = parse_pane_line(&line).unwrap();
        assert_eq!(
            pane.path, "/home/user/project",
            "should fall back to pane_current_path when pane_cwd is empty"
        );
    }

    #[test]
    fn parse_pane_line_preserves_pipe_in_path() {
        let mut fields = full_fields();
        fields[5] = "/home/user/a\\|b";
        fields[15] = "";
        let line = make_pane_line(&fields);
        let pane = parse_pane_line(&line).unwrap();
        assert_eq!(pane.path, "/home/user/a|b");
    }

    #[test]
    fn split_tmux_fields_unescapes_delimiter() {
        let fields = split_tmux_fields("one|two\\|still-two|three", '|');
        assert_eq!(fields, vec!["one", "two|still-two", "three"]);
    }

    #[test]
    fn parse_pane_line_wipes_stale_state_for_pi_shell_pane() {
        // A pi process can be killed without a shutdown event reaching the
        // sidebar, so a pi pane that fell back to the shell is swept the
        // same way a Codex or an OpenCode pane is.
        let _guard = test_mock::install();
        let pane = "%PI_STALE";
        test_mock::set(pane, PANE_AGENT, "pi");
        test_mock::set(pane, PANE_PROMPT, "previous pi prompt");
        test_mock::set(pane, PANE_PROMPT_SOURCE, "user");
        test_mock::set(pane, PANE_STATUS, "running");
        test_mock::set(pane, PANE_CWD, "/repo");
        test_mock::set(pane, PANE_SESSION_ID, "ses-pi");
        let log = crate::activity::log_file_path(pane);
        let _ = std::fs::create_dir_all(log.parent().unwrap());
        std::fs::write(&log, "1234|Read|main.rs\n").unwrap();

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "pi";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "zsh";
        let line = make_pane_line(&fields);

        assert!(parse_pane_line(&line).is_none());
        for key in &[
            PANE_AGENT,
            PANE_PROMPT,
            PANE_PROMPT_SOURCE,
            PANE_STATUS,
            PANE_CWD,
        ] {
            assert!(
                !test_mock::contains(pane, key),
                "{key} must be cleared when a pi pane falls back to shell"
            );
        }
        assert!(
            !log.exists(),
            "pi activity log must be removed once the agent process is gone"
        );
    }

    #[test]
    fn parse_pane_fields_keeps_pi_shell_pane_when_process_is_alive() {
        let _guard = test_mock::install();
        let pane = "%PI_LIVE";
        test_mock::set(pane, PANE_AGENT, "pi");
        test_mock::set(pane, PANE_PROMPT, "keep me");

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "pi";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "zsh";
        fields[pane_line_field::PANE_PID] = "300";
        let fields = field_strings(&fields);
        let snapshot = process_snapshot("300 1 zsh zsh -c pi\n301 300 pi pi\n");

        let pane_info = parse_pane_fields_with_processes(&fields, Some(&snapshot))
            .expect("live pi child process should keep pane visible");

        assert_eq!(pane_info.agent, AgentType::Pi);
        assert!(test_mock::contains(pane, PANE_AGENT));
        assert_eq!(
            test_mock::get(pane, PANE_PROMPT).as_deref(),
            Some("keep me"),
            "live pi panes must not be swept just because tmux reports a shell"
        );
    }

    #[test]
    fn pi_pane_needs_a_process_snapshot() {
        // A pi pane is swept when its process is gone, so the sidebar must
        // build the process snapshot even when pi is the only agent on
        // screen. Without the snapshot the sweep sees no live agent and
        // clears the pane.
        let mut fields = full_fields();
        fields[pane_line_field::AGENT] = "pi";
        let mut line: Vec<String> = vec![
            "sess".into(),
            "3".into(),
            "1".into(),
            "win".into(),
            "1".into(),
            "0".into(),
        ];
        line.extend(field_strings(&fields));
        let output = line.join("|");
        assert!(
            pane_output_needs_process_snapshot(&output),
            "a pi pane must make the sidebar build a process snapshot"
        );
    }

    #[test]
    fn claude_only_panes_need_no_process_snapshot() {
        let mut fields = full_fields();
        fields[pane_line_field::AGENT] = "claude";
        let mut line: Vec<String> = vec![
            "sess".into(),
            "3".into(),
            "1".into(),
            "win".into(),
            "1".into(),
            "0".into(),
        ];
        line.extend(field_strings(&fields));
        let output = line.join("|");
        assert!(!pane_output_needs_process_snapshot(&output));
    }

    #[test]
    fn parse_pane_line_codex_ignores_permission_mode_field() {
        let mut fields = full_fields();
        fields[3] = "codex";
        fields[6] = "node";
        fields[16] = "auto"; // should be ignored for codex
        let line = make_pane_line(&fields);
        let pane = parse_pane_line(&line).unwrap();
        assert_eq!(
            pane.permission_mode,
            PermissionMode::Default,
            "codex should not read permission_mode from tmux variable"
        );
    }

    #[test]
    fn parse_pane_line_rejects_stale_codex_shell_pane() {
        let mut fields = full_fields();
        fields[3] = "codex";
        fields[6] = "zsh";
        let line = make_pane_line(&fields);
        assert!(
            parse_pane_line(&line).is_none(),
            "codex metadata on a shell pane should be treated as stale"
        );
    }

    #[test]
    fn parse_pane_line_rejects_stale_codex_shell_pane_with_path_and_args() {
        let mut fields = full_fields();
        fields[3] = "codex";
        fields[6] = "/usr/local/bin/PwSh -l";
        let line = make_pane_line(&fields);
        assert!(
            parse_pane_line(&line).is_none(),
            "shell detection should handle paths, args, and case differences"
        );
    }

    #[test]
    fn parse_pane_line_does_not_wipe_claude_on_shell_command() {
        // Claude owns its own SessionEnd hook, so Rust must NOT sweep its
        // pane state even when `current_command` is a shell — otherwise
        // we race with the hook and lose legitimate prompt/status.
        let _guard = test_mock::install();
        let pane = "%CLAUDE_SHELL";
        test_mock::set(pane, PANE_AGENT, "claude");
        test_mock::set(pane, PANE_PROMPT, "keep me");
        test_mock::set(pane, PANE_STATUS, "running");

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "claude";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "bash";
        let _ = parse_pane_line(&make_pane_line(&fields));

        assert!(test_mock::contains(pane, PANE_AGENT));
        assert!(test_mock::contains(pane, PANE_PROMPT));
        assert_eq!(
            test_mock::get(pane, PANE_PROMPT).as_deref(),
            Some("keep me"),
            "claude prompt must survive — SessionEnd hook is the clear path",
        );
    }

    #[test]
    fn parse_pane_fields_keeps_opencode_shell_pane_when_process_is_alive() {
        let _guard = test_mock::install();
        let pane = "%OPENCODE_LIVE";
        test_mock::set(pane, PANE_AGENT, "opencode");
        test_mock::set(pane, PANE_PROMPT, "keep me");

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "opencode";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "fish";
        fields[pane_line_field::PANE_PID] = "100";
        let fields = field_strings(&fields);
        let snapshot = process_snapshot("100 1 fish fish -c opencode\n101 100 opencode opencode\n");

        let pane_info = parse_pane_fields_with_processes(&fields, Some(&snapshot))
            .expect("live OpenCode child process should keep pane visible");

        assert_eq!(pane_info.agent, AgentType::OpenCode);
        assert!(test_mock::contains(pane, PANE_AGENT));
        assert_eq!(
            test_mock::get(pane, PANE_PROMPT).as_deref(),
            Some("keep me"),
            "live OpenCode panes must not be swept just because tmux reports a shell"
        );
    }

    #[test]
    fn parse_pane_fields_keeps_codex_shell_pane_when_process_is_alive() {
        let _guard = test_mock::install();
        let pane = "%CODEX_LIVE";
        test_mock::set(pane, PANE_AGENT, "codex");
        test_mock::set(pane, PANE_PROMPT, "keep me");

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "codex";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "zsh";
        fields[pane_line_field::PANE_PID] = "200";
        let fields = field_strings(&fields);
        let snapshot = process_snapshot(
            "200 1 zsh zsh -c codex\n201 200 codex /opt/homebrew/bin/codex --full-auto\n",
        );

        let pane_info = parse_pane_fields_with_processes(&fields, Some(&snapshot))
            .expect("live Codex child process should keep pane visible");

        assert_eq!(pane_info.agent, AgentType::Codex);
        assert!(test_mock::contains(pane, PANE_AGENT));
        assert_eq!(
            test_mock::get(pane, PANE_PROMPT).as_deref(),
            Some("keep me"),
            "live Codex panes must not be swept just because tmux reports a shell"
        );
    }

    #[test]
    fn parse_pane_line_wipes_stale_state_for_codex_shell_pane() {
        // Codex shares the same shell-fallback sweep path as OpenCode —
        // neither fires a reliable process-exit hook, so the Rust poller
        // must clear @pane_* keys and the activity log when the pane
        // reverts to a shell. Mirrors the OpenCode regression test below.
        let _guard = test_mock::install();
        let pane = "%CODEX_STALE";
        test_mock::set(pane, PANE_AGENT, "codex");
        test_mock::set(pane, PANE_PROMPT, "previous codex prompt");
        test_mock::set(pane, PANE_PROMPT_SOURCE, "user");
        test_mock::set(pane, PANE_STATUS, "waiting");
        test_mock::set(pane, PANE_STARTED_AT, "1700000000");
        test_mock::set(pane, PANE_CWD, "/repo/codex");
        test_mock::set(pane, PANE_WAIT_REASON, "permission");
        let log = crate::activity::log_file_path(pane);
        let _ = std::fs::create_dir_all(log.parent().unwrap());
        std::fs::write(&log, "1234|Bash|pytest\n").unwrap();

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "codex";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "zsh";
        let line = make_pane_line(&fields);

        assert!(parse_pane_line(&line).is_none());
        for key in &[
            PANE_AGENT,
            PANE_PROMPT,
            PANE_PROMPT_SOURCE,
            PANE_STATUS,
            PANE_STARTED_AT,
            PANE_CWD,
            PANE_WAIT_REASON,
        ] {
            assert!(
                !test_mock::contains(pane, key),
                "{key} must be cleared when a codex pane falls back to shell"
            );
        }
        assert!(
            !log.exists(),
            "codex activity log must be removed once the agent process is gone"
        );
    }

    #[test]
    fn parse_pane_line_wipes_stale_state_for_opencode_shell_pane() {
        // When an OpenCode pane falls back to the user's shell, the Rust
        // polling side owns teardown because OpenCode has no reliable
        // process-exit hook. The detector must unset every @pane_* key it
        // seeded and remove the activity log so the next launch starts
        // from a clean slate without flashing stale prompt/status.
        let _guard = test_mock::install();
        let pane = "%OPENCODE_STALE";
        test_mock::set(pane, PANE_AGENT, "opencode");
        test_mock::set(pane, PANE_PROMPT, "previous run");
        test_mock::set(pane, PANE_PROMPT_SOURCE, "user");
        test_mock::set(pane, PANE_STATUS, "running");
        test_mock::set(pane, PANE_STARTED_AT, "1700000000");
        test_mock::set(pane, PANE_CWD, "/repo");
        test_mock::set(pane, PANE_SESSION_ID, "ses-1");
        let log = crate::activity::log_file_path(pane);
        let _ = std::fs::create_dir_all(log.parent().unwrap());
        std::fs::write(&log, "1234|Bash|ls\n").unwrap();

        let mut fields = full_fields();
        fields[pane_line_field::PANE_ID] = pane;
        fields[pane_line_field::AGENT] = "opencode";
        fields[pane_line_field::PANE_CURRENT_COMMAND] = "fish";
        let line = make_pane_line(&fields);

        assert!(parse_pane_line(&line).is_none());
        for key in &[
            PANE_AGENT,
            PANE_PROMPT,
            PANE_PROMPT_SOURCE,
            PANE_STATUS,
            PANE_STARTED_AT,
            PANE_CWD,
            PANE_SESSION_ID,
        ] {
            assert!(
                !test_mock::contains(pane, key),
                "{key} must be cleared after shell fallback sweep"
            );
        }
        assert!(
            !log.exists(),
            "activity log must be removed when the agent process is gone"
        );
    }

    // ─── finalize_sessions ─────────────────────────────────────────

    #[test]
    fn finalize_sessions_drops_windows_with_no_panes() {
        // Regression: build_session_hierarchy() creates a WindowInfo as
        // soon as it sees a tmux row, but parse_pane_line() may then
        // reject every pane in that window (sidebar / shell / unknown).
        // finalize_sessions must filter out the resulting empty windows
        // so downstream code never has to special-case them.
        let mut sessions_map: SessionMap = indexmap::IndexMap::new();
        let entry = sessions_map.entry("main".to_string()).or_default();
        entry.insert(
            "@1".to_string(),
            WindowInfo {
                window_id: "@1".into(),
                window_name: "with-pane".into(),
                window_active: true,
                auto_rename: false,
                panes: vec![PaneInfo {
                    pane_id: "%1".into(),
                    pane_active: true,
                    status: PaneStatus::Running,
                    attention: false,
                    agent: AgentType::Claude,
                    path: "/repo".into(),
                    current_command: String::new(),
                    prompt: String::new(),
                    prompt_is_response: false,
                    started_at: None,
                    wait_reason: String::new(),
                    permission_mode: PermissionMode::Default,
                    subagents: vec![],
                    pane_pid: None,
                    worktree: WorktreeMetadata::default(),
                    session_id: None,
                    sidebar_spawned: false,
                    pane_name: None,
                    bg_shell_cmd: None,
                    subagent: None,
                }],
            },
        );
        entry.insert(
            "@2".to_string(),
            WindowInfo {
                window_id: "@2".into(),
                window_name: "empty".into(),
                window_active: false,
                auto_rename: false,
                panes: vec![],
            },
        );

        let sessions = finalize_sessions(sessions_map);

        assert_eq!(sessions.len(), 1, "session should survive");
        assert_eq!(
            sessions[0].windows.len(),
            1,
            "empty window must be filtered out"
        );
        assert_eq!(sessions[0].windows[0].window_id, "@1");
    }

    #[test]
    fn finalize_sessions_drops_session_when_all_windows_are_empty() {
        let mut sessions_map: SessionMap = indexmap::IndexMap::new();
        let entry = sessions_map.entry("dead".to_string()).or_default();
        entry.insert(
            "@9".to_string(),
            WindowInfo {
                window_id: "@9".into(),
                window_name: "ghost".into(),
                window_active: false,
                auto_rename: false,
                panes: vec![],
            },
        );

        let sessions = finalize_sessions(sessions_map);

        assert!(sessions.is_empty(), "session with no panes must be dropped");
    }

    // ─── build_session_hierarchy dedup ─────────────────────────────

    /// Construct a minimal valid pane line for `build_session_hierarchy`
    /// with the given session name and pane_pid. All other fields are
    /// empty/defaults — enough to survive parsing as an opencode pane.
    fn make_full_pane_line(session_name: &str, pane_pid: u32) -> String {
        // Field layout (pane_format):
        // 0:session_name|1:window_id|2:window_index|3:window_name|
        // 4:window_active|5:automatic-rename|6:pane_active|7:@pane_status|
        // 8:@pane_attention|9:@pane_agent|10:@pane_name|
        // 11:pane_current_path|12:pane_current_command|13:@pane_role|
        // 14:pane_id|15:@pane_prompt|16:@pane_prompt_source|
        // 17:@pane_started_at|18:@pane_wait_reason|19:pane_pid|
        // 20:@pane_subagents|21:@pane_cwd|22:@pane_permission_mode|
        // 23:@pane_worktree_name|24:@pane_worktree_branch|
        // 25:@pane_session_id|26:@pmux-explorer-spawned|27:@pane_bg_cmd|
        // 28:@pane_subagent_kind|29:@pane_subagent_parent|
        // 30:@pane_subagent_task|31:pane_title|32:host

        // 33 total fields (MIN_FIELDS = 33)
        let mut fields: Vec<&str> = vec![""; 33];
        fields[0] = session_name;
        fields[1] = "@0"; // window_id
        fields[3] = "win"; // window_name
        fields[4] = "1"; // window_active
        fields[9] = "opencode"; // @pane_agent
        fields[11] = "/tmp"; // pane_current_path
        fields[14] = "%0"; // pane_id
        fields[21] = "/tmp"; // @pane_cwd
        let pid_str = pane_pid.to_string();
        fields[19] = &pid_str; // pane_pid
        fields.join("|")
    }

    #[test]
    fn build_session_hierarchy_dedups_nonzero_pane_pid_only() {
        // Two rows with same non-zero pane_pid: only the first is retained.
        let line_a = make_full_pane_line("primary", 42);
        let line_b = make_full_pane_line("grouped", 42);
        // Two more rows with pane_pid = 0: both retained (no dedup on zero).
        let line_c = make_full_pane_line("primary", 0);
        let line_d = make_full_pane_line("grouped", 0);

        let input = format!("{line_a}\n{line_b}\n{line_c}\n{line_d}");
        let (sessions_map, _, _) = build_session_hierarchy(&input, None);
        let sessions = finalize_sessions(sessions_map);

        // Should produce two sessions: "primary" and "grouped"
        assert_eq!(sessions.len(), 2);

        let primary = sessions
            .iter()
            .find(|s| s.session_name == "primary")
            .unwrap();
        let grouped = sessions
            .iter()
            .find(|s| s.session_name == "grouped")
            .unwrap();

        // primary: non-zero pane (42) + zero pane (0) — both retained
        assert_eq!(
            primary.windows[0].panes.len(),
            2,
            "primary session should have both pane 42 and pane 0"
        );

        // grouped: panes with pane_pid 42 and 42 are duplicates,
        // so 42 is skipped. Only pane 0 is retained (no dedup on zero).
        assert_eq!(
            grouped.windows[0].panes.len(),
            1,
            "grouped session should only have pane 0 (pane 42 was deduped)"
        );
        assert_eq!(
            grouped.windows[0].panes[0].pane_pid,
            Some(0),
            "retained pane should be the zero-pid one"
        );
    }

    // ─── subagent parent resolution ──────────────────────────────────

    /// Build a full 32-field hierarchy line for the parent-resolution
    /// tests: an agent pane in `session_name` with the given pane id,
    /// `@pane_session_id`, `pane_title`, and optionally the three
    /// `@pane_subagent_*` options (kind, parent session id, task).
    #[allow(clippy::type_complexity)]
    fn make_hierarchy_line(
        session_name: &str,
        pane_id: &str,
        session_id: &str,
        title: &str,
        subagent: Option<(&str, &str, &str)>,
    ) -> String {
        let mut fields: Vec<&str> = vec![""; 33];
        fields[0] = session_name;
        fields[1] = "@0"; // window_id
        fields[3] = "win"; // window_name
        fields[4] = "1"; // window_active
        fields[9] = "pi"; // @pane_agent
        fields[14] = pane_id;
        fields[25] = session_id;
        if let Some((kind, parent, task)) = subagent {
            fields[28] = kind;
            fields[29] = parent;
            fields[30] = task;
        }
        fields[31] = title;
        fields.join("|")
    }

    /// Parse the lines, run the parent-resolution post-pass, and flatten
    /// the result into the parsed panes.
    fn resolve_in(input: &str) -> Vec<PaneInfo> {
        let (sessions_map, _, resolution) = build_session_hierarchy(input, None);
        let mut sessions_map = sessions_map;
        resolve_subagent_parents(&mut sessions_map, &resolution);
        finalize_sessions(sessions_map)
            .into_iter()
            .flat_map(|session| session.windows.into_iter().flat_map(|window| window.panes))
            .collect()
    }

    fn subagent_parent_of(panes: &[PaneInfo], pane_id: &str) -> SubagentParent {
        let pane = panes
            .iter()
            .find(|pane| pane.pane_id == pane_id)
            .unwrap_or_else(|| panic!("pane {pane_id} should be parsed"));
        pane.subagent
            .as_ref()
            .unwrap_or_else(|| panic!("pane {pane_id} should be a subagent"))
            .parent
            .clone()
    }

    #[test]
    fn resolve_subagent_parents_uses_the_parent_title() {
        let input = format!(
            "{}\n{}",
            make_hierarchy_line("main", "%0", "P1", "orchestrator", None),
            make_hierarchy_line("main", "%1", "", "", Some(("spawned", "P1", ""))),
        );
        let panes = resolve_in(&input);
        assert_eq!(
            subagent_parent_of(&panes, "%1"),
            SubagentParent::Pane("orchestrator".to_string()),
            "a live parent with a title resolves to that title"
        );
    }

    #[test]
    fn resolve_subagent_parents_falls_back_to_the_pane_id_without_a_title() {
        let input = format!(
            "{}\n{}",
            make_hierarchy_line("main", "%0", "P1", "", None),
            make_hierarchy_line("main", "%1", "", "", Some(("resumed", "P1", ""))),
        );
        let panes = resolve_in(&input);
        assert_eq!(
            subagent_parent_of(&panes, "%1"),
            SubagentParent::Pane("%0".to_string()),
            "a live parent with an empty title resolves to its pane id"
        );
    }

    #[test]
    fn resolve_subagent_parents_leaves_a_subagent_orphaned_without_a_parent() {
        let input = make_hierarchy_line("main", "%1", "", "", Some(("spawned", "P1", "")));
        let panes = resolve_in(&input);
        assert_eq!(
            subagent_parent_of(&panes, "%1"),
            SubagentParent::Orphaned,
            "no pane reports the recorded session id, so the subagent is orphaned"
        );
    }

    #[test]
    fn resolve_subagent_parents_resolves_a_parent_in_another_session() {
        let input = format!(
            "{}\n{}",
            make_hierarchy_line("other", "%0", "P1", "orchestrator", None),
            make_hierarchy_line("main", "%1", "", "", Some(("spawned", "P1", ""))),
        );
        let panes = resolve_in(&input);
        assert_eq!(
            subagent_parent_of(&panes, "%1"),
            SubagentParent::Pane("orchestrator".to_string()),
            "the parent lookup spans every session in the list-panes output"
        );
    }
}
