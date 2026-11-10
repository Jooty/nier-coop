use std::cell::Cell;

use rc_game::dialogue::{
    CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS, ChoiceFollowInput, ChoiceFollower, ChoiceStep, DialogueTracker, open_prompt_kind, FOLLOW_HOST_TALK_WAIT_MS, MirrorAction, MirrorInput,
    OWN_ENDED_TALK_NOT_MIRRORED_MS, PartnerTalkMemory, PartnerTalkMirror, TALK_JOIN_RANGE_UNITS,
    TALK_TABLE_SCAN_CHUNK_SLOTS, TALK_WINDOW_CURRENT_LINE_OFFSET, TALK_WINDOW_CURRENT_TALK_OFFSET,
    TALK_WINDOW_RVA, TALK_WINDOW_TALK_TABLE_OFFSET, TalkCursor, TalkLeader, WATCHER_HOLD_MASK,
    WatcherHold, WatcherHoldAction, WatcherHoldInput, find_talk_id, leader_for_own_talk,
    own_talk_started, read_talk_cursor, replace_player_name_tag, speaker_name_to_show,
    talk_tags_run_here, within_talk_range,
};
use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA, read_world_flags};
use rc_game::ui_scenes::PAUSE_MENU_SCENE_ID;
use rc_proto::{DialogueLine, DialogueUpdate, PartnerActivity, TalkChoice, TalkChoiceKind};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const TALK_ADDRESS: u64 = 0x2_0000_0000;

fn window() -> u64 {
    FAKE_MODULE_BASE + u64::from(TALK_WINDOW_RVA)
}

fn put_window(memory: &mut FakeMemory, state: i32, talk_address: u64, line: i32) {
    memory.put(window(), &state.to_le_bytes());
    memory.put(
        window() + TALK_WINDOW_CURRENT_TALK_OFFSET,
        &talk_address.to_le_bytes(),
    );
    memory.put(
        window() + TALK_WINDOW_CURRENT_LINE_OFFSET,
        &line.to_le_bytes(),
    );
}

fn line(talk_id: u32, line: u16) -> DialogueLine {
    DialogueLine { talk_id, line }
}

#[test]
fn a_closed_window_has_no_cursor_and_an_open_one_names_its_talk_and_line() {
    let mut memory = FakeMemory::new();
    put_window(&mut memory, 0, TALK_ADDRESS, 1);
    assert_eq!(read_talk_cursor(&memory).unwrap(), None);
    put_window(&mut memory, 1, TALK_ADDRESS, 1);
    assert_eq!(
        read_talk_cursor(&memory).unwrap(),
        Some(TalkCursor {
            talk_address: TALK_ADDRESS,
            line: 1
        })
    );
    put_window(&mut memory, 1, TALK_ADDRESS, -1);
    assert_eq!(read_talk_cursor(&memory).unwrap(), None);
}

#[test]
fn the_talk_id_is_the_table_slot_that_holds_the_talk() {
    let talk_id = TALK_TABLE_SCAN_CHUNK_SLOTS + 5;
    let mut memory = FakeMemory::new();
    let table = window() + TALK_WINDOW_TALK_TABLE_OFFSET;
    memory.put(table, &vec![0; TALK_TABLE_SCAN_CHUNK_SLOTS as usize * 16]);
    memory.put(table + u64::from(talk_id) * 8, &TALK_ADDRESS.to_le_bytes());
    assert_eq!(find_talk_id(&memory, TALK_ADDRESS).unwrap(), Some(talk_id));
}

#[test]
fn the_tracker_sends_each_new_line_once_and_ended_when_the_window_closes() {
    let mut tracker = DialogueTracker::new();
    let cursor = |line| {
        Some(TalkCursor {
            talk_address: TALK_ADDRESS,
            line,
        })
    };
    let scans = Cell::new(0);
    let find = |_| {
        scans.set(scans.get() + 1);
        Some(42)
    };
    assert_eq!(
        tracker.observe(cursor(0), true, find),
        Some(DialogueUpdate::Line(line(42, 0)))
    );
    assert_eq!(tracker.observe(cursor(0), true, find), None);
    assert_eq!(
        tracker.observe(cursor(1), true, find),
        Some(DialogueUpdate::Line(line(42, 1)))
    );
    assert_eq!(scans.get(), 1);
    assert_eq!(
        tracker.observe(None, true, find),
        Some(DialogueUpdate::Ended)
    );
    assert_eq!(tracker.observe(None, true, find), None);
}

