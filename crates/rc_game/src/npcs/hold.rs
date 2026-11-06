use crate::marker::{CONTROL_MASK_FIRST_RVA, EVENT_SCENE_BYTE_RVA};
use crate::memory::{GameMemory, MemoryError};
use crate::offsets::TALK_CONTROL_MASK_SLOT;

/// Why the guest lets its own NPCs run for now: an event scene or a conversation drives them on
/// this machine, and holding them would stall it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcHoldBlocker {
    EventScene,
    Talk,
}

pub fn read_npc_hold_blocker<M: GameMemory>(
    memory: &M,
) -> Result<Option<NpcHoldBlocker>, MemoryError> {
    let base = memory.module_base();
    if memory.read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))? != 0 {
        return Ok(Some(NpcHoldBlocker::EventScene));
    }
    let talk_slot =
        base + u64::from(CONTROL_MASK_FIRST_RVA) + u64::from(TALK_CONTROL_MASK_SLOT) * 4;
    if memory.read_i32(talk_slot)? != 0 {
        return Ok(Some(NpcHoldBlocker::Talk));
    }
    Ok(None)
}
