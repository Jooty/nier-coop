use rc_game::puppet::{BOOK_BOB_PERIOD_MS, BOOK_HOVER_AMPLITUDE, book_hover_offset, book_pack_name};
use rc_proto::PartnerBody;

#[test]
fn the_prologue_body_gets_its_own_book_and_the_others_the_default() {
    assert_eq!(book_pack_name(PartnerBody::PrologueCharacter), "bookb012");
    for body in [
        PartnerBody::Nier,
        PartnerBody::SeedNierY,
        PartnerBody::SeedNierF,
        PartnerBody::SeedKaineE,
    ] {
        assert_eq!(book_pack_name(body), "bookw011");
    }
}

#[test]
fn the_book_bobs_around_head_height_and_repeats_each_period() {
    let heights: Vec<f32> = (0..BOOK_BOB_PERIOD_MS)
        .step_by(50)
        .map(|time| book_hover_offset(time)[1])
        .collect();
    let (lowest, highest) = heights
        .iter()
        .fold((f32::MAX, f32::MIN), |(low, high), h| (low.min(*h), high.max(*h)));
    assert!(highest - lowest > BOOK_HOVER_AMPLITUDE);
    assert!(highest - lowest <= 2.0 * BOOK_HOVER_AMPLITUDE + 1e-4);
    assert_eq!(book_hover_offset(70), book_hover_offset(70 + BOOK_BOB_PERIOD_MS));
}

#[test]
fn the_book_floats_beside_and_behind_the_body() {
    let [side, up, forward] = book_hover_offset(0);
    assert!(side > 0.0);
    assert!(up > 1.0);
    assert!(forward < 0.0);
}