#[test]
fn a_partner_who_joins_mid_talk_gets_the_current_line() {
    let mut tracker = DialogueTracker::new();
    let cursor = Some(TalkCursor {
        talk_address: TALK_ADDRESS,
        line: 3,
    });
    assert_eq!(tracker.observe(cursor, false, |_| Some(42)), None);
    assert_eq!(
        tracker.observe(cursor, true, |_| Some(42)),
        Some(DialogueUpdate::Line(line(42, 3)))
    );
}

#[test]
fn an_open_talk_window_means_in_dialogue_unless_a_cutscene_or_menu_wins() {
    let activity = |event: u8, mask: u32, window_state: i32, menu: bool| {
        let mut memory = FakeMemory::new();
        memory.put_at_rva(EVENT_SCENE_BYTE_RVA, &[event]);
        let slots: Vec<u8> = [mask, 0, 0, 0]
            .iter()
            .flat_map(|slot| slot.to_le_bytes())
            .collect();
        memory.put_at_rva(CONTROL_MASK_FIRST_RVA, &slots);
        memory.put(window(), &window_state.to_le_bytes());
        let scene_ids: &[u8] = if menu { &[PAUSE_MENU_SCENE_ID] } else { &[] };
        memory.put_ui_scenes(scene_ids, false);
        read_world_flags(&memory).unwrap().activity()
    };
    assert_eq!(activity(0, 0xA0, 1, false), PartnerActivity::InDialogue);
    assert_eq!(activity(0, 0xA0, 0, false), PartnerActivity::Playing);
    assert_eq!(activity(1, 0xA0, 1, false), PartnerActivity::InCutscene);
    assert_eq!(activity(0, 0x19F, 1, true), PartnerActivity::InMenu);
    assert!(!PartnerActivity::InDialogue.world_paused());
    assert_eq!(PartnerActivity::InDialogue.label(), Some("in dialogue"));
}

fn mirror_input(partner_line: Option<DialogueLine>, now_ms: u64) -> MirrorInput {
    MirrorInput {
        partner_line,
        partner_ended_talk: None,
        can_show: true,
        window_open: false,
        own_talk: None,
        leader: TalkLeader::Either,
        answer_open: false,
        now_ms,
    }
}

#[test]
fn the_mirror_opens_follows_each_line_and_closes_with_the_talk() {
    let mut mirror = PartnerTalkMirror::new();
    assert_eq!(
        mirror.step(mirror_input(Some(line(42, 0)), 0)),
        Some(MirrorAction::Open(line(42, 0)))
    );
    let open = |partner_line| MirrorInput {
        window_open: true,
        ..mirror_input(partner_line, 10)
    };
    assert_eq!(mirror.step(open(Some(line(42, 0)))), None);
    assert_eq!(
        mirror.step(open(Some(line(42, 1)))),
        Some(MirrorAction::ShowLine(1))
    );
    assert_eq!(mirror.step(open(None)), Some(MirrorAction::Close));
    assert!(!mirror.is_showing());
}

#[test]
fn a_partner_who_comes_in_on_line_three_opens_on_line_three() {
    let mut mirror = PartnerTalkMirror::new();
    let busy = MirrorInput {
        can_show: false,
        ..mirror_input(Some(line(42, 3)), 0)
    };
    assert_eq!(mirror.step(busy), None);
    assert_eq!(
        mirror.step(mirror_input(Some(line(42, 3)), 10)),
        Some(MirrorAction::Open(line(42, 3)))
    );
}

#[test]
fn a_menu_or_zone_load_closes_the_mirror_and_it_reopens_at_the_current_line() {
    let mut mirror = PartnerTalkMirror::new();
    mirror.step(mirror_input(Some(line(42, 1)), 0));
    let in_menu = MirrorInput {
        can_show: false,
        window_open: true,
        ..mirror_input(Some(line(42, 2)), 100)
    };
    assert_eq!(mirror.step(in_menu), Some(MirrorAction::Close));
    assert_eq!(
        mirror.step(mirror_input(Some(line(42, 4)), 5000)),
        Some(MirrorAction::Open(line(42, 4)))
    );
}

#[test]
fn a_window_the_game_closed_is_reopened_but_not_every_frame() {
    let mut mirror = PartnerTalkMirror::new();
    mirror.step(mirror_input(Some(line(42, 1)), 0));
    assert_eq!(mirror.step(mirror_input(Some(line(42, 1)), 16)), None);
    assert_eq!(
        mirror.step(mirror_input(Some(line(42, 1)), 2000)),
        Some(MirrorAction::Open(line(42, 1)))
    );
}

