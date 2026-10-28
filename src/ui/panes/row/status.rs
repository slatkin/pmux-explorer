use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::ctx::RowCtx;
use crate::pmux_task::PmuxTaskKind;
use crate::tmux::PaneStatus;
use crate::ui::icons::StatusIcons;
use crate::ui::text::{display_width, elapsed_label, truncate_to_width};

pub(super) fn status_row(
    pane: &crate::tmux::PaneInfo,
    ctx: &RowCtx,
    icons: &StatusIcons,
    spinner_frame: usize,
    now: u64,
) -> Line<'static> {
    use crate::tmux::PermissionMode;
    let theme = ctx.theme;

    let (icon, pulse_color) = running_icon_for(&pane.status, spinner_frame, icons);
    let icon_color =
        pulse_color.unwrap_or_else(|| theme.status_color(&pane.status, pane.attention));
    // The agent label is always shown; a real pane name (set via pmux's
    // `select-pane -T`) is appended after the badge slot, so the subagent
    // badge (♟ / ↻) and a permission badge sit between the agent label and
    // the pane name. Only the pane's own name is shown in the title slot —
    // never the tmux session name. OpenCode renders its label as a padded
    // Nerd glyph chip on a black background instead of the word, since the
    // agent's own OSC title ("OpenCode") is filtered from the name slot and
    // would only duplicate the label.
    let agent_is_opencode = pane.agent == crate::tmux::AgentType::OpenCode;
    let agent_label = pane.agent.label();
    // A subagent pane owns the badge slot: a pi pane always has the default
    // permission mode, whose badge is empty, so the pmux identity can use it.
    let (badge, badge_override) = match &pane.subagent {
        Some(info) => match info.kind {
            PmuxTaskKind::Spawned => ("♟", Some(theme.badge_plan)),
            PmuxTaskKind::Resumed => ("↻", Some(theme.badge_auto)),
        },
        None => (pane.permission_mode.badge(), None),
    };
    let elapsed = elapsed_label(pane.started_at, now);

    let title_fg = theme.agent_color(&pane.agent);
    let elapsed_fg = if pane.status.is_active() {
        theme.text_active
    } else {
        theme.text_muted
    };

    let badge_extra = if badge.is_empty() { 0 } else { 1 };
    let agent_width = if agent_is_opencode {
        1 + display_width(crate::ui::icons::OPENCODE_AGENT_GLYPH)
    } else {
        1 + display_width(agent_label)
    };
    let fixed_width = display_width(icon) + agent_width + badge_extra + display_width(badge);
    // Reserve room for the icon, the agent label, the badge, and the
    // elapsed label; the pane name gets what remains and takes the
    // ellipsis when cut. The agent label itself is never truncated, and
    // the elapsed-truncation fallback below stays in place.
    let elapsed_width = display_width(&elapsed);
    let elapsed_gap = usize::from(elapsed_width > 0);
    let title_budget = ctx
        .inner_width
        .saturating_sub(fixed_width + elapsed_gap + elapsed_width);
    let name_budget = title_budget.saturating_sub(1);
    let pane_name = pane
        .pane_name
        .as_deref()
        .filter(|_| name_budget > 0)
        .map(|name| truncate_to_width(name, name_budget));

    let mut left_width = fixed_width;
    if let Some(name) = &pane_name {
        left_width += 1 + display_width(name);
    }
    let available_for_elapsed = ctx.inner_width.saturating_sub(left_width);
    let elapsed = truncate_to_width(&elapsed, available_for_elapsed);
    let elapsed_width = display_width(&elapsed);

    let mut left_spans: Vec<Span<'static>> = Vec::with_capacity(4);
    left_spans.push(Span::styled(
        icon.to_string(),
        ctx.apply_bg(Style::default().fg(icon_color)),
    ));
    if agent_is_opencode {
        // One row-background cell after the status icon first, so the chip
        // sits in the same column as the other agents' labels instead of
        // touching the status icon.
        left_spans.push(Span::styled(" ", ctx.apply_bg(Style::default())));
        left_spans.push(Span::styled(
            crate::ui::icons::OPENCODE_AGENT_GLYPH.to_string(),
            // Literal black/white, not ANSI: the chip must stay a white
            // glyph on black under any terminal palette.
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(0, 0, 0)),
        ));
    } else {
        left_spans.push(Span::styled(
            format!(" {agent_label}"),
            ctx.apply_bg(Style::default().fg(title_fg)),
        ));
    }
    if !badge.is_empty() {
        let badge_color = badge_override.unwrap_or(match pane.permission_mode {
            PermissionMode::BypassPermissions => theme.badge_danger,
            PermissionMode::Auto => theme.badge_auto,
            PermissionMode::DontAsk => theme.badge_auto,
            PermissionMode::Plan => theme.badge_plan,
            PermissionMode::AcceptEdits => theme.badge_auto,
            PermissionMode::Defer => theme.badge_auto,
            PermissionMode::Default => theme.text_muted,
        });
        left_spans.push(Span::styled(
            format!(" {}", badge),
            ctx.apply_bg(Style::default().fg(badge_color)),
        ));
    }
    if let Some(name) = &pane_name {
        left_spans.push(Span::styled(
            format!(" {name}"),
            ctx.apply_bg(Style::default().fg(title_fg)),
        ));
    }

    let right_spans = vec![Span::styled(
        elapsed,
        ctx.apply_bg(Style::default().fg(elapsed_fg)),
    )];

    ctx.row_line_split(left_spans, left_width, right_spans, elapsed_width)
}

pub(super) fn running_icon_for<'a>(
    status: &PaneStatus,
    spinner_frame: usize,
    icons: &'a StatusIcons,
) -> (&'a str, Option<Color>) {
    use crate::SPINNER_PULSE;

    match status {
        PaneStatus::Running => {
            let color_idx = SPINNER_PULSE[spinner_frame % SPINNER_PULSE.len()];
            (icons.status_icon(status), Some(Color::Indexed(color_idx)))
        }
        _ => (icons.status_icon(status), None),
    }
}
