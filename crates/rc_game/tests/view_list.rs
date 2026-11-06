use rc_game::marker::{
    FRAME_KEY_BYTES, FrameRing, VIEW_BUFFER_ENTRY_COUNT_OFFSET, VIEW_BUFFER_STRIDE,
    VIEW_ENTRY_STRIDE, VIEW_LIST_BUFFER_COUNT_OFFSET, VIEW_LIST_BUFFERS_OFFSET,
    VIEW_LIST_WRITE_INDEX_OFFSET, drawn_buffer_index, read_drawn_view_key,
};

use crate::fake_memory::FakeMemory;

const FAKE_VIEW_LIST_ADDRESS: u64 = 0x3_0000_0000;
const FAKE_VIEW_BUFFERS_ADDRESS: u64 = 0x3_0000_1000;
const FAKE_FIRST_ENTRIES_ADDRESS: u64 = 0x3_0001_0000;
const ENTRIES_PER_FAKE_BUFFER: i32 = 2;

fn key(tag: u8) -> [u8; FRAME_KEY_BYTES] {
    [tag; FRAME_KEY_BYTES]
}

fn entries_of(buffer: u64) -> u64 {
    FAKE_FIRST_ENTRIES_ADDRESS + buffer * 0x1000
}

fn two_buffer_list(write_index: i32, slot_zero_tags: [u8; 2]) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        FAKE_VIEW_LIST_ADDRESS + VIEW_LIST_BUFFERS_OFFSET,
        &FAKE_VIEW_BUFFERS_ADDRESS.to_le_bytes(),
    );
    memory.put(
        FAKE_VIEW_LIST_ADDRESS + VIEW_LIST_WRITE_INDEX_OFFSET,
        &write_index.to_le_bytes(),
    );
    memory.put(
        FAKE_VIEW_LIST_ADDRESS + VIEW_LIST_BUFFER_COUNT_OFFSET,
        &2_i32.to_le_bytes(),
    );
    for (buffer, tag) in (0_u64..).zip(slot_zero_tags) {
        let at = FAKE_VIEW_BUFFERS_ADDRESS + buffer * VIEW_BUFFER_STRIDE;
        memory.put(at, &entries_of(buffer).to_le_bytes());
        memory.put(
            at + VIEW_BUFFER_ENTRY_COUNT_OFFSET,
            &ENTRIES_PER_FAKE_BUFFER.to_le_bytes(),
        );
        memory.put(entries_of(buffer), &key(tag));
        memory.put(entries_of(buffer) + VIEW_ENTRY_STRIDE, &key(tag + 100));
    }
    memory
}

#[test]
fn the_drawn_buffer_is_the_one_before_the_buffer_being_written() {
    assert_eq!(drawn_buffer_index(1, 2), Some(0));
    assert_eq!(drawn_buffer_index(0, 2), Some(1));
    assert_eq!(drawn_buffer_index(2, 3), Some(1));
}

#[test]
fn a_single_buffer_list_draws_buffer_zero() {
    assert_eq!(drawn_buffer_index(0, 1), Some(0));
    assert_eq!(drawn_buffer_index(5, 1), Some(0));
}

#[test]
fn a_list_without_buffers_has_no_drawn_buffer() {
    assert_eq!(drawn_buffer_index(0, 0), None);
    assert_eq!(drawn_buffer_index(1, -1), None);
}

#[test]
fn the_drawn_key_comes_from_the_previous_buffer() {
    let memory = two_buffer_list(1, [7, 8]);
    assert_eq!(
        read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, 0),
        Ok(Some(key(7)))
    );
    let memory = two_buffer_list(0, [7, 8]);
    assert_eq!(
        read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, 0),
        Ok(Some(key(8)))
    );
}

#[test]
fn the_drawn_key_reads_the_requested_slot() {
    let memory = two_buffer_list(1, [7, 8]);
    assert_eq!(
        read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, 1),
        Ok(Some(key(107)))
    );
}

#[test]
fn a_slot_past_the_buffer_entries_has_no_key() {
    let memory = two_buffer_list(1, [7, 8]);
    assert_eq!(
        read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, ENTRIES_PER_FAKE_BUFFER),
        Ok(None)
    );
    assert_eq!(
        read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, -1),
        Ok(None)
    );
}

#[test]
fn the_drawn_frame_is_one_back_after_the_game_pushes_the_next_view() {
    let mut ring = FrameRing::new();
    ring.push(key(7), "frame on screen");
    ring.push(key(8), "frame being built");
    let memory = two_buffer_list(1, [7, 8]);
    let drawn = read_drawn_view_key(&memory, FAKE_VIEW_LIST_ADDRESS, 0)
        .ok()
        .flatten()
        .and_then(|drawn| ring.find_with_age(&drawn));
    assert_eq!(drawn, Some(("frame on screen", 1)));
    assert_eq!(ring.nth_back(0), Some("frame being built"));
}