#[test]
fn a_different_talk_closes_first_then_opens_the_new_one() {
    let mut mirror = PartnerTalkMirror::new();
    mirror.step(mirror_input(Some(line(42, 1)), 0));
    let next = MirrorInput {
        window_open: true,
        ..mirror_input(Some(line(7, 0)), 10)
    };
    assert_eq!(mirror.step(next), Some(MirrorAction::Close));
    assert_eq!(
        mirror.step(mirror_input(Some(line(7, 0)), 2000)),
        Some(MirrorAction::Open(line(7, 0)))
    );
}

#[test]
fn a_window_this_player_opened_is_never_taken_over() {
    let mut mirror = PartnerTalkMirror::new();
    let local_window = MirrorInput {
        window_open: true,
        ..mirror_input(Some(line(42, 1)), 0)
    };
    assert_eq!(mirror.step(local_window), None);
}

#[test]
fn talking_to_the_same_npc_jumps_to_the_partners_line_once() {
    let mut mirror = PartnerTalkMirror::new();
    let own = |own_line, now_ms| MirrorInput {
        window_open: true,
        own_talk: Some(line(42, own_line)),
        ..mirror_input(Some(line(42, 3)), now_ms)
    };
    assert_eq!(mirror.step(own(0, 0)), Some(MirrorAction::JumpOwnTalk(3)));
    assert_eq!(mirror.step(own(0, 10)), None);
    let other_talk = MirrorInput {
        window_open: true,
        own_talk: Some(line(9, 0)),
        ..mirror_input(Some(line(42, 3)), 20)
    };
    assert_eq!(mirror.step(other_talk), None);
}

#[test]
fn only_a_partner_within_talk_range_joins_and_the_edge_does_not_flicker() {
    assert!(within_talk_range(TALK_JOIN_RANGE_UNITS - 0.1, false));
    assert!(!within_talk_range(TALK_JOIN_RANGE_UNITS + 0.5, false));
    assert!(within_talk_range(TALK_JOIN_RANGE_UNITS + 0.5, true));
    assert!(!within_talk_range(TALK_JOIN_RANGE_UNITS * 3.0, true));
    assert!(!within_talk_range(f32::NAN, false));
}

#[test]
fn walking_into_range_mid_talk_opens_at_the_current_line_and_leaving_closes() {
    let mut mirror = PartnerTalkMirror::new();
    let step = |mirror: &mut PartnerTalkMirror, distance: f32, partner_line, open, now_ms| {
        let can_show = within_talk_range(distance, mirror.is_showing());
        mirror.step(MirrorInput {
            can_show,
            window_open: open,
            ..mirror_input(Some(partner_line), now_ms)
        })
    };
    assert_eq!(step(&mut mirror, 40.0, line(42, 0), false, 0), None);
    assert_eq!(step(&mut mirror, 40.0, line(42, 2), false, 10), None);
    assert_eq!(
        step(&mut mirror, 10.0, line(42, 2), false, 20),
        Some(MirrorAction::Open(line(42, 2)))
    );
    assert_eq!(
        step(&mut mirror, 16.0, line(42, 3), true, 30),
        Some(MirrorAction::ShowLine(3))
    );
    assert_eq!(
        step(&mut mirror, 30.0, line(42, 3), true, 40),
        Some(MirrorAction::Close)
    );
}

#[test]
fn only_the_talker_runs_a_lines_tag_callbacks() {
    assert!(talk_tags_run_here(false, false));
    assert!(!talk_tags_run_here(true, false));
    assert!(!talk_tags_run_here(false, true));
    assert!(!talk_tags_run_here(true, true));
}

fn hold_input(mirror_showing: bool, slot_one_mask: u32, taken_over: bool) -> WatcherHoldInput {
    WatcherHoldInput {
        mirror_showing,
        slot_one_mask,
        taken_over,
    }
}

#[test]
fn the_watcher_is_held_when_the_mirror_opens_and_released_when_it_closes() {
    let mut hold = WatcherHold::new();
    assert_eq!(hold.step(hold_input(false, 0, false)), None);
    assert_eq!(
        hold.step(hold_input(true, 0, false)),
        Some(WatcherHoldAction::Hold)
    );
    assert!(hold.is_holding());
    assert_eq!(hold.step(hold_input(true, WATCHER_HOLD_MASK, false)), None);
    assert_eq!(
        hold.step(hold_input(false, WATCHER_HOLD_MASK, false)),
        Some(WatcherHoldAction::Release)
    );
    assert!(!hold.is_holding());
}

