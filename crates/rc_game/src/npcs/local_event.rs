use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{
    HUMAN_NPC_STATE_MACHINE_OFFSET, NPC_EVENT_CALLBACK_STATE_VTABLE_RVA,
    NPC_EVENT_TALK_STATE_VTABLE_RVA, NPC_SCRIPT_STATE_VTABLE_RVA,
    STATE_MACHINE_CURRENT_STATE_OFFSET,
};

/// A state in which this game's own scripts drive the NPC: a conversation with this machine's
/// player, an event callback or a script. The game swaps the state in at once when the talk or
/// script starts, so it is visible before the NPC's next AI step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcEvent {
    Talk,
    Callback,
    Script,
}

pub fn read_npc_event<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Option<NpcEvent>, MemoryError> {
    let machine = memory.read_u64(actor + HUMAN_NPC_STATE_MACHINE_OFFSET)?;
    if machine == 0 {
        return Ok(None);
    }
    let state = memory.read_u64(machine + STATE_MACHINE_CURRENT_STATE_OFFSET)?;
    if state == 0 {
        return Ok(None);
    }
    let vtable = memory.read_u64(state)?;
    let base = memory.module_base();
    let event = [
        (NPC_EVENT_TALK_STATE_VTABLE_RVA, NpcEvent::Talk),
        (NPC_EVENT_CALLBACK_STATE_VTABLE_RVA, NpcEvent::Callback),
        (NPC_SCRIPT_STATE_VTABLE_RVA, NpcEvent::Script),
    ]
    .into_iter()
    .find(|(rva, _)| base + u64::from(*rva) == vtable)
    .map(|(_, event)| event);
    Ok(event)
}
