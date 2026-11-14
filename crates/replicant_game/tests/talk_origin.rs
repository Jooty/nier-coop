use replicant_game::dialogue::{
    CHAINED_TALK_KEEPS_THE_STORY_STEP_WITHIN_MS, MirrorAction, MirrorInput, OwnTalkWindow,
    PartnerTalkMemory, PartnerTalkMirror, SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS, TalkLeader,
    TalkOrigin, TalkOriginTracker,
};
use replicant_proto::{DialogueLine, DialogueUpdate};

const STORY_TALK: u32 = 1226;
const CHAINED_TALK: u32 = 1224;
const OWN_TALK: u32 = 1230;
const STORY_START_CALL: &str = "EID_1410_a0065g_020_Start";
const STORY_END_CALL: &str = "EID_1410_a0066g_End";
const NOT_A_STORY_EVENT_CALL: &str = "Talk_Text_09";
const FORCED_TALK: u32 = 1230;
const FORCED_TALK_CALL: &str = "MiniGame_End";
const LOCAL_CALL: &str = "Play_ENV_17";
const MOTION_STEP: &str = "EID_1450_a0010g_010_Motion01";
const MOTION_STEP_TALK: u32 = 1265;
const TALK_CHAINED_AFTER_AN_END: u32 = 1266;
const MOTION_STEP_TALK_START_MS: u64 = 2_318_483;
const MOTION_STEP_TALK_OPEN_MS: u64 = 2_318_498;
const MOTION_STEP_TALK_ENDED_MS: u64 = 2_319_821;
const CHAINED_TALK_OPEN_MS: u64 = 2_319_837;
const SET_TALK_STEP: &str = "EID_1410_a0100g_21_03";
const SET_TALK_STEP_TALK: u32 = 1258;
const SET_TALK_STEP_CHAINED_TALK: u32 = 1259;
const LOOK_STEP: &str = "EID_1450_CharLookChar01";
const LATER_STEP: &str = "EID_1450_a0010g_020_01";
const LATER_STEP_TALK: u32 = 1273;
const TALK_COMMAND_TALK: u32 = 1260;
const RIDERS_OWN_STEP: &str = "EID_1410_a0100g_21_04";

fn line(talk_id: u32, line: u16) -> DialogueLine {
    DialogueLine { talk_id, line }
}

fn guest_tick(
    mirror: &mut PartnerTalkMirror,
    host: &PartnerTalkMemory,
    own: Option<DialogueLine>,
    window_open: bool,
    now_ms: u64,
) -> Option<MirrorAction> {
    mirror.step(MirrorInput {
        partner_line: host.for_mirror(own, TalkLeader::Either, true),
        partner_ended_talk: None,
        can_show: true,
        window_open,
        own_talk: own,
        leader: TalkLeader::Either,
        answer_open: false,
        now_ms,
    })
}

#[test]
fn a_talk_a_story_event_callback_opened_is_a_story_step() {
    for call in [STORY_START_CALL, STORY_END_CALL] {
        let mut origin = TalkOriginTracker::new();
        origin.note_script_talk_start(call, 1000);
        assert_eq!(
            origin.observe(Some(STORY_TALK), false, 1017),
            Some((STORY_TALK, TalkOrigin::StoryStep(call.to_owned())))
        );
        assert!(origin.current_is_story_step());
        assert_eq!(origin.observe(Some(STORY_TALK), false, 1500), None);
    }
}

#[test]
fn a_talk_started_with_the_talk_command_stays_the_players_own() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(STORY_START_CALL, 1000);
    assert_eq!(
        origin.observe(Some(STORY_TALK), true, 1017),
        Some((STORY_TALK, TalkOrigin::TalkCommand))
    );
    assert_eq!(
        origin.class_update(DialogueUpdate::Line(line(STORY_TALK, 0))),
        DialogueUpdate::Line(line(STORY_TALK, 0))
    );
}