#[test]
fn a_watcher_with_its_own_talk_is_never_held() {
    let mut hold = WatcherHold::new();
    assert_eq!(hold.step(hold_input(true, WATCHER_HOLD_MASK, false)), None);
    assert!(!hold.is_holding());
}

#[test]
fn an_own_talk_taking_the_mask_leaves_it_alone_on_release() {
    let mut hold = WatcherHold::new();
    hold.step(hold_input(true, 0, false));
    assert_eq!(hold.step(hold_input(false, WATCHER_HOLD_MASK, true)), None);
    assert!(!hold.is_holding());
    assert_eq!(hold.step(hold_input(false, WATCHER_HOLD_MASK, false)), None);
}

#[test]
fn a_hold_whose_mask_was_cleared_by_the_game_writes_nothing_on_release() {
    let mut hold = WatcherHold::new();
    hold.step(hold_input(true, 0, false));
    assert_eq!(hold.step(hold_input(false, 0, false)), None);
    assert!(!hold.is_holding());
}

#[test]
fn an_abandoned_hold_is_held_again_on_the_next_tick() {
    let mut hold = WatcherHold::new();
    hold.step(hold_input(true, 0, false));
    hold.abandon();
    assert!(!hold.is_holding());
    assert_eq!(
        hold.step(hold_input(true, 0, false)),
        Some(WatcherHoldAction::Hold)
    );
}

#[test]
fn losing_the_link_releases_a_held_watcher() {
    let mut hold = WatcherHold::new();
    hold.step(hold_input(true, 0, false));
    assert_eq!(
        hold.release_on_link_loss(WATCHER_HOLD_MASK),
        Some(WatcherHoldAction::Release)
    );
    assert_eq!(hold.release_on_link_loss(0), None);
}

#[test]
fn the_player_tag_in_a_line_becomes_the_talkers_name() {
    assert_eq!(
        replace_player_name_tag(b"Hello <NIER>, <NIER>!", "Ayla"),
        Some(b"Hello Ayla, Ayla!".to_vec())
    );
    assert_eq!(replace_player_name_tag(b"No tag here", "Ayla"), None);
    assert_eq!(replace_player_name_tag(b"<NIER>", "   "), None);
}

#[test]
fn only_the_protagonists_speaker_name_is_replaced() {
    assert_eq!(
        speaker_name_to_show("<NIER>", "Host", Some("Guest")),
        Some("Guest".to_owned())
    );
    assert_eq!(
        speaker_name_to_show("Host", "Host", Some("Guest")),
        Some("Guest".to_owned())
    );
    assert_eq!(speaker_name_to_show("Emil", "Host", Some("Guest")), None);
    assert_eq!(speaker_name_to_show("<NIER>", "Host", None), None);
    assert_eq!(speaker_name_to_show("", "", Some("Guest")), None);
}

fn guest_with_own_talk(own: DialogueLine, host: Option<DialogueLine>, now_ms: u64) -> MirrorInput {
    MirrorInput {
        window_open: true,
        own_talk: Some(own),
        leader: TalkLeader::Partner,
        ..mirror_input(host, now_ms)
    }
}

#[test]
fn the_guest_follows_the_hosts_lines_through_a_talk_both_opened() {
    let mut mirror = PartnerTalkMirror::new();
    assert_eq!(
        mirror.step(guest_with_own_talk(line(1002, 0), Some(line(1002, 0)), 0)),
        Some(MirrorAction::FollowStarted(1002))
    );
    assert!(mirror.holds_own_talk());
    assert_eq!(
        mirror.step(guest_with_own_talk(line(1002, 0), Some(line(1002, 0)), 10)),
        None
    );
    assert_eq!(
        mirror.step(guest_with_own_talk(line(1002, 0), Some(line(1002, 1)), 20)),
        Some(MirrorAction::FollowLine(1))
    );
    assert_eq!(
        mirror.step(guest_with_own_talk(line(1002, 1), Some(line(2605, 0)), 30)),
        Some(MirrorAction::FinishOwnTalk(1002))
    );
    assert!(!mirror.holds_own_talk());
    assert_eq!(
        mirror.step(guest_with_own_talk(line(2605, 0), Some(line(2605, 0)), 40)),
        Some(MirrorAction::FollowStarted(2605))
    );
    assert!(mirror.holds_own_talk());
}

