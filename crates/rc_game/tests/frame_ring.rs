use rc_game::marker::{FRAME_KEY_BYTES, FRAME_RING_SLOTS, FrameRing};

fn key(tag: u8) -> [u8; FRAME_KEY_BYTES] {
    [tag; FRAME_KEY_BYTES]
}

#[test]
fn finds_the_value_stored_under_a_key() {
    let mut ring = FrameRing::new();
    ring.push(key(1), 10_u32);
    ring.push(key(2), 20_u32);
    assert_eq!(ring.find(&key(1)), Some(10));
    assert_eq!(ring.find(&key(2)), Some(20));
    assert_eq!(ring.find(&key(3)), None);
}

#[test]
fn the_previous_frame_is_still_found_after_the_next_is_pushed() {
    let mut ring = FrameRing::new();
    ring.push(key(1), 1_u32);
    ring.push(key(2), 2_u32);
    assert_eq!(ring.find(&key(1)), Some(1));
}

#[test]
fn the_newest_value_wins_for_a_repeated_key() {
    let mut ring = FrameRing::new();
    ring.push(key(7), 1_u32);
    ring.push(key(7), 2_u32);
    assert_eq!(ring.find(&key(7)), Some(2));
}

#[test]
fn the_oldest_entry_is_dropped_when_the_ring_is_full() {
    let mut ring = FrameRing::new();
    for tag in 0..=FRAME_RING_SLOTS as u8 {
        ring.push(key(tag), u32::from(tag));
    }
    assert_eq!(ring.find(&key(0)), None);
    assert_eq!(ring.find(&key(1)), Some(1));
    assert_eq!(
        ring.find(&key(FRAME_RING_SLOTS as u8)),
        Some(FRAME_RING_SLOTS as u32)
    );
}

#[test]
fn nth_back_counts_from_the_newest_and_age_is_reported() {
    let mut ring = FrameRing::new();
    ring.push(key(1), 1_u32);
    ring.push(key(2), 2_u32);
    ring.push(key(3), 3_u32);
    assert_eq!(ring.nth_back(0), Some(3));
    assert_eq!(ring.nth_back(2), Some(1));
    assert_eq!(ring.nth_back(3), None);
    assert_eq!(ring.find_with_age(&key(2)), Some((2, 1)));
}