#[test]
fn a_talk_that_cannot_be_classed_stays_the_players_own() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(NOT_A_STORY_EVENT_CALL, 1000);
    assert_eq!(
        origin.observe(Some(STORY_TALK), false, 1017),
        Some((
            STORY_TALK,
            TalkOrigin::NotClassed(Some(NOT_A_STORY_EVENT_CALL.to_owned()))
        ))
    );
    assert!(!origin.current_is_story_step());

    let mut stale = TalkOriginTracker::new();
    stale.note_script_talk_start(STORY_START_CALL, 1000);
    let late = 1000 + SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS + 1;
    assert_eq!(
        stale.observe(Some(OWN_TALK), false, late),
        Some((OWN_TALK, TalkOrigin::NotClassed(None)))
    );
}

#[test]
fn a_talk_chained_in_a_story_window_stays_a_story_step_until_the_window_closes() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(STORY_END_CALL, 1000);
    origin.observe(Some(STORY_TALK), false, 1017);
    assert_eq!(
        origin.observe(Some(CHAINED_TALK), false, 9000),
        Some((
            CHAINED_TALK,
            TalkOrigin::StoryStep(STORY_END_CALL.to_owned())
        ))
    );
    assert_eq!(origin.observe(None, false, 9100), None);
    assert_eq!(
        origin.observe(Some(OWN_TALK), false, 20_000),
        Some((OWN_TALK, TalkOrigin::NotClassed(None)))
    );
}

#[test]
fn only_a_story_steps_lines_go_out_as_story_lines() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(STORY_START_CALL, 1000);
    origin.observe(Some(STORY_TALK), false, 1017);
    assert_eq!(
        origin.class_update(DialogueUpdate::Line(line(STORY_TALK, 2))),
        DialogueUpdate::StoryLine(line(STORY_TALK, 2))
    );
    assert_eq!(
        origin.class_update(DialogueUpdate::Ended),
        DialogueUpdate::Ended
    );
}

#[test]
fn the_guest_is_shown_the_hosts_story_talk_line_by_line_in_its_own_window() {
    let mut host = PartnerTalkMemory::new();
    let mut mirror = PartnerTalkMirror::new();
    host.receive(DialogueUpdate::StoryLine(line(STORY_TALK, 0)), 0);
    assert_eq!(
        guest_tick(&mut mirror, &host, None, false, 0),
        Some(MirrorAction::Open(line(STORY_TALK, 0)))
    );
    host.receive(DialogueUpdate::StoryLine(line(STORY_TALK, 1)), 800);
    assert_eq!(
        guest_tick(&mut mirror, &host, None, true, 800),
        Some(MirrorAction::ShowLine(1))
    );
    host.receive(DialogueUpdate::Ended, 2500);
    assert_eq!(
        guest_tick(&mut mirror, &host, None, true, 2500),
        Some(MirrorAction::Close)
    );
}

#[test]
fn the_hosts_own_talk_is_not_shown_to_the_guest() {
    let mut host = PartnerTalkMemory::new();
    let mut mirror = PartnerTalkMirror::new();
    host.receive(DialogueUpdate::Line(line(OWN_TALK, 0)), 0);
    assert_eq!(host.story_line(), None);
    assert_eq!(guest_tick(&mut mirror, &host, None, false, 0), None);
    assert!(!mirror.is_showing());
}

#[test]
fn a_story_talk_never_takes_a_window_the_guest_opened_itself() {
    let mut host = PartnerTalkMemory::new();
    let mut mirror = PartnerTalkMirror::new();
    host.receive(DialogueUpdate::StoryLine(line(STORY_TALK, 0)), 0);
    let own = Some(line(OWN_TALK, 0));
    assert_eq!(host.for_mirror(own, TalkLeader::Either, true), None);
    assert_eq!(guest_tick(&mut mirror, &host, own, true, 0), None);
    assert!(!mirror.is_showing());
}

#[test]
fn the_host_does_not_show_the_guests_story_talk() {
    let mut guest = PartnerTalkMemory::new();
    guest.receive(DialogueUpdate::StoryLine(line(STORY_TALK, 0)), 0);
    assert_eq!(guest.for_mirror(None, TalkLeader::Local, false), None);
}