#[test]
fn the_guest_finishes_its_copy_when_the_host_ends_the_talk() {
    let mut mirror = PartnerTalkMirror::new();
    mirror.step(guest_with_own_talk(line(2606, 2), Some(line(2606, 2)), 0));
    assert_eq!(
        mirror.step(guest_with_own_talk(line(2606, 2), None, 10)),
        Some(MirrorAction::FinishOwnTalk(2606))
    );
    assert_eq!(
        mirror.step(guest_with_own_talk(line(2606, 2), None, 20)),
        None
    );
}

#[test]
fn a_guest_talk_the_host_never_shows_goes_on_alone_after_a_short_wait() {
    let mut mirror = PartnerTalkMirror::new();
    assert_eq!(
        mirror.step(guest_with_own_talk(line(76, 0), None, 0)),
        Some(MirrorAction::FollowStarted(76))
    );
    assert_eq!(
        mirror.step(guest_with_own_talk(line(76, 0), None, 100)),
        None
    );
    assert_eq!(
        mirror.step(guest_with_own_talk(
            line(76, 0),
            None,
            FOLLOW_HOST_TALK_WAIT_MS
        )),
        Some(MirrorAction::FollowGaveUp(76))
    );
    assert!(!mirror.holds_own_talk());
}

#[test]
fn the_host_never_jumps_to_the_guests_line() {
    let mut mirror = PartnerTalkMirror::new();
    let host = MirrorInput {
        window_open: true,
        own_talk: Some(line(2605, 0)),
        leader: TalkLeader::Local,
        ..mirror_input(Some(line(2606, 2)), 0)
    };
    assert_eq!(mirror.step(host), None);
    assert!(!mirror.holds_own_talk());
}

#[test]
fn a_talk_this_player_just_finished_is_not_mirrored_back_from_the_partner() {
    let mut mirror = PartnerTalkMirror::new();
    let own = MirrorInput {
        window_open: true,
        own_talk: Some(line(2606, 2)),
        ..mirror_input(Some(line(2606, 2)), 0)
    };
    mirror.step(own);
    assert_eq!(
        mirror.step(mirror_input(Some(line(2606, 2)), 100)),
        Some(MirrorAction::SkipEndedTalk(2606))
    );
    assert_eq!(mirror.step(mirror_input(Some(line(2606, 2)), 200)), None);
    assert_eq!(
        mirror.step(mirror_input(
            Some(line(2606, 2)),
            100 + OWN_ENDED_TALK_NOT_MIRRORED_MS
        )),
        Some(MirrorAction::Open(line(2606, 2)))
    );
}

#[test]
fn the_partners_end_or_next_talk_lifts_the_ended_talk_guard() {
    let mut mirror = PartnerTalkMirror::new();
    let own = MirrorInput {
        window_open: true,
        own_talk: Some(line(76, 0)),
        ..mirror_input(Some(line(76, 0)), 0)
    };
    mirror.step(own);
    assert_eq!(mirror.step(mirror_input(None, 10)), None);
    assert_eq!(
        mirror.step(mirror_input(Some(line(76, 0)), 20)),
        Some(MirrorAction::Open(line(76, 0)))
    );
}

#[test]
fn a_mask_the_mirror_wrote_is_not_an_own_talk() {
    assert!(own_talk_started(0x20, false, false, false));
    assert!(!own_talk_started(0x20, true, false, false));
    assert!(!own_talk_started(0, false, false, false));
    assert!(!own_talk_started(WATCHER_HOLD_MASK, false, true, false));
    assert!(own_talk_started(0x20, false, true, true));
}

fn guest_tick(
    mirror: &mut PartnerTalkMirror,
    memory: &PartnerTalkMemory,
    own: Option<DialogueLine>,
    now_ms: u64,
) -> Option<MirrorAction> {
    mirror.step(MirrorInput {
        window_open: true,
        own_talk: own,
        leader: TalkLeader::Partner,
        partner_ended_talk: memory.ended_for_follow(own, TalkLeader::Partner, now_ms),
        ..mirror_input(memory.for_follow(own, TalkLeader::Partner), now_ms)
    })
}

