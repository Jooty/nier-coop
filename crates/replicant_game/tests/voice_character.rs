use replicant_game::effects::{
    FirstTimeNames, VoiceCharacterSchedule, read_character_state, wwise_name_hash,
};
use replicant_game::offsets::{CHARACTER_RECORD_NAME_OFFSET, PLAYER_CHARACTER_RECORD_OFFSET};

use crate::fake_memory::FakeMemory;

const ACTOR: u64 = 0x3_0000_0000;
const RECORD: u64 = 0x3_1000_0000;
const NAME: u64 = 0x3_1000_0100;

fn memory_with_character(name: &str) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        ACTOR + PLAYER_CHARACTER_RECORD_OFFSET,
        &RECORD.to_le_bytes(),
    );
    let relative = i32::try_from(NAME - (RECORD + CHARACTER_RECORD_NAME_OFFSET)).unwrap();
    memory.put(
        RECORD + CHARACTER_RECORD_NAME_OFFSET,
        &relative.to_le_bytes(),
    );
    let mut text = name.as_bytes().to_vec();
    text.push(0);
    memory.put(NAME, &text);
    memory
}

#[test]
fn the_name_hash_is_the_lowercase_fnv_1_the_game_posts_events_by() {
    assert_eq!(wwise_name_hash("a"), 0x050C_5D7E);
    assert_eq!(wwise_name_hash("Kaine"), wwise_name_hash("kaine"));
    assert_eq!(wwise_name_hash(""), 0x811C_9DC5);
}

#[test]
fn the_players_character_is_the_string_its_record_points_to() {
    let memory = memory_with_character("kaine");
    assert_eq!(
        read_character_state(&memory, ACTOR).unwrap(),
        Some("kaine".to_owned())
    );
}

#[test]
fn an_actor_without_a_character_record_has_no_character() {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR + PLAYER_CHARACTER_RECORD_OFFSET, &0_u64.to_le_bytes());
    assert_eq!(read_character_state(&memory, ACTOR).unwrap(), None);
}

#[test]
fn a_character_is_sent_when_it_changes_and_again_every_few_seconds() {
    let mut schedule = VoiceCharacterSchedule::new();
    assert!(schedule.is_due("nier", 0));
    assert!(!schedule.is_due("nier", 1000));
    assert!(schedule.is_due("kaine", 1500));
    assert!(!schedule.is_due("kaine", 3000));
    assert!(schedule.is_due("kaine", 4600));
    schedule.reset();
    assert!(schedule.is_due("kaine", 4700));
}

#[test]
fn each_sound_name_is_reported_first_once() {
    let mut names = FirstTimeNames::new();
    assert!(names.is_first("move_jump_begin"));
    assert!(!names.is_first("move_jump_begin"));
    assert!(names.is_first("move_fall"));
}
