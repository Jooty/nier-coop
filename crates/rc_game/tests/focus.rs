use rc_game::focus::{
    ActivateKind, FOCUS_LOSS_CLEAR_DELAY_MS, FocusLossTracker, WM_ACTIVATE, classify_activate,
};

const WM_PAINT: u32 = 0x000F;
const WA_ACTIVE: usize = 1;
const WA_CLICKACTIVE: usize = 2;
const MINIMISED_FLAG: usize = 0x1_0000;

#[test]
fn inactive_activation_is_a_focus_loss() {
    assert_eq!(classify_activate(WM_ACTIVATE, 0), ActivateKind::Lost);
}

#[test]
fn a_minimised_deactivation_is_still_a_focus_loss() {
    assert_eq!(
        classify_activate(WM_ACTIVATE, MINIMISED_FLAG),
        ActivateKind::Lost
    );
}

#[test]
fn active_and_click_active_are_a_focus_gain() {
    assert_eq!(
        classify_activate(WM_ACTIVATE, WA_ACTIVE),
        ActivateKind::Gained
    );
    assert_eq!(
        classify_activate(WM_ACTIVATE, WA_CLICKACTIVE),
        ActivateKind::Gained
    );
}

#[test]
fn other_messages_are_not_activation() {
    assert_eq!(classify_activate(WM_PAINT, 0), ActivateKind::NotActivation);
}

#[test]
fn a_focus_loss_is_not_due_until_over_a_second() {
    let mut tracker = FocusLossTracker::default();
    tracker.lose(5000);
    assert!(tracker.is_pending());
    assert!(!tracker.take_due(5000 + FOCUS_LOSS_CLEAR_DELAY_MS));
    assert!(tracker.take_due(5000 + FOCUS_LOSS_CLEAR_DELAY_MS + 1));
    assert!(!tracker.is_pending());
    assert!(!tracker.take_due(9000));
}

#[test]
fn flapping_focus_never_becomes_due() {
    let mut tracker = FocusLossTracker::default();
    for second in 0..10_u64 {
        tracker.lose(second * 400);
        assert!(!tracker.take_due(second * 400 + 200));
        tracker.regain();
    }
    assert!(!tracker.is_pending());
}

#[test]
fn a_repeated_loss_keeps_the_first_loss_time() {
    let mut tracker = FocusLossTracker::default();
    tracker.lose(100);
    tracker.lose(900);
    assert!(tracker.take_due(1101));
}