#[test]
fn the_guest_follows_a_host_that_was_a_tick_ahead_and_never_gives_up() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(1057, 0)), 0);
    let own = Some(line(1057, 0));
    assert_eq!(
        guest_tick(&mut mirror, &host, own, 0),
        Some(MirrorAction::FollowStarted(1057))
    );
    host.receive(DialogueUpdate::Line(line(1057, 1)), 0);
    assert_eq!(
        guest_tick(&mut mirror, &host, own, 50),
        Some(MirrorAction::FollowLine(1))
    );
    host.receive(DialogueUpdate::Ended, 0);
    let own = Some(line(1057, 1));
    assert_eq!(
        guest_tick(&mut mirror, &host, own, 100),
        Some(MirrorAction::FinishOwnTalk(1057))
    );
    assert_eq!(
        guest_tick(&mut mirror, &host, own, FOLLOW_HOST_TALK_WAIT_MS * 3),
        None
    );
}

#[test]
fn a_host_line_stays_seen_while_the_host_reads_longer_than_the_wait() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(1056, 0)), 0);
    let own = Some(line(1056, 0));
    guest_tick(&mut mirror, &host, own, FOLLOW_HOST_TALK_WAIT_MS * 5);
    assert_eq!(
        guest_tick(&mut mirror, &host, own, FOLLOW_HOST_TALK_WAIT_MS * 6),
        None
    );
    assert!(mirror.holds_own_talk());
}

#[test]
fn a_talk_the_host_never_opens_still_gives_up_after_the_wait() {
    let mut mirror = PartnerTalkMirror::new();
    let host = PartnerTalkMemory::new();
    let own = Some(line(76, 0));
    guest_tick(&mut mirror, &host, own, 0);
    assert_eq!(
        guest_tick(&mut mirror, &host, own, FOLLOW_HOST_TALK_WAIT_MS),
        Some(MirrorAction::FollowGaveUp(76))
    );
}

#[test]
fn an_npc_talk_only_the_host_has_open_is_not_opened_on_the_guest() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(500, 0)), 0);
    assert_eq!(host.for_follow(None, TalkLeader::Partner), None);
    assert_eq!(
        mirror.step(MirrorInput {
            can_show: true,
            leader: TalkLeader::Partner,
            ..mirror_input(host.for_follow(None, TalkLeader::Partner), 0)
        }),
        None
    );
    assert_eq!(host.for_follow(Some(line(9, 0)), TalkLeader::Local), None);
}

fn host_that_ended_talk_at(talk_id: u32, ended_ms: u64) -> PartnerTalkMemory {
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(talk_id, 0)), 0);
    host.receive(DialogueUpdate::Ended, ended_ms);
    host
}

#[test]
fn a_copy_opened_within_the_wait_after_the_host_left_closes_at_once() {
    let mut mirror = PartnerTalkMirror::new();
    let host = host_that_ended_talk_at(1057, 1000);
    assert_eq!(
        guest_tick(
            &mut mirror,
            &host,
            Some(line(1057, 0)),
            1000 + FOLLOW_HOST_TALK_WAIT_MS
        ),
        Some(MirrorAction::FinishOwnTalk(1057))
    );
    assert!(!mirror.holds_own_talk());
    assert_eq!(
        guest_tick(
            &mut mirror,
            &host,
            Some(line(1057, 0)),
            1000 + FOLLOW_HOST_TALK_WAIT_MS * 3
        ),
        None
    );
}

#[test]
fn a_copy_opened_after_the_wait_runs_by_its_own_buttons() {
    let mut mirror = PartnerTalkMirror::new();
    let host = host_that_ended_talk_at(1057, 1000);
    assert_eq!(
        guest_tick(
            &mut mirror,
            &host,
            Some(line(1057, 0)),
            1001 + FOLLOW_HOST_TALK_WAIT_MS
        ),
        Some(MirrorAction::FollowStarted(1057))
    );
}

#[test]
fn a_different_talk_than_the_one_the_host_left_is_not_closed() {
    let mut mirror = PartnerTalkMirror::new();
    let host = host_that_ended_talk_at(1056, 1000);
    assert_eq!(
        guest_tick(&mut mirror, &host, Some(line(1057, 0)), 1100),
        Some(MirrorAction::FollowStarted(1057))
    );
}

