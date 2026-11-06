pub const NORMAL_TIME_SCALE: f32 = 1.0;

pub fn restored_time_scale(before: f32, after: f32) -> Option<f32> {
    if !after.is_finite() || (after - before).abs() <= f32::EPSILON {
        return None;
    }
    let target = if before.is_finite() && before > 0.0 {
        before
    } else {
        NORMAL_TIME_SCALE
    };
    ((after - target).abs() > f32::EPSILON).then_some(target)
}
