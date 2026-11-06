use rc_game::focus::{ActivateKind, WM_ACTIVATE, classify_activate};

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
