use replicant_game::kaine::{swapped_body_box_grab_refused_text, swapped_body_cannot_move_boxes};
use replicant_proto::PartnerBody;

#[test]
fn only_the_swapped_body_is_refused_a_box() {
    assert!(swapped_body_cannot_move_boxes(
        PartnerBody::SeedKaineE,
        true
    ));
    assert!(!swapped_body_cannot_move_boxes(
        PartnerBody::SeedKaineE,
        false
    ));
    assert!(!swapped_body_cannot_move_boxes(PartnerBody::Nier, true));
    assert!(!swapped_body_cannot_move_boxes(PartnerBody::Nier, false));
    assert!(!swapped_body_cannot_move_boxes(
        PartnerBody::PrologueCharacter,
        false
    ));
}

#[test]
fn the_refusal_line_names_the_box_and_who_moves_it() {
    // Run 1114f guest 12:47:18.255: PlayerHoldBox fired on the swapped body's own grab.
    assert_eq!(
        swapped_body_box_grab_refused_text(0x9c96_8680),
        "objects: refused your grab of box 0x9c968680: the swapped body has no box push or pull motions, the box stays the host's to move"
    );
}
