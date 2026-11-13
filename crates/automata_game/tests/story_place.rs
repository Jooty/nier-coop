use automata_game::load_gate::{PlaceRelation, StoryPlace};

#[test]
fn equal_named_places_are_the_same() {
    let here = StoryPlace::new(0x100, "00_30_A_Place");
    assert_eq!(here.relation_to(&here.clone()), PlaceRelation::Same);
}

#[test]
fn a_different_phase_or_sub_phase_is_another_place() {
    let here = StoryPlace::new(0x100, "00_30_A_Place");
    assert_eq!(
        here.relation_to(&StoryPlace::new(0x200, "00_30_A_Place")),
        PlaceRelation::Other
    );
    assert_eq!(
        here.relation_to(&StoryPlace::new(0x100, "00_40_A_Place")),
        PlaceRelation::Other
    );
}

#[test]
fn an_empty_name_on_either_side_is_unknown() {
    let here = StoryPlace::new(0x100, "00_30_A_Place");
    let nameless = StoryPlace::new(0x100, "");
    assert_eq!(here.relation_to(&nameless), PlaceRelation::Unknown);
    assert_eq!(nameless.relation_to(&here), PlaceRelation::Unknown);
    assert_eq!(nameless.relation_to(&nameless), PlaceRelation::Unknown);
    assert_eq!(
        here.relation_to(&StoryPlace::default()),
        PlaceRelation::Unknown
    );
}

#[test]
fn a_long_name_is_cut_to_thirty_one_bytes() {
    let cut = StoryPlace::new(0x100, &"a".repeat(40));
    assert_eq!(cut.sub_phase.len(), 31);
}

#[test]
fn a_place_is_described_by_its_id_and_name() {
    assert_eq!(StoryPlace::new(0x100, "n").describe(), "0x100 n");
    assert_eq!(StoryPlace::new(0xF00, "").describe(), "0xF00 (no name)");
}
