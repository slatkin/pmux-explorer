use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::ctx::RowCtx;
use crate::tmux::PaneStatus;
use crate::ui::text::{
    display_width, truncate_to_width, wait_reason_entry, wait_reason_label, wrap_text,
};

pub(super) fn task_progress_row(
    task_progress: Option<&crate::activity::TaskProgress>,
    ctx: &RowCtx,
) -> Option<Line<'static>> {
    use crate::activity::TaskStatus;
    let progress = task_progress?;
    if progress.is_empty() {
        return None;
    }

    let mut icons = String::with_capacity(progress.tasks.len() * 3);
    for (_, status) in &progress.tasks {
        let ch = match status {
            TaskStatus::Completed => "✔",
            TaskStatus::InProgress => "◼",
            TaskStatus::Pending => "◻",
        };
        icons.push_str(ch);
    }
    let summary = format!(
        "  {} {}/{}",
        icons,
        progress.completed_count(),
        progress.total()
    );
    let summary_dw = display_width(&summary);
    let task_color = ctx.theme.task_progress;
    Some(ctx.row_line(
        vec![Span::styled(
            summary,
            ctx.apply_bg(Style::default().fg(task_color)),
        )],
        summary_dw,
    ))
}

pub(super) fn subagent_rows(subagents: &[String], ctx: &RowCtx) -> Vec<Line<'static>> {
    if subagents.is_empty() {
        return Vec::new();
    }
    let theme = ctx.theme;
    let subagent_color = theme.subagent;
    let tree_color = theme.text_muted;
    let last_idx = subagents.len() - 1;
    let mut out = Vec::with_capacity(subagents.len());
    for (i, sa) in subagents.iter().enumerate() {
        let connector = if i == last_idx { "└ " } else { "├ " };
        let numbered = if sa.contains('#') {
            sa.clone()
        } else {
            format!("{} #{}", sa, i + 1)
        };
        let prefix = format!("  {}", connector);
        let prefix_dw = display_width(&prefix);
        let max_sa_w = ctx.inner_width.saturating_sub(prefix_dw);
        let truncated_sa = truncate_to_width(&numbered, max_sa_w);
        let text_dw = prefix_dw + display_width(&truncated_sa);
        out.push(ctx.row_line(
            vec![
                Span::styled(prefix, ctx.apply_bg(Style::default().fg(tree_color))),
                Span::styled(
                    truncated_sa,
                    ctx.apply_bg(Style::default().fg(subagent_color)),
                ),
            ],
            text_dw,
        ));
    }
    out
}

pub(super) fn wait_reason_row(
    wait_reason: &str,
    status: &PaneStatus,
    ctx: &RowCtx,
    spinner_frame: usize,
) -> Option<Line<'static>> {
    if wait_reason.is_empty() {
        return None;
    }
    let reason = wait_reason_label(wait_reason);
    let text = format!("  {}", reason);
    let text_dw = display_width(&text);
    // A reason that demands the user is painted red, so it does not blend
    // into the agent title on the status row above it.
    let action_required = wait_reason_entry(wait_reason).is_some_and(|entry| entry.action_required);
    let reason_color = if matches!(status, PaneStatus::Error) {
        ctx.theme.status_error
    } else if action_required {
        // An action-required label blinks: it flips to the dimmed red for
        // the second half of each spinner cycle (~800 ms at the 200 ms
        // frame clock) — the same cadence the running icon pulses on — so
        // the demand for input reads as alive, not as static decoration.
        // The dim phase stays in the red family rather than greying out.
        // The chat prompt text itself never blinks.
        if spinner_frame >= crate::SPINNER_PULSE.len() / 2 {
            ctx.theme.wait_reason_action_dim
        } else {
            ctx.theme.wait_reason_action
        }
    } else {
        ctx.theme.wait_reason
    };
    Some(ctx.row_line(
        vec![Span::styled(
            text,
            ctx.apply_bg(Style::default().fg(reason_color)),
        )],
        text_dw,
    ))
}

pub(super) fn background_hint_row(ctx: &RowCtx, cmd: &str) -> Line<'static> {
    const PREFIX: &str = "  $ ";
    let room = ctx.inner_width.saturating_sub(display_width(PREFIX));
    let shown = truncate_to_width(cmd.trim(), room);
    let text = format!("{PREFIX}{shown}");
    let text_dw = display_width(&text);
    ctx.row_line(
        vec![Span::styled(
            text,
            ctx.apply_bg(Style::default().fg(ctx.theme.status_running)),
        )],
        text_dw,
    )
}

/// Glyph that heads the response preview line, so the line reads as the
/// agent's reply rather than the user's prompt. Includes the trailing
/// space the reply text starts after.
pub(super) const RESPONSE_ARROW_PREFIX: &str = "\u{f0b79} ";

pub(super) fn prompt_rows(pane: &crate::tmux::PaneInfo, ctx: &RowCtx) -> Vec<Line<'static>> {
    let theme = ctx.theme;
    let is_response = pane.prompt_is_response;
    let prompt_color = if ctx.active {
        theme.text_active
    } else {
        theme.text_inactive
    };
    let wrap_width = ctx.inner_width.saturating_sub(2);
    let wrapped = wrap_text(&pane.prompt, wrap_width, ctx.prompt_max_lines);

    let mut out = Vec::with_capacity(wrapped.len());
    for (li, wl) in wrapped.iter().enumerate() {
        if is_response && li == 0 {
            let arrow_color = theme.response_arrow;
            let text_dw = display_width(RESPONSE_ARROW_PREFIX) + display_width(wl);
            out.push(ctx.row_line(
                vec![
                    Span::styled(
                        RESPONSE_ARROW_PREFIX,
                        ctx.apply_bg(
                            Style::default()
                                .fg(arrow_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ),
                    Span::styled(wl.clone(), ctx.apply_bg(Style::default().fg(prompt_color))),
                ],
                text_dw,
            ));
        } else {
            let indent = "  ";
            let text = format!("{}{}", indent, wl);
            let text_dw = display_width(&text);
            out.push(ctx.row_line(
                vec![Span::styled(
                    text,
                    ctx.apply_bg(Style::default().fg(prompt_color)),
                )],
                text_dw,
            ));
        }
    }
    out
}

pub(super) fn idle_hint_row(ctx: &RowCtx) -> Line<'static> {
    let text = "  Waiting for prompt…";
    let text_dw = display_width(text);
    let idle_color = if ctx.active {
        ctx.theme.text_active
    } else {
        ctx.theme.text_inactive
    };
    ctx.row_line(
        vec![Span::styled(
            text.to_string(),
            ctx.apply_bg(Style::default().fg(idle_color)),
        )],
        text_dw,
    )
}
