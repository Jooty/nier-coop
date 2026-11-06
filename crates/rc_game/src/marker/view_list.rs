use crate::marker::{
    FRAME_KEY_BYTES, FrameKey, VIEW_BUFFER_ENTRY_COUNT_OFFSET, VIEW_BUFFER_STRIDE,
    VIEW_ENTRY_STRIDE, VIEW_LIST_BUFFER_COUNT_OFFSET, VIEW_LIST_BUFFERS_OFFSET,
    VIEW_LIST_WRITE_INDEX_OFFSET,
};
use crate::memory::{GameMemory, MemoryError};

/// The view-list buffer the render thread draws while the game thread fills `write_index`:
/// the game's own `(index - 1) mod count`, or buffer 0 when the list is single-buffered.
pub fn drawn_buffer_index(write_index: i32, buffer_count: i32) -> Option<i32> {
    if buffer_count <= 0 {
        return None;
    }
    if buffer_count == 1 {
        return Some(0);
    }
    Some((write_index - 1).rem_euclid(buffer_count))
}

/// The 48 view bytes of `slot` in the buffer the presented image was drawn from.
pub fn read_drawn_view_key<M: GameMemory>(
    memory: &M,
    list: u64,
    slot: i32,
) -> Result<Option<FrameKey>, MemoryError> {
    let write_index = memory.read_i32(list + VIEW_LIST_WRITE_INDEX_OFFSET)?;
    let buffer_count = memory.read_i32(list + VIEW_LIST_BUFFER_COUNT_OFFSET)?;
    let Some(drawn) = drawn_buffer_index(write_index, buffer_count) else {
        return Ok(None);
    };
    let buffers = memory.read_u64(list + VIEW_LIST_BUFFERS_OFFSET)?;
    if buffers == 0 || slot < 0 {
        return Ok(None);
    }
    let buffer = buffers + drawn as u64 * VIEW_BUFFER_STRIDE;
    let entries = memory.read_u64(buffer)?;
    let entry_count = memory.read_i32(buffer + VIEW_BUFFER_ENTRY_COUNT_OFFSET)?;
    if entries == 0 || slot >= entry_count {
        return Ok(None);
    }
    let entry = entries + slot as u64 * VIEW_ENTRY_STRIDE;
    memory.read_array::<FRAME_KEY_BYTES>(entry).map(Some)
}
