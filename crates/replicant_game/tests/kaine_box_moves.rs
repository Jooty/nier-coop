use replicant_game::kaine::{
    SwappedBoxMoveNote, swapped_body_box_move_text, swapped_body_moves_boxes_without_motions,
};
use replicant_proto::PartnerBody;

const BOX_HANDLE: u64 = 0x0000_0119_0000_0FC5;
const OTHER_BOX_HANDLE: u64 = 0x0000_0129_0000_0F70;

#[test]
fn only_the_swapped_body_moves_boxes_without_motions() {
    assert!(swapped_body_moves_boxes_without_motions(
        PartnerBody::SeedKaineE,
        true
    ));
    assert!(!swapped_body_moves_boxes_without_motions(
        PartnerBody::SeedKaineE,
        false
    ));
    assert!(!swapped_body_moves_boxes_without_motions(
        PartnerBody::Nier,
        true
    ));
    assert!(!swapped_body_moves_boxes_without_motions(
        PartnerBody::PrologueCharacter,
        false
    ));
}

#[test]
fn the_per_frame_can_grab_probe_of_two_boxes_is_never_noted() {
    // Run 1114h guest from 13:18:53.085: the gate was asked for 0x9ff77a60 and 0x9ff90d90 in turn
    // every frame and the old note logged 9,152 lines; the body never left its own move state.
    let mut note = SwappedBoxMoveNote::default();
    for _frame in 0..9_152 {
        assert!(!note.observe(None));
    }
}

#[test]
fn entering_the_box_move_state_is_noted_once_however_long_it_lasts() {
    let mut note = SwappedBoxMoveNote::new();
    assert!(!note.observe(None));
    assert!(note.observe(Some(BOX_HANDLE)));
    for _frame in 1..240 {
        assert!(!note.observe(Some(BOX_HANDLE)));
    }
}

#[test]
fn a_new_entry_or_another_box_is_noted_again() {
    let mut note = SwappedBoxMoveNote::new();
    assert!(note.observe(Some(BOX_HANDLE)));
    assert!(!note.observe(None));
    assert!(note.observe(Some(BOX_HANDLE)));
    assert!(note.observe(Some(OTHER_BOX_HANDLE)));
}

#[test]
fn the_box_move_line_names_the_box_and_says_it_has_no_box_motions() {
    assert_eq!(
        swapped_body_box_move_text(BOX_HANDLE, Some(0xA70C_4D50)),
        "objects: the swapped body is moving box handle 0x11900000fc5 (actor 0xa70c4d50): it entered the box move state, with no box push or pull motions"
    );
    assert_eq!(
        swapped_body_box_move_text(BOX_HANDLE, None),
        "objects: the swapped body is moving box handle 0x11900000fc5 (actor unresolved): it entered the box move state, with no box push or pull motions"
    );
}