#[test]
fn a_guest_talk_started_by_its_own_talk_command_is_not_followed_or_closed() {
    let leader = leader_for_own_talk(TalkLeader::Partner, true);
    assert_eq!(leader, TalkLeader::Local);
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    host.receive(DialogueUpdate::Line(line(1057, 0)), 0);
    let own = Some(line(1057, 0));
    let tick = |mirror: &mut PartnerTalkMirror, host: &PartnerTalkMemory, now_ms| {
        mirror.step(MirrorInput {
            window_open: true,
            own_talk: own,
            leader,
            partner_ended_talk: host.ended_for_follow(own, leader, now_ms),
            ..mirror_input(host.for_follow(own, leader), now_ms)
        })
    };
    assert_eq!(tick(&mut mirror, &host, 0), None);
    host.receive(DialogueUpdate::Line(line(1057, 1)), 50);
    assert_eq!(tick(&mut mirror, &host, 50), None);
    host.receive(DialogueUpdate::Ended, 100);
    assert_eq!(tick(&mut mirror, &host, 100), None);
    assert!(!mirror.holds_own_talk());
}

#[test]
fn a_script_started_guest_copy_still_follows_the_host() {
    assert_eq!(
        leader_for_own_talk(TalkLeader::Partner, false),
        TalkLeader::Partner
    );
    assert_eq!(
        leader_for_own_talk(TalkLeader::Either, true),
        TalkLeader::Either
    );
}

const PROMPT_TALK: u32 = 1089;
const PROMPT_LINE: u16 = 2;

fn host_pick(index: u8) -> TalkChoice {
    TalkChoice {
        talk_id: PROMPT_TALK,
        line: PROMPT_LINE,
        kind: TalkChoiceKind::TalkList,
        index,
        target: 1090 + u32::from(index),
    }
}

fn choice_input(
    own_line: Option<u16>,
    prompt: Option<TalkChoiceKind>,
    now_ms: u64,
) -> ChoiceFollowInput {
    ChoiceFollowInput {
        follows_host: true,
        own_talk: own_line.map(|own_line| line(PROMPT_TALK, own_line)),
        prompt,
        now_ms,
    }
}

#[test]
fn the_hosts_answer_is_taken_when_the_guests_prompt_is_open() {
    let mut follower = ChoiceFollower::new();
    follower.receive(host_pick(1), 0);
    assert_eq!(
        follower.step(choice_input(
            Some(PROMPT_LINE),
            Some(TalkChoiceKind::TalkList),
            10
        )),
        Some(ChoiceStep::Take(host_pick(1)))
    );
    assert_eq!(
        follower.step(choice_input(
            Some(PROMPT_LINE),
            Some(TalkChoiceKind::TalkList),
            20
        )),
        None
    );
    assert!(follower.is_waiting());
    assert_eq!(follower.step(choice_input(None, None, 30)), None);
    assert!(!follower.is_waiting());
}

#[test]
fn an_answer_that_comes_first_waits_until_the_guests_prompt_opens() {
    let mut follower = ChoiceFollower::new();
    follower.receive(host_pick(0), 0);
    assert_eq!(follower.step(choice_input(Some(0), None, 10)), None);
    assert_eq!(
        follower.step(choice_input(Some(PROMPT_LINE), None, 20)),
        None
    );
    assert_eq!(
        follower.step(choice_input(
            Some(PROMPT_LINE),
            Some(TalkChoiceKind::MenuList),
            30
        )),
        None
    );
    assert_eq!(
        follower.step(choice_input(
            Some(PROMPT_LINE),
            Some(TalkChoiceKind::TalkList),
            40
        )),
        Some(ChoiceStep::Take(host_pick(0)))
    );
}

#[test]
fn an_answer_is_dropped_when_the_guests_talk_ends_or_the_phase_changes() {
    let mut follower = ChoiceFollower::new();
    follower.receive(host_pick(1), 0);
    assert_eq!(follower.step(choice_input(Some(0), None, 10)), None);
    assert_eq!(
        follower.step(choice_input(None, None, 20)),
        Some(ChoiceStep::Drop(host_pick(1)))
    );
    assert!(!follower.is_waiting());
    follower.receive(host_pick(1), 100);
    let left_follow = ChoiceFollowInput {
        follows_host: false,
        ..choice_input(Some(PROMPT_LINE), Some(TalkChoiceKind::TalkList), 110)
    };
    assert_eq!(
        follower.step(left_follow),
        Some(ChoiceStep::Drop(host_pick(1)))
    );
}

#[test]
fn an_answer_for_a_talk_the_guest_never_opened_is_dropped_after_the_limit() {
    let mut follower = ChoiceFollower::new();
    follower.receive(host_pick(1), 0);
    assert_eq!(follower.step(choice_input(None, None, 100)), None);
    assert_eq!(
        follower.step(choice_input(
            None,
            None,
            CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS
        )),
        Some(ChoiceStep::Drop(host_pick(1)))
    );
}

