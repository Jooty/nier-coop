use replicant_game::dialogue::{
    HeldTalkRelease, HeldTalkStart, HeldTalkStep, PartnerTalkMemory, TalkOrigin, TalkOriginTracker,
};
use replicant_game::offsets::START_TALK_NATIVE_RVA;
use replicant_game::quiet_end::{TalkStartVerdict, talk_start_verdict};
use replicant_game::shared_sequence::{LedStepPop, is_led_by_the_host, led_step_pop};
use replicant_proto::{DialogueLine, DialogueUpdate};

const PACK: &str = "A_DESERT_TOWN_01";
const STEP_BEFORE_THE_RIDE: &str = "EID_1410_a0100g_05_01";
const FIRST_TALK: u32 = 1237;
const CHAINED_TALKS: [u32; 3] = [1238, 1239, 1240];
const NATIVE_BLOCK: u64 = 0x7FF6_1234_5000;

fn line(talk_id: u32, line: u16) -> DialogueUpdate {
    DialogueUpdate::Line(DialogueLine { talk_id, line })
}

fn poll_like_the_hook(
    held: &mut HeldTalkStart,
    partner: &PartnerTalkMemory,
    now_ms: u64,
) -> HeldTalkStep {
    let mut on = partner.is_on(FIRST_TALK);
    let mut ended = partner.ended_within_follow_wait(FIRST_TALK, now_ms);
    if held.waits_for_the_whole_talk() {
        on |= partner.any_talk_on();
        ended |= partner.any_talk_ended_within_follow_wait(now_ms);
    }
    held.poll(on, ended, now_ms)
}

#[test]
fn the_step_that_starts_the_ride_is_led_by_the_host() {
    assert!(is_led_by_the_host(PACK, STEP_BEFORE_THE_RIDE));
}

#[test]
fn the_hosts_copy_runs_once_whichever_relay_brings_it_first() {
    // Run 1113l, guest 00:56:05.638 and 05.653: the shared-function copy and the story-event copy
    // of the step both popped, and each was dropped as a duplicate of the other.
    let first = led_step_pop(false, true, false);
    assert_eq!(first, LedStepPop::GuestRunsTheHostsCopy);
    assert!(first.passes_the_duplicate_checks());
    assert!(!first.drops());
    let second = led_step_pop(false, true, true);
    assert_eq!(second, LedStepPop::GuestDropsASecondCopyOfTheHosts);
    assert!(second.drops());
}

#[test]
fn the_guests_own_copy_after_the_hosts_is_dropped_and_the_host_never_drops_its_own() {
    assert!(led_step_pop(false, false, true).drops());
    assert!(!led_step_pop(false, false, false).drops());
    assert_eq!(
        led_step_pop(true, false, false),
        LedStepPop::HostRunsAndSends
    );
    assert!(!led_step_pop(true, false, true).drops());
    assert!(!led_step_pop(true, false, false).passes_the_duplicate_checks());
}

#[test]
fn a_talk_start_in_the_hosts_step_copy_waits_for_the_hosts_talk() {
    let verdict = talk_start_verdict(START_TALK_NATIVE_RVA, false, false);
    assert_eq!(verdict, TalkStartVerdict::RunNoInjectedEnd);
    let inside = verdict.inside_the_hosts_step_copy();
    assert_eq!(inside, TalkStartVerdict::SkipInsideTheHostsStepCopy);
    assert!(inside.skips());
    assert_eq!(
        TalkStartVerdict::RunNotATalkStart.inside_the_hosts_step_copy(),
        TalkStartVerdict::RunNotATalkStart
    );
}

#[test]
fn the_copy_goes_on_only_when_the_hosts_chained_talks_are_over() {
    // Run 1113l, guest: the host's talk 1237 from 05.684, 1238 07.245, 1239 07.801, 1240 08.376,
    // the host's talk ended 08.655.
    let mut partner = PartnerTalkMemory::new();
    let mut held = HeldTalkStart::new();
    partner.receive(line(FIRST_TALK, 0), 5_684);
    held.begin_for_the_whole_talk(NATIVE_BLOCK, FIRST_TALK, 5_690);
    assert_eq!(
        poll_like_the_hook(&mut held, &partner, 5_700),
        HeldTalkStep::Wait
    );
    for (talk, at_ms) in CHAINED_TALKS.into_iter().zip([7_245, 7_801, 8_376]) {
        partner.receive(line(talk, 0), at_ms);
        assert_eq!(
            poll_like_the_hook(&mut held, &partner, at_ms + 16),
            HeldTalkStep::Wait
        );
    }
    partner.receive(DialogueUpdate::Ended, 8_655);
    assert_eq!(
        poll_like_the_hook(&mut held, &partner, 8_671),
        HeldTalkStep::Release(HeldTalkRelease::PartnerFinished)
    );
}

#[test]
fn a_plain_held_talk_still_goes_on_when_its_own_talk_is_over() {
    let mut partner = PartnerTalkMemory::new();
    let mut held = HeldTalkStart::new();
    partner.receive(line(FIRST_TALK, 0), 5_684);
    held.begin(NATIVE_BLOCK, FIRST_TALK, 5_690);
    assert!(!held.waits_for_the_whole_talk());
    assert_eq!(
        poll_like_the_hook(&mut held, &partner, 5_700),
        HeldTalkStep::Wait
    );
    partner.receive(line(CHAINED_TALKS[0], 0), 7_245);
    assert_eq!(
        poll_like_the_hook(&mut held, &partner, 7_261),
        HeldTalkStep::Release(HeldTalkRelease::PartnerFinished)
    );
}

#[test]
fn the_hosts_talk_in_a_step_of_its_sequence_is_shown_to_the_guest() {
    // Run 1113l, host: _StartTalk 1237 in the step at 05.650, its window 05.666 was "not classed",
    // and the chained talks 1238 to 1240 kept that class.
    let mut origin = TalkOriginTracker::new();
    origin.note_shared_step_talk_start(STEP_BEFORE_THE_RIDE, 5_650);
    assert_eq!(
        origin.observe(Some(FIRST_TALK), false, 5_666),
        Some((
            FIRST_TALK,
            TalkOrigin::StoryStep(STEP_BEFORE_THE_RIDE.to_owned())
        ))
    );
    assert_eq!(
        origin.observe(Some(CHAINED_TALKS[0]), false, 7_234),
        Some((
            CHAINED_TALKS[0],
            TalkOrigin::StoryStep(STEP_BEFORE_THE_RIDE.to_owned())
        ))
    );
    assert!(matches!(
        origin.class_update(line(CHAINED_TALKS[0], 0)),
        DialogueUpdate::StoryLine(_)
    ));
}

#[test]
fn the_same_talk_start_outside_a_shared_step_stays_the_players_own() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(STEP_BEFORE_THE_RIDE, 5_650);
    assert_eq!(
        origin.observe(Some(FIRST_TALK), false, 5_666),
        Some((
            FIRST_TALK,
            TalkOrigin::NotClassed(Some(STEP_BEFORE_THE_RIDE.to_owned()))
        ))
    );
}
