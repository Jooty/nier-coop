use rc_proto::MAX_DIALOGUE_LINE_INDEX;

use crate::dialogue::{
    TALK_TABLE_SCAN_CHUNK_SLOTS, TALK_TABLE_SLOTS, TALK_WINDOW_CURRENT_LINE_OFFSET,
    TALK_WINDOW_CURRENT_TALK_OFFSET, TALK_WINDOW_RVA, TALK_WINDOW_TALK_TABLE_OFFSET,
};
use crate::memory::{GameMemory, MemoryError};

/// Where the open talk window is: the address of the talk it plays and the line on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TalkCursor {
    pub talk_address: u64,
    pub line: u16,
}

pub fn read_talk_cursor<M: GameMemory>(memory: &M) -> Result<Option<TalkCursor>, MemoryError> {
    let window = talk_window(memory);
    if memory.read_i32(window)? == 0 {
        return Ok(None);
    }
    let talk_address = memory.read_u64(window + TALK_WINDOW_CURRENT_TALK_OFFSET)?;
    let line = memory.read_i32(window + TALK_WINDOW_CURRENT_LINE_OFFSET)?;
    let Ok(line) = u16::try_from(line) else {
        return Ok(None);
    };
    if talk_address == 0 || line > MAX_DIALOGUE_LINE_INDEX {
        return Ok(None);
    }
    Ok(Some(TalkCursor { talk_address, line }))
}

/// The talk id whose slot in the window's talk table holds `talk_address`. The window keeps
/// only the address, so this scans the table; call it once per talk, not every frame.
pub fn find_talk_id<M: GameMemory>(
    memory: &M,
    talk_address: u64,
) -> Result<Option<u32>, MemoryError> {
    let table = talk_window(memory) + TALK_WINDOW_TALK_TABLE_OFFSET;
    let mut chunk = vec![0_u8; TALK_TABLE_SCAN_CHUNK_SLOTS as usize * 8];
    for first in (0..TALK_TABLE_SLOTS).step_by(TALK_TABLE_SCAN_CHUNK_SLOTS as usize) {
        memory.read(table + u64::from(first) * 8, &mut chunk)?;
        let found = (first..)
            .zip(chunk.as_chunks::<8>().0)
            .find(|(_, slot)| u64::from_le_bytes(**slot) == talk_address);
        if let Some((talk_id, _)) = found {
            return Ok(Some(talk_id));
        }
    }
    Ok(None)
}

pub(super) fn talk_window<M: GameMemory>(memory: &M) -> u64 {
    memory.module_base() + u64::from(TALK_WINDOW_RVA)
}
