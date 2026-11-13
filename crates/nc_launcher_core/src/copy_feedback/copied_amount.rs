pub const COPIED_HOLD_SECONDS: f32 = 1.2;
pub const COPIED_EASE_SECONDS: f32 = 0.3;

/// How far a copy-on-click line shows its "copied" state, `seconds` after the click: fully for the hold, then
/// eased back to rest. Zero before a click and after the ease.
pub fn copied_amount(seconds: f32) -> f32 {
    if !(0.0..COPIED_HOLD_SECONDS + COPIED_EASE_SECONDS).contains(&seconds) {
        return 0.0;
    }
    if seconds < COPIED_HOLD_SECONDS {
        return 1.0;
    }
    let t = (seconds - COPIED_HOLD_SECONDS) / COPIED_EASE_SECONDS;
    // Smoothstep, so the inverted fill leaves without a visible jump.
    1.0 - t * t * (3.0 - 2.0 * t)
}
