use replicant_game::dialogue::{
    FOLLOW_HOST_TALK_WAIT_MS, HostLedScriptTalk, MirrorAction, MirrorInput, PartnerTalkMemory,
    PartnerTalkMirror, TalkLeader,
};
use replicant_proto::{DialogueLine, DialogueUpdate};

const SHARED_TALK: u32 = 1082;
const NEXT_TALK: u32 = 2608;
const OTHER_TALK: u32 = 1083;

fn line(talk_id: u32, line: u16) -> DialogueLine {
    DialogueLine { talk_id, line }
}

fn far_guest_tick(
    led: &mut HostLedScriptTalk,
    mirror: &mut PartnerTalkMirror,
    host: &PartnerTalkMemory,
    own: Option<DialogueLine>,
    now_ms: u64,
) -> Option<MirrorAction> {
    let host_on_same_talk = own.is_some_and(|own| host.is_or_just_was_on(own.talk_id, now_ms));
    let leader = if led.observe(own, false, host_on_same_talk) {
        TalkLeader::Partner
    } else {
        TalkLeader::Either
    };
    mirror.step(MirrorInput {
        partner_line: host.for_follow(own, leader),
        partner_ended_talk: host.ended_for_follow(own, leader, now_ms),
        can_show: false,
        window_open: own.is_some(),
        own_talk: own,
        leader,
        answer_open: false,
        now_ms,
    })
}

#[test]
fn a_script_talk_the_host_is_on_too_is_led_until_its_window_closes() {
    let mut led = HostLedScriptTalk::new();
    assert!(!led.is_led());
    assert!(led.observe(Some(line(SHARED_TALK, 0)), false, true));
    // The host moved on to the next talk; this copy has not stepped yet.
    assert!(led.observe(Some(line(SHARED_TALK, 0)), false, false));
    assert!(led.observe(Some(line(NEXT_TALK, 0)), false, false));
    assert!(led.is_led());
    assert!(!led.observe(None, false, false));
    assert!(!led.observe(Some(line(OTHER_TALK, 0)), false, false));
}

#[test]
fn a_talk_the_player_started_is_never_led_from_afar() {
    let mut led = HostLedScriptTalk::new();
    assert!(!led.observe(Some(line(SHARED_TALK, 0)), true, true));
    assert!(led.observe(Some(line(SHARED_TALK, 0)), false, true));
    assert!(!led.observe(Some(line(SHARED_TALK, 0)), true, false));
}

#[test]
fn a_script_talk_the_host_is_not_on_is_not_led() {
    let mut led = HostLedScriptTalk::new();
    assert!(!led.observe(Some(line(SHARED_TALK, 0)), false, false));
    assert!(!led.observe(None, false, true));
}

#[test]
fn the_host_counts_as_on_a_talk_it_left_within_the_follow_wait() {
    let mut host = PartnerTalkMemory::new();
    assert!(!host.is_or_just_was_on(SHARED_TALK, 0));
    host.receive(DialogueUpdate::Line(line(SHARED_TALK, 0)), 0);
    assert!(host.is_or_just_was_on(SHARED_TALK, 10));
    assert!(!host.is_or_just_was_on(OTHER_TALK, 10));
    host.receive(DialogueUpdate::Ended, 100);
    assert!(host.is_or_just_was_on(SHARED_TALK, 100 + FOLLOW_HOST_TALK_WAIT_MS));
    assert!(!host.is_or_just_was_on(SHARED_TALK, 101 + FOLLOW_HOST_TALK_WAIT_MS));
}

#[test]
fn a_far_guest_with_nobody_at_it_follows_the_host_through_a_shared_script_talk() {
    let mut led = HostLedScriptTalk::new();
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(SHARED_TALK, 0)), 0);
    let own = Some(line(SHARED_TALK, 0));
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, own, 0),
        Some(MirrorAction::FollowStarted(SHARED_TALK))
    );
    assert!(mirror.holds_own_talk());
    host.receive(DialogueUpdate::Line(line(SHARED_TALK, 1)), 900);
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, own, 900),
        Some(MirrorAction::FollowLine(1))
    );
    let own = Some(line(SHARED_TALK, 1));
    host.receive(DialogueUpdate::Line(line(NEXT_TALK, 0)), 1_300);
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, own, 1_300),
        Some(MirrorAction::ChainOwnTalk(line(NEXT_TALK, 0)))
    );
    let own = Some(line(NEXT_TALK, 0));
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, own, 1_316),
        Some(MirrorAction::FollowStarted(NEXT_TALK))
    );
    host.receive(DialogueUpdate::Ended, 1_600);
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, own, 1_600),
        Some(MirrorAction::FinishOwnTalk(NEXT_TALK))
    );
    assert_eq!(
        far_guest_tick(&mut led, &mut mirror, &host, None, 1_700),
        None
    );
    assert!(!led.is_led());
}

#[test]
fn a_far_guests_script_talk_the_host_never_had_stays_with_its_own_buttons() {
    let mut led = HostLedScriptTalk::new();
    let mut mirror = PartnerTalkMirror::new();
    let host = PartnerTalkMemory::new();
    let own = Some(line(SHARED_TALK, 0));
    for tick in 0..1_000 {
        assert_eq!(
            far_guest_tick(&mut led, &mut mirror, &host, own, tick * 16),
            None
        );
        assert!(!mirror.holds_own_talk());
    }
}
