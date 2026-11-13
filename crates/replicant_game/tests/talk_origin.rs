use replicant_game::dialogue::{
    MirrorAction, MirrorInput, PartnerTalkMemory, PartnerTalkMirror,
    SCRIPT_TALK_START_OPENS_THE_WINDOW_WITHIN_MS, TalkLeader, TalkOrigin, TalkOriginTracker,
};
use replicant_proto::{DialogueLine, DialogueUpdate};

const STORY_TALK: u32 = 1226;
const CHAINED_TALK: u32 = 1224;
const OWN_TALK: u32 = 1230;
const STORY_START_CALL: &str = "EID_1410_a0065g_020_Start";
const STORY_END_CALL: &str = "EID_1410_a0066g_End";
const NOT_A_STORY_EVENT_CALL: &str = "EID_1410_a0100g_03_01";

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
