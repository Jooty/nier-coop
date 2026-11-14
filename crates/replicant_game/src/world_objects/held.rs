use crate::memory::GameMemory;
use crate::offsets::PLAYER_ACTION_STATE_OFFSET;
use crate::world_objects::{
    ACTION_STATE_TARGET_HANDLE_OFFSET, BOX_MOVE_STATE_VTABLE_RVA, PICK_MOVE_STATE_VTABLE_RVA,
    PLAYER_REQUESTED_ACTION_STATE_OFFSET,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldHow {
    Push,
    Carry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldTarget {
    pub handle: u64,
    pub how: HoldHow,
}

/// What the player's action state holds: a pushed or pulled box, or a lifted object, named by
/// the self handle the game copied into the state when the grab started. The state the game runs
/// is read first; a grab it was only just asked for still sits in the requested slot.
pub fn read_held_target<M: GameMemory>(memory: &M, player: u64) -> Option<HeldTarget> {
    if player == 0 {
        return None;
    }
    [
        PLAYER_ACTION_STATE_OFFSET,
        PLAYER_REQUESTED_ACTION_STATE_OFFSET,
    ]
    .into_iter()
    .find_map(|slot| held_target_in_slot(memory, player + slot))
}

fn held_target_in_slot<M: GameMemory>(memory: &M, slot: u64) -> Option<HeldTarget> {
    let state = memory.read_u64(slot).ok()?;
    if state == 0 {
        return None;
    }
    let vtable = memory.read_u64(state).ok()?;
    let base = memory.module_base();
    let how = if vtable == base + u64::from(BOX_MOVE_STATE_VTABLE_RVA) {
        HoldHow::Push
    } else if vtable == base + u64::from(PICK_MOVE_STATE_VTABLE_RVA) {
        HoldHow::Carry
    } else {
        return None;
    };
    let handle = memory
        .read_u64(state + ACTION_STATE_TARGET_HANDLE_OFFSET)
        .ok()?;
    (handle != 0).then_some(HeldTarget { handle, how })
}
