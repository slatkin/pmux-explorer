//! Shared floor/adopt/resize helpers for the always-on docked-size
//! sync: the column width and the docked strip height every sidebar
//! instance converges on through the shared tmux options
//! (`@sidebar_docked_width` / `@sidebar_docked_height`). There is no
//! toggle — the publish-or-adopt machine in `app/width_guard.rs` runs
//! unconditionally, and these helpers carry the floor, clamp and
//! resize-argument math it shares with the pop-in restore
//! (`state/popout.rs`).

use crate::app::size_guard::DEFAULT_MIN_WIDTH;
use crate::state::LayoutKind;

/// The width-lock floor for the tick's parsed `@sidebar_min_width` in
/// columns ([`GlobalState::min_width`]). `None` falls back to
/// [`DEFAULT_MIN_WIDTH`], and a floor beyond pane geometry is clamped
/// into the u16 range — the u32 option value cannot silently wrap in
/// the u16 math of the three users below (seed, adopt, pop-in).
pub(crate) fn width_floor(min_width: Option<u32>) -> u16 {
    min_width.unwrap_or(DEFAULT_MIN_WIDTH).min(u16::MAX as u32) as u16
}

/// The minimum docked strip height in rows. The strip floor is the
/// fixed constant, not an option: the floor's job is stopping
/// collapse, not readability, and a strip has no
/// `@sidebar_min_height` option (design D3 of the strip-height-sync
/// change). It binds the sync step's publishes and adoptions and the
/// pop-in restore, and wins over the adoption cap where the two cannot
/// both hold — mirroring `width_floor`.
pub const DEFAULT_MIN_STRIP_HEIGHT: u16 = 4;

/// The strip-height floor for the sync's math — the height-axis twin
/// of [`width_floor`]. The strip floor is constant, so the u16 value
/// needs no clamp; the helper keeps the axis rules uniform at the
/// call sites the width option's floor and this constant feed.
pub fn strip_height_floor() -> u16 {
    DEFAULT_MIN_STRIP_HEIGHT
}

/// The size an adopting sidebar resizes to with `resize-pane`: the
/// shared docked size clamped to at least `floor` — the floor binds
/// both ways — and at most `cap`, normally half the adopting window's
/// matching dimension. The cap binds adopted sizes only; a publishing
/// sidebar may stay past half its window. The floor comes resolved:
/// the width option's floor on the column axis, the strip-height
/// constant on the strip axis. The two users per axis: the size-sync
/// step's adopt side (`app/width_guard::step`) and the pop-in restore
/// (`state/popout::pop_in_restore_size`).
pub(crate) fn adopt_target(shared: u16, floor: u16, cap: u16) -> u16 {
    shared.max(floor).min(cap)
}

/// The `resize-pane` argument list that applies a docked size on the
/// axis: `-x` for a column, `-y` for a strip. Pure, like
/// `split_window_args` in `cli/toggle.rs`, so a unit test can assert
/// the exact arguments the two issuers — the sync step's adopt side
/// (`app/width_guard::enforce`) and the pop-in restore
/// (`state/popout`) — hand to tmux per axis.
pub(crate) fn resize_pane_args(pane: &str, axis: LayoutKind, size: u16) -> Vec<String> {
    let flag = match axis {
        LayoutKind::Column => "-x",
        LayoutKind::Strip => "-y",
    };
    vec![
        "resize-pane".to_string(),
        "-t".to_string(),
        pane.to_string(),
        flag.to_string(),
        size.to_string(),
    ]
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn width_floor_none_falls_back_to_default() {
        assert_eq!(width_floor(None), DEFAULT_MIN_WIDTH as u16);
        assert_eq!(width_floor(Some(24)), 24);
        assert_eq!(width_floor(Some(u32::MAX)), u16::MAX, "clamped at u16::MAX");
    }

    #[test]
    fn adopt_target_floors_and_caps() {
        assert_eq!(adopt_target(80, 20, 100), 80);
        assert_eq!(adopt_target(8, 20, 100), 20, "the floor binds both ways");
        assert_eq!(adopt_target(80, 20, 50), 50, "the cap clamps adopted sizes");
    }

    #[test]
    fn strip_height_floor_is_the_documented_constant() {
        assert_eq!(strip_height_floor(), 4);
        assert_eq!(strip_height_floor(), DEFAULT_MIN_STRIP_HEIGHT);
    }

    #[test]
    fn resize_pane_args_resizes_on_the_axis() {
        // Pure argv assertion for the shared resize call both issuers
        // run: `-y` rows for a strip, `-x` columns for a column.
        assert_eq!(
            resize_pane_args("%9", LayoutKind::Strip, 12),
            vec![
                "resize-pane".to_string(),
                "-t".to_string(),
                "%9".to_string(),
                "-y".to_string(),
                "12".to_string(),
            ]
        );
        assert_eq!(
            resize_pane_args("%9", LayoutKind::Column, 45),
            vec![
                "resize-pane".to_string(),
                "-t".to_string(),
                "%9".to_string(),
                "-x".to_string(),
                "45".to_string(),
            ]
        );
    }
}