#[test]
fn a_prompt_that_never_opens_gives_the_guest_its_buttons_after_the_limit() {
    let mut follower = ChoiceFollower::new();
    follower.receive(host_pick(1), 1000);
    assert_eq!(follower.step(choice_input(Some(0), None, 5999)), None);
    assert_eq!(
        follower.step(choice_input(
            Some(0),
            None,
            1000 + CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS
        )),
        Some(ChoiceStep::GiveUp(host_pick(1)))
    );
    assert!(!follower.is_waiting());
}

#[test]
fn the_open_prompt_kind_comes_from_the_window_state() {
    assert_eq!(open_prompt_kind(7, 0), Some(TalkChoiceKind::TalkList));
    assert_eq!(open_prompt_kind(9, 0), Some(TalkChoiceKind::MenuList));
    assert_eq!(open_prompt_kind(6, 3), Some(TalkChoiceKind::TalkList));
    assert_eq!(open_prompt_kind(6, 5), Some(TalkChoiceKind::MenuList));
    assert_eq!(open_prompt_kind(6, 4), None);
    assert_eq!(open_prompt_kind(4, 3), None);
}

fn prompt_tick(
    mirror: &mut PartnerTalkMirror,
    memory: &PartnerTalkMemory,
    own: DialogueLine,
    answer_open: bool,
    now_ms: u64,
) -> Option<MirrorAction> {
    mirror.step(MirrorInput {
        answer_open,
        ..MirrorInput {
            window_open: true,
            own_talk: Some(own),
            leader: TalkLeader::Partner,
            partner_ended_talk: memory.ended_for_follow(Some(own), TalkLeader::Partner, now_ms),
            ..mirror_input(memory.for_follow(Some(own), TalkLeader::Partner), now_ms)
        }
    })
}

#[test]
fn a_copy_at_an_answer_prompt_is_not_closed_when_the_host_moves_on() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    let own = line(PROMPT_TALK, PROMPT_LINE);
    host.receive(DialogueUpdate::Line(own), 0);
    assert_eq!(
        prompt_tick(&mut mirror, &host, own, true, 0),
        Some(MirrorAction::FollowStarted(PROMPT_TALK))
    );
    host.receive(DialogueUpdate::Line(line(1091, 0)), 100);
    assert_eq!(prompt_tick(&mut mirror, &host, own, true, 100), None);
    assert!(mirror.holds_own_talk());
    assert_eq!(prompt_tick(&mut mirror, &host, own, true, 3000), None);
    assert!(mirror.holds_own_talk());
}

#[test]
fn a_copy_whose_answer_was_taken_is_closed_normally_without_a_prompt() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    let own = line(PROMPT_TALK, PROMPT_LINE);
    host.receive(DialogueUpdate::Line(own), 0);
    prompt_tick(&mut mirror, &host, own, true, 0);
    host.receive(DialogueUpdate::Line(line(1091, 0)), 100);
    assert_eq!(
        prompt_tick(&mut mirror, &host, own, false, 100),
        Some(MirrorAction::FinishOwnTalk(PROMPT_TALK))
    );
}

#[test]
fn a_prompt_the_host_never_answers_gives_the_guest_its_buttons_after_the_limit() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    let own = line(PROMPT_TALK, PROMPT_LINE);
    host.receive(DialogueUpdate::Line(own), 0);
    prompt_tick(&mut mirror, &host, own, true, 0);
    host.receive(DialogueUpdate::Ended, 100);
    assert_eq!(prompt_tick(&mut mirror, &host, own, true, 100), None);
    assert_eq!(
        prompt_tick(
            &mut mirror,
            &host,
            own,
            true,
            100 + CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS
        ),
        Some(MirrorAction::ChoiceGaveUp(PROMPT_TALK))
    );
    assert!(!mirror.holds_own_talk());
}

#[test]
fn releasing_the_followed_talk_gives_the_buttons_back() {
    let mut mirror = PartnerTalkMirror::new();
    let mut host = PartnerTalkMemory::new();
    let own = line(PROMPT_TALK, PROMPT_LINE);
    host.receive(DialogueUpdate::Line(own), 0);
    prompt_tick(&mut mirror, &host, own, true, 0);
    assert!(mirror.holds_own_talk());
    mirror.release_followed_talk();
    assert!(!mirror.holds_own_talk());
}
