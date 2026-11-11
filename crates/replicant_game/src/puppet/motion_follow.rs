use crate::puppet::{LayerMotion, MOTION_BLEND_SECONDS, MOTION_SEEK_TOLERANCE_FRAMES};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionStep {
    Keep,
    Start { blend: f32 },
}

/// Whether the puppet's body layer must start `index`. A motion in `last_started` is never
/// restarted because the read-back (None when unreadable) disagrees; frame drift is left to
/// [`frame_to_seek`].
pub fn follow_step(
    on_puppet: Option<LayerMotion>,
    last_started: Option<i32>,
    index: i32,
) -> MotionStep {
    match (on_puppet, last_started) {
        (Some(current), _) if current.index == index => MotionStep::Keep,
        (_, Some(started)) if started == index => MotionStep::Keep,
        _ => MotionStep::Start {
            blend: MOTION_BLEND_SECONDS,
        },
    }
}

/// `frame` brought into the motion's range as the game's own seek does: wrapped for a loop,
/// clamped to the last frame otherwise.
pub fn wrapped_frame(frame: f32, frame_count: i32, looping: bool) -> f32 {
    let last = (frame_count - 1).max(0) as f32;
    if last <= 0.0 || !frame.is_finite() {
        return 0.0;
    }
    if looping {
        frame.rem_euclid(last)
    } else {
        frame.clamp(0.0, last)
    }
}

/// The frame to write into the puppet's slot, or None when it is already within half a frame of
/// the partner's. A loop is compared across its seam, so frame 0.2 and frame 59.9 of 61 are close.
pub fn frame_to_seek(on_puppet: f32, target: f32, frame_count: i32, looping: bool) -> Option<f32> {
    let target = wrapped_frame(target, frame_count, looping);
    let mut gap = (on_puppet - target).abs();
    if looping {
        let period = (frame_count - 1).max(1) as f32;
        gap = gap.min(period - gap.rem_euclid(period));
    }
    (gap > MOTION_SEEK_TOLERANCE_FRAMES).then_some(target)
}
