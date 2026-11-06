use rc_game::Replicant;
use rc_game::offsets::{ENTITY_NAME_OFFSET, ENTITY_RVA};
use rc_game::player_name::{
    NAME_FIELD_LEN, diary_name, fit_name_field, local_name, name_field_bytes,
};

use crate::fake_memory::FakeMemory;

fn game_with_name(name: &[u8]) -> Replicant<FakeMemory> {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(ENTITY_RVA, &[0_u8; 0x4A8]);
    memory.put_at_rva(ENTITY_RVA + ENTITY_NAME_OFFSET, name);
    Replicant::new(memory)
}

#[test]
fn a_plain_name_is_kept_as_it_is() {
    assert_eq!(fit_name_field("Ayla").as_deref(), Some("Ayla"));
}

#[test]
fn an_empty_or_blank_name_is_nothing() {
    assert_eq!(fit_name_field(""), None);
    assert_eq!(fit_name_field("   \t"), None);
}

#[test]
fn control_characters_and_characters_above_the_basic_plane_are_dropped() {
    assert_eq!(fit_name_field("Ay\nla\u{1F600}").as_deref(), Some("Ayla"));
}

#[test]
fn a_long_name_is_cut_to_leave_room_for_the_terminator() {
    let fitted = fit_name_field(&"a".repeat(80)).expect("not empty");
    assert_eq!(fitted.len(), NAME_FIELD_LEN - 1);
}

#[test]
fn a_long_name_is_never_cut_inside_a_character() {
    let fitted = fit_name_field(&"\u{e9}".repeat(40)).expect("not empty");
    assert!(fitted.len() < NAME_FIELD_LEN);
    assert!(fitted.chars().all(|character| character == '\u{e9}'));
}

#[test]
fn the_field_bytes_end_in_zeroes() {
    let field = name_field_bytes("Ayla").expect("not empty");
    assert_eq!(&field[..5], b"Ayla\0");
    assert!(field[4..].iter().all(|&byte| byte == 0));
}

#[test]
fn the_launcher_name_wins_over_the_save_block_name() {
    assert_eq!(local_name(Some("Ayla"), "Rook").as_deref(), Some("Ayla"));
}

#[test]
fn the_save_block_name_is_used_when_the_launcher_has_none() {
    assert_eq!(local_name(None, "Rook").as_deref(), Some("Rook"));
    assert_eq!(local_name(Some("  "), "Rook").as_deref(), Some("Rook"));
}

#[test]
fn the_diary_shows_the_partner_and_leaves_the_save_alone_without_one() {
    assert_eq!(diary_name(None), None);
    assert_eq!(diary_name(Some("Ayla")), name_field_bytes("Ayla"));
}

#[test]
fn the_whole_save_name_field_is_read_and_written_back_exactly() {
    let long_name = [b'x'; 31];
    let mut game = game_with_name(&long_name);
    let before = game.save_name_field().expect("readable");
    assert_eq!(game.save_name().expect("readable").len(), 31);
    game.set_save_name_field(&name_field_bytes("Ayla").expect("not empty"))
        .expect("writable");
    assert_eq!(game.save_name().as_deref(), Ok("Ayla"));
    game.set_save_name_field(&before).expect("writable");
    assert_eq!(game.save_name_field(), Ok(before));
}
