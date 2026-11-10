use replicant_game::enemies::{pack_hit_kind, unpack_hit_kind};

#[test]
fn kind_and_class_survive_the_wire_word() {
    assert_eq!(unpack_hit_kind(pack_hit_kind(7, 3)), (7, Some(3)));
}

#[test]
fn a_hit_without_a_class_unpacks_to_none() {
    assert_eq!(unpack_hit_kind(6), (6, None));
}

#[test]
fn a_negative_kind_keeps_its_sign_and_the_class() {
    assert_eq!(unpack_hit_kind(pack_hit_kind(-1, 3)), (-1, Some(3)));
}
