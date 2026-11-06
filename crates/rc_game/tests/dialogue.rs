use std::cell::Cell;

use rc_game::dialogue::{
    DialogueTracker, LineText, TALK_LINE_COUNT_OFFSET, TALK_LINE_SPEAKER_ID_OFFSET,
    TALK_LINE_TEXT_OFFSET, TALK_LINES_OFFSET, TALK_TABLE_SCAN_CHUNK_SLOTS,
    TALK_WINDOW_CURRENT_LINE_OFFSET, TALK_WINDOW_CURRENT_TALK_OFFSET, TALK_WINDOW_RVA,
    TALK_WINDOW_TALK_TABLE_OFFSET, TALKER_NAME_COUNT_OFFSET, TALKER_NAME_ENTRIES_OFFSET,
    TALKER_NAME_TABLE_RVA, TalkCursor, find_talk_id, partner_line_events, read_line_text,
    read_talk_cursor,
};
use rc_game::event_log::EVENT_LOG_MAX_TEXT_CHARS;
use rc_game::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA, read_world_flags};
use rc_proto::{DialogueLine, DialogueUpdate, PartnerActivity};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const TALK_ADDRESS: u64 = 0x2_0000_0000;
const LINE_ADDRESS: u64 = 0x2_0000_1000;
const TEXT_ADDRESS: u64 = 0x2_0000_2000;
const SPEAKER_NAME_ADDRESS: u64 = 0x2_0000_3000;
const SPEAKER_ID: i32 = 7;

fn window() -> u64 {
    FAKE_MODULE_BASE + u64::from(TALK_WINDOW_RVA)
}

fn put_c_string(memory: &mut FakeMemory, address: u64, text: &str) {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    memory.put(address, &bytes);
}

fn memory_with_talk(talk_id: u32, text: &str) -> FakeMemory {
    let mut memory = FakeMemory::new();
    let table = window() + TALK_WINDOW_TALK_TABLE_OFFSET;
    memory.put(table + u64::from(talk_id) * 8, &TALK_ADDRESS.to_le_bytes());
    memory.put(TALK_ADDRESS + TALK_LINE_COUNT_OFFSET, &2_i32.to_le_bytes());
    memory.put(TALK_ADDRESS + TALK_LINES_OFFSET, &0_u64.to_le_bytes());
    memory.put(
        TALK_ADDRESS + TALK_LINES_OFFSET + 8,
        &LINE_ADDRESS.to_le_bytes(),
    );
    memory.put(
        LINE_ADDRESS + TALK_LINE_TEXT_OFFSET,
        &TEXT_ADDRESS.to_le_bytes(),
    );
    memory.put(
        LINE_ADDRESS + TALK_LINE_SPEAKER_ID_OFFSET,
        &SPEAKER_ID.to_le_bytes(),
    );
    put_c_string(&mut memory, TEXT_ADDRESS, text);
    let names = FAKE_MODULE_BASE + u64::from(TALKER_NAME_TABLE_RVA);
    memory.put(names + TALKER_NAME_COUNT_OFFSET, &2_i32.to_le_bytes());
    memory.put(names + TALKER_NAME_ENTRIES_OFFSET, &3_i32.to_le_bytes());
    memory.put(names + TALKER_NAME_ENTRIES_OFFSET + 8, &0_u64.to_le_bytes());
    memory.put(
        names + TALKER_NAME_ENTRIES_OFFSET + 0x10,
        &SPEAKER_ID.to_le_bytes(),
    );
    memory.put(
        names + TALKER_NAME_ENTRIES_OFFSET + 0x18,
        &SPEAKER_NAME_ADDRESS.to_le_bytes(),
    );
    put_c_string(&mut memory, SPEAKER_NAME_ADDRESS, "Clerk");
    memory
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
fn a_line_reads_its_text_and_speaker_from_this_games_talk_data() {
    let memory = memory_with_talk(42, "Hello there.");
    assert_eq!(
        read_line_text(&memory, line(42, 1)).unwrap(),
        Some(LineText {
            speaker: Some("Clerk".to_owned()),
            text: "Hello there.".to_owned(),
        })
    );
    assert_eq!(read_line_text(&memory, line(42, 0)).unwrap(), None);
    assert_eq!(read_line_text(&memory, line(42, 2)).unwrap(), None);
}

#[test]
fn a_talk_not_loaded_here_has_no_text() {
    let mut memory = memory_with_talk(42, "Hello there.");
    let table = window() + TALK_WINDOW_TALK_TABLE_OFFSET;
    memory.put(table + 43 * 8, &0_u64.to_le_bytes());
    assert_eq!(read_line_text(&memory, line(43, 1)).unwrap(), None);
}

#[test]
fn the_protagonist_tag_becomes_the_partner_name_and_other_tags_go() {
    let entry = partner_line_events(
        "Ana",
        &LineText {
            speaker: Some("<NIER>".to_owned()),
            text: "Thanks,\n<COLOR>friend</COLOR>, <NIER>.".to_owned(),
        },
    );
    assert_eq!(entry, vec!["Ana: Thanks, friend, Ana.".to_owned()]);
}

#[test]
fn a_long_line_splits_at_word_breaks_to_fit_the_event_log() {
    let text = "word ".repeat(40);
    let entries = partner_line_events(
        "Ana",
        &LineText {
            speaker: None,
            text,
        },
    );
    assert!(entries.len() > 1);
    assert!(
        entries
            .iter()
            .all(|entry| entry.chars().count() <= EVENT_LOG_MAX_TEXT_CHARS)
    );
    assert_eq!(entries.join(" "), "word ".repeat(40).trim_end());
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
    let activity = |event: u8, mask: u32, window_state: i32| {
        let mut memory = FakeMemory::new();
        memory.put_at_rva(EVENT_SCENE_BYTE_RVA, &[event]);
        let slots: Vec<u8> = [mask, 0, 0, 0]
            .iter()
            .flat_map(|slot| slot.to_le_bytes())
            .collect();
        memory.put_at_rva(CONTROL_MASK_FIRST_RVA, &slots);
        memory.put(window(), &window_state.to_le_bytes());
        read_world_flags(&memory).unwrap().activity()
    };
    assert_eq!(activity(0, 0xA0, 1), PartnerActivity::InDialogue);
    assert_eq!(activity(0, 0xA0, 0), PartnerActivity::Playing);
    assert_eq!(activity(1, 0xA0, 1), PartnerActivity::InCutscene);
    assert_eq!(activity(0, 0x19F, 1), PartnerActivity::InMenu);
    assert!(!PartnerActivity::InDialogue.world_paused());
    assert_eq!(PartnerActivity::InDialogue.label(), Some("in dialogue"));
}
