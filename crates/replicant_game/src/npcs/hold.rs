use crate::marker::EVENT_SCENE_BYTE_RVA;
use crate::memory::{GameMemory, MemoryError};

/// Why the guest lets every NPC run on its own for now: an event scene drives them on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcHoldBlocker {
    EventScene,
}

pub fn read_npc_hold_blocker<M: GameMemory>(
    memory: &M,
) -> Result<Option<NpcHoldBlocker>, MemoryError> {
    let base = memory.module_base();
    if memory.read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))? != 0 {
        return Ok(Some(NpcHoldBlocker::EventScene));
    }
    Ok(None)
}
