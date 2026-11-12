use automata_game::partner::{HeldBodies, MAX_HELD_BODIES};

const SLOT_BODY_HANDLE: u32 = 0x100;
const SLOT_BODY_PLAYER_SLOT: u8 = 1;

fn three_bodies() -> HeldBodies {
    let mut bodies = HeldBodies::default();
    assert!(bodies.add(SLOT_BODY_HANDLE, SLOT_BODY_PLAYER_SLOT, true));
    assert!(bodies.add(0x200, 2, false));
    assert!(bodies.add(0x300, 3, false));
    bodies
}

#[test]
fn a_guard_knows_every_held_body() {
    let bodies = three_bodies();
    for handle in [SLOT_BODY_HANDLE, 0x200, 0x300] {
        assert!(bodies.holds(handle));
    }
    assert!(!bodies.holds(0x400));
    assert_eq!(bodies.slot_of(0x300), Some(3));
    assert_eq!(bodies.slot_of(0x400), None);
}

#[test]
fn the_slot_body_is_told_apart() {
    let bodies = three_bodies();
    assert_eq!(bodies.slot_body(), Some(SLOT_BODY_HANDLE));
    let mut without = HeldBodies::default();
    without.add(0x200, 2, false);
    assert_eq!(without.slot_body(), None);
    assert!(without.holds(0x200));
}

#[test]
fn releasing_one_body_keeps_the_others() {
    let mut bodies = three_bodies();
    bodies.remove(0x200);
    assert!(!bodies.holds(0x200));
    assert!(bodies.holds(SLOT_BODY_HANDLE));
    assert!(bodies.holds(0x300));
    assert_eq!(bodies.slot_body(), Some(SLOT_BODY_HANDLE));
    bodies.clear();
    assert!(bodies.is_empty());
    assert_eq!(bodies.slot_body(), None);
}

#[test]
fn a_sixteenth_body_is_refused() {
    let mut bodies = HeldBodies::default();
    for index in 0..MAX_HELD_BODIES as u32 {
        assert!(bodies.add(0x1000 + index, index as u8, false));
    }
    assert_eq!(bodies.len(), 15);
    assert!(!bodies.add(0x2000, 15, false));
    assert!(!bodies.holds(0x2000));
    assert!(bodies.add(0x1000, 9, false));
    assert_eq!(bodies.slot_of(0x1000), Some(9));
}
