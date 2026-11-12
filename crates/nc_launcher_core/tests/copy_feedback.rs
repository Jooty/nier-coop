use nc_launcher_core::{COPIED_EASE_SECONDS, COPIED_HOLD_SECONDS, copied_amount};

#[test]
fn the_copied_state_holds_then_eases_back_to_rest() {
    assert_eq!(copied_amount(0.0), 1.0);
    assert_eq!(copied_amount(COPIED_HOLD_SECONDS - 0.01), 1.0);
    let midway = copied_amount(COPIED_HOLD_SECONDS + COPIED_EASE_SECONDS / 2.0);
    assert!(midway > 0.0 && midway < 1.0);
    assert_eq!(
        copied_amount(COPIED_HOLD_SECONDS + COPIED_EASE_SECONDS),
        0.0
    );
}

#[test]
fn nothing_shows_before_a_click() {
    assert_eq!(copied_amount(-1.0), 0.0);
}