#[test]
fn a_talk_a_story_callback_forced_without_a_talk_start_is_a_story_step() {
    // Run 1113k: MiniGame_End popped at 17:19:13.579 and talk 1230 opened 16 ms later.
    let mut origin = TalkOriginTracker::new();
    origin.note_popped_call(FORCED_TALK_CALL, 79_153_579);
    assert_eq!(
        origin.observe(Some(FORCED_TALK), false, 79_153_595),
        Some((
            FORCED_TALK,
            TalkOrigin::StoryStep(FORCED_TALK_CALL.to_owned())
        ))
    );
    assert_eq!(
        origin.class_update(DialogueUpdate::Line(line(FORCED_TALK, 0))),
        DialogueUpdate::StoryLine(line(FORCED_TALK, 0))
    );
}

#[test]
fn a_talk_after_a_local_call_or_a_stale_pop_stays_not_classed() {
    let opened_ms = 5_000;
    for (call, popped_ms) in [
        (LOCAL_CALL, opened_ms),
        (
            FORCED_TALK_CALL,
            opened_ms - SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS - 1,
        ),
    ] {
        let mut origin = TalkOriginTracker::new();
        origin.note_popped_call(call, popped_ms);
        assert_eq!(
            origin.observe(Some(FORCED_TALK), false, opened_ms),
            Some((FORCED_TALK, TalkOrigin::NotClassed(None)))
        );
    }
}

#[test]
fn the_talk_command_wins_over_a_forcing_story_callback() {
    let mut origin = TalkOriginTracker::new();
    origin.note_popped_call(FORCED_TALK_CALL, 1_000);
    assert_eq!(
        origin.observe(Some(FORCED_TALK), true, 1_010),
        Some((FORCED_TALK, TalkOrigin::TalkCommand))
    );
}

#[test]
fn a_talk_a_step_of_a_story_event_starts_is_a_story_step() {
    // Run 1114a, host: talk 1265 from its motion step and 1273 from a later step were "not
    // classed ... kept as your own", so the guest was never shown them.
    for (call, talk_id) in [
        (MOTION_STEP, MOTION_STEP_TALK),
        (LATER_STEP, LATER_STEP_TALK),
    ] {
        let mut origin = TalkOriginTracker::new();
        origin.note_script_talk_start(call, MOTION_STEP_TALK_START_MS);
        assert_eq!(
            origin.observe(Some(talk_id), false, MOTION_STEP_TALK_OPEN_MS),
            Some((talk_id, TalkOrigin::StoryStep(call.to_owned())))
        );
    }
}

#[test]
fn a_talk_chained_right_after_a_story_talk_ended_stays_a_story_step() {
    // Run 1114a, host: 1265 sent its end at 39.821 and 1266 opened in the window 16 ms later with
    // no talk start native.
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(MOTION_STEP, MOTION_STEP_TALK_START_MS);
    origin.observe(Some(MOTION_STEP_TALK), false, MOTION_STEP_TALK_OPEN_MS);
    assert_eq!(origin.observe(None, false, MOTION_STEP_TALK_ENDED_MS), None);
    assert_eq!(
        origin.observe(Some(TALK_CHAINED_AFTER_AN_END), false, CHAINED_TALK_OPEN_MS),
        Some((
            TALK_CHAINED_AFTER_AN_END,
            TalkOrigin::StoryStep(MOTION_STEP.to_owned())
        ))
    );
    assert!(matches!(
        origin.class_update(DialogueUpdate::Line(line(TALK_CHAINED_AFTER_AN_END, 0))),
        DialogueUpdate::StoryLine(_)
    ));
}

#[test]
fn a_talk_long_after_a_story_talk_ended_is_not_chained_to_it() {
    let mut origin = TalkOriginTracker::new();
    origin.note_script_talk_start(MOTION_STEP, MOTION_STEP_TALK_START_MS);
    origin.observe(Some(MOTION_STEP_TALK), false, MOTION_STEP_TALK_OPEN_MS);
    origin.observe(None, false, MOTION_STEP_TALK_ENDED_MS);
    let late_ms = MOTION_STEP_TALK_ENDED_MS + CHAINED_TALK_KEEPS_THE_STORY_STEP_WITHIN_MS + 1;
    assert_eq!(
        origin.observe(Some(OWN_TALK), false, late_ms),
        Some((OWN_TALK, TalkOrigin::NotClassed(None)))
    );
}

