use replicant_game::kaine::{
    BOX_GRAB_COUNTS_AS_NEW_AFTER_A_GAP_OF_MS, SwappedBoxGrabNote, swapped_body_box_grab_text,
    swapped_body_moves_boxes_without_motions,
};
use replicant_proto::PartnerBody;

const BOX: u64 = 0x9c96_8680;
const OTHER_BOX: u64 = 0x9d0a_58e0;

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
fn a_grab_is_noted_once_however_many_frames_the_gate_is_asked() {
    // Run 1114f guest 12:47:18.255: PlayerHoldBox on the swapped body's own grab, held about 4 s.
    let mut note = SwappedBoxGrabNote::default();
    assert!(note.observe(BOX, 1_000));
    for frame in 1..240_u64 {
        assert!(!note.observe(BOX, 1_000 + frame * 16));
    }
}

#[test]
fn another_box_or_a_grab_after_a_gap_is_noted_again() {
    let mut note = SwappedBoxGrabNote::new();
    assert!(note.observe(BOX, 1_000));
    assert!(note.observe(OTHER_BOX, 1_016));
    assert!(!note.observe(OTHER_BOX, 1_032));
    assert!(note.observe(OTHER_BOX, 1_032 + BOX_GRAB_COUNTS_AS_NEW_AFTER_A_GAP_OF_MS));
}

#[test]
fn the_grab_line_says_the_box_moves_with_no_box_motions() {
    assert_eq!(
        swapped_body_box_grab_text(BOX),
        "objects: the swapped body is moving box 0x9c968680 with no box motions (it has no box push or pull motions)"
    );
}
