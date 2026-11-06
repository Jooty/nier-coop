use rc_game::fix_buttons::{
    ButtonFacts, FixButton, FixButtonState, RejoinRefusal, TakeOnce, rejoin_target, tail_lines,
};
use rc_game::stay_together::{FollowRole, PhaseTarget};

const HOST_TARGET: PhaseTarget = PhaseTarget { phase: 4, point: 2 };

fn facts(role: FollowRole, connected: bool) -> ButtonFacts {
    ButtonFacts { role, connected }
}

#[test]
fn guest_only_buttons_are_greyed_for_the_host() {
    let host = facts(FollowRole::Host, true);
    assert_eq!(
        FixButton::ResyncStory.state(host),
        FixButtonState::Disabled("guest only")
    );
    assert_eq!(
        FixButton::RejoinHostArea.state(host),
        FixButtonState::Disabled("guest only")
    );
    assert_eq!(FixButton::GoToPartner.state(host), FixButtonState::Enabled);
    assert_eq!(FixButton::RespawnPartner.state(host), FixButtonState::Enabled);
}

#[test]
fn a_connected_guest_can_press_every_button() {
    let guest = facts(FollowRole::Guest, true);
    for button in [
        FixButton::GoToPartner,
        FixButton::ResyncStory,
        FixButton::RejoinHostArea,
        FixButton::RespawnPartner,
        FixButton::CopyLogs,
    ] {
        assert_eq!(button.state(guest), FixButtonState::Enabled);
    }
}

#[test]
fn only_copy_logs_works_without_a_link() {
    let alone = facts(FollowRole::Unknown, false);
    assert_eq!(FixButton::CopyLogs.state(alone), FixButtonState::Enabled);
    assert!(matches!(
        FixButton::GoToPartner.state(alone),
        FixButtonState::Disabled(_)
    ));
    assert!(matches!(
        FixButton::RejoinHostArea.state(facts(FollowRole::Guest, false)),
        FixButtonState::Disabled(_)
    ));
}

#[test]
fn a_take_once_flag_fires_once_per_request() {
    let flag = TakeOnce::new();
    assert!(!flag.take());
    flag.request();
    flag.request();
    assert!(flag.take());
    assert!(!flag.take());
}

#[test]
fn rejoin_uses_the_hosts_announced_target_even_when_already_in_that_phase() {
    assert_eq!(rejoin_target(Some(HOST_TARGET), Some(4)), Ok(HOST_TARGET));
    assert_eq!(rejoin_target(Some(HOST_TARGET), None), Ok(HOST_TARGET));
}

#[test]
fn rejoin_waits_when_nothing_is_announced_or_the_host_moved_on() {
    assert_eq!(
        rejoin_target(None, Some(4)),
        Err(RejoinRefusal::HostAreaNotKnownYet)
    );
    assert_eq!(
        rejoin_target(Some(HOST_TARGET), Some(9)),
        Err(RejoinRefusal::HostMovedSinceLastAnnounce)
    );
}

#[test]
fn the_log_tail_keeps_the_last_lines() {
    assert_eq!(tail_lines("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail_lines("a\nb", 10), "a\nb");
}
