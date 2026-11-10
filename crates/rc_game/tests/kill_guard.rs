use rc_game::downed::{
    BlockedKillResponse, KillGuardInputs, blocked_kill_response, kill_is_blocked,
};

const PARTNER_UP: KillGuardInputs = KillGuardInputs {
    session_active: true,
    partner_downed: false,
    game_over: false,
    rewinding: false,
};

#[test]
fn a_kill_is_blocked_while_the_partner_is_up_and_in_play() {
    assert!(kill_is_blocked(PARTNER_UP));
}

#[test]
fn a_kill_passes_without_a_session() {
    assert!(!kill_is_blocked(KillGuardInputs {
        session_active: false,
        ..PARTNER_UP
    }));
}

#[test]
fn a_kill_passes_when_the_partner_is_downed() {
    assert!(!kill_is_blocked(KillGuardInputs {
        partner_downed: true,
        ..PARTNER_UP
    }));
}

#[test]
fn a_kill_passes_during_game_over_and_rewind() {
    assert!(!kill_is_blocked(KillGuardInputs {
        game_over: true,
        ..PARTNER_UP
    }));
    assert!(!kill_is_blocked(KillGuardInputs {
        rewinding: true,
        ..PARTNER_UP
    }));
}

#[test]
fn a_blocked_kill_without_ground_recovers_from_the_fall() {
    assert_eq!(
        blocked_kill_response(false),
        BlockedKillResponse::RecoverFromFall
    );
}

#[test]
fn a_blocked_kill_on_the_ground_sends_the_player_down() {
    assert_eq!(blocked_kill_response(true), BlockedKillResponse::GoDown);
}
