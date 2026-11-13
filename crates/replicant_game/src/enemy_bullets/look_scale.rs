use core::ops::Range;

use replicant_proto::BulletLook;

use crate::bullet_flights::style_is_set;

const STYLE_SCALE_INDICES: Range<usize> = 4..8;

/// Whether any slot of `now` is drawn at another scale than in `before`. The game grows a widening
/// ordinary bullet's effects each frame with its hit radius.
pub fn look_resized(before: &BulletLook, now: &BulletLook) -> bool {
    before
        .styles
        .iter()
        .zip(&now.styles)
        .any(|(was, is)| was[STYLE_SCALE_INDICES] != is[STYLE_SCALE_INDICES])
}

/// The largest scale axis among the look's styled slots, or 0 when no slot is styled.
pub fn look_scale(look: &BulletLook) -> f32 {
    look.styles
        .iter()
        .filter(|style| style_is_set(style))
        .flat_map(|style| style[STYLE_SCALE_INDICES].iter().copied())
        .fold(0.0, f32::max)
}
