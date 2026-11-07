use rc_game::enemies::{NORMAL_TIME_SCALE, restored_time_scale, stalled_copy_time_scale_fix};

#[test]
fn an_unchanged_scale_needs_no_restore() {
    assert_eq!(restored_time_scale(1.0, 1.0), None);
}

#[test]
fn a_slowed_copy_gets_its_old_scale_back() {
    assert_eq!(restored_time_scale(1.0, 0.0), Some(1.0));
}

#[test]
fn a_bad_old_scale_falls_back_to_normal() {
    assert_eq!(restored_time_scale(0.0, 0.2), Some(NORMAL_TIME_SCALE));
    assert_eq!(restored_time_scale(f32::NAN, 0.2), Some(NORMAL_TIME_SCALE));
}

#[test]
fn an_unreadable_new_scale_is_left_alone() {
    assert_eq!(restored_time_scale(1.0, f32::NAN), None);
}

#[test]
fn a_stalled_copy_at_normal_speed_needs_no_fix() {
    assert_eq!(stalled_copy_time_scale_fix(1.0), None);
}

#[test]
fn a_stalled_copy_with_a_slowed_or_broken_time_scale_goes_back_to_normal() {
    assert_eq!(stalled_copy_time_scale_fix(0.0), Some(NORMAL_TIME_SCALE));
    assert_eq!(stalled_copy_time_scale_fix(0.05), Some(NORMAL_TIME_SCALE));
    assert_eq!(
        stalled_copy_time_scale_fix(f32::NAN),
        Some(NORMAL_TIME_SCALE)
    );
}