#[test]
fn a_talk_a_step_forced_with_no_talk_start_is_a_story_step_and_so_is_its_chain() {
    // Run 1114a, host: the step popped at 30.204 sets talk 1258 (no talk start native), its window
    // opened at 30.220, and 1259 followed in the same window.
    let mut origin = TalkOriginTracker::new();
    origin.note_popped_call(SET_TALK_STEP, 2_230_204);
    assert_eq!(
        origin.observe(Some(SET_TALK_STEP_TALK), false, 2_230_220),
        Some((
            SET_TALK_STEP_TALK,
            TalkOrigin::StoryStep(SET_TALK_STEP.to_owned())
        ))
    );
    assert_eq!(
        origin.observe(Some(SET_TALK_STEP_CHAINED_TALK), false, 2_232_534),
        Some((
            SET_TALK_STEP_CHAINED_TALK,
            TalkOrigin::StoryStep(SET_TALK_STEP.to_owned())
        ))
    );
    let mut look = TalkOriginTracker::new();
    look.note_popped_call(LOOK_STEP, 1_000);
    assert!(matches!(
        look.observe(Some(OWN_TALK), false, 1_016),
        Some((_, TalkOrigin::StoryStep(_)))
    ));
}

#[test]
fn a_talk_command_talk_and_a_rides_own_step_stay_the_players_own() {
    let mut command = TalkOriginTracker::new();
    command.note_popped_call(LATER_STEP, 1_000);
    assert_eq!(
        command.observe(Some(TALK_COMMAND_TALK), true, 1_016),
        Some((TALK_COMMAND_TALK, TalkOrigin::TalkCommand))
    );
    let mut ride = TalkOriginTracker::new();
    ride.note_script_talk_start(RIDERS_OWN_STEP, 1_000);
    assert_eq!(
        ride.observe(Some(OWN_TALK), false, 1_016),
        Some((
            OWN_TALK,
            TalkOrigin::NotClassed(Some(RIDERS_OWN_STEP.to_owned()))
        ))
    );
}

#[test]
fn a_talk_the_game_chains_in_its_own_window_stays_its_own_copy() {
    // Run 1114a, guest: its own 1265 had the talk mark in slot one, the chained 1266 did not
    // (mask 0x20/0x0), so its copy was neither followed nor mirrored and sat on line 0.
    let mut window = OwnTalkWindow::new();
    assert!(window.observe(true, true));
    assert!(window.observe(true, false));
    assert!(!window.observe(false, false));
    assert!(!window.observe(true, false));
}

#[test]
fn the_guests_chained_copy_follows_the_hosts_lines() {
    let mut host = PartnerTalkMemory::new();
    let mut mirror = PartnerTalkMirror::new();
    let own = Some(line(TALK_CHAINED_AFTER_AN_END, 0));
    host.receive(
        DialogueUpdate::StoryLine(line(TALK_CHAINED_AFTER_AN_END, 0)),
        0,
    );
    let follow = |mirror: &mut PartnerTalkMirror, host: &PartnerTalkMemory, now_ms| {
        mirror.step(MirrorInput {
            partner_line: host.for_mirror(own, TalkLeader::Partner, true),
            partner_ended_talk: None,
            can_show: true,
            window_open: true,
            own_talk: own,
            leader: TalkLeader::Partner,
            answer_open: false,
            now_ms,
        })
    };
    assert_eq!(
        follow(&mut mirror, &host, 0),
        Some(MirrorAction::FollowStarted(TALK_CHAINED_AFTER_AN_END))
    );
    host.receive(
        DialogueUpdate::StoryLine(line(TALK_CHAINED_AFTER_AN_END, 3)),
        3_500,
    );
    assert_eq!(
        follow(&mut mirror, &host, 3_500),
        Some(MirrorAction::FollowLine(3))
    );
}
