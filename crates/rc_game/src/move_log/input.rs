use crate::memory::GameMemory;
use crate::move_log::{
    ACTOR_INPUT_BLOCK_GETTER_SLOT_OFFSET, CODE_SECTION_END_RVA, INPUT_BLOCK_MOVE_AXIS_X_OFFSET,
    INPUT_BLOCK_MOVE_AXIS_Y_OFFSET, KEY_BIT_COUNT, KEY_BIT_TABLE_RVA, KEY_SET_OFFSET,
    KEYBOARD_DEVICE_POINTER_RVA, MOVE_KEY_VIRTUAL_KEYS,
};

pub fn input_block_getter<M: GameMemory>(memory: &M, actor: u64) -> Option<u64> {
    let vftable = memory.read_u64(actor).ok()?;
    let getter = memory
        .read_u64(vftable + ACTOR_INPUT_BLOCK_GETTER_SLOT_OFFSET)
        .ok()?;
    let base = memory.module_base();
    (getter > base && getter < base + u64::from(CODE_SECTION_END_RVA)).then_some(getter)
}

pub fn read_move_axis<M: GameMemory>(memory: &M, input_block: u64) -> Option<[f32; 2]> {
    Some([
        memory
            .read_f32(input_block + INPUT_BLOCK_MOVE_AXIS_X_OFFSET)
            .ok()?,
        memory
            .read_f32(input_block + INPUT_BLOCK_MOVE_AXIS_Y_OFFSET)
            .ok()?,
    ])
}

pub fn read_move_keys<M: GameMemory>(memory: &M) -> Option<[bool; 4]> {
    let base = memory.module_base();
    let device = memory
        .read_u64(base + u64::from(KEYBOARD_DEVICE_POINTER_RVA))
        .ok()
        .filter(|device| *device != 0)?;
    let mut keys = [false; 4];
    for (down, virtual_key) in keys.iter_mut().zip(MOVE_KEY_VIRTUAL_KEYS) {
        let bit = u64::try_from(
            memory
                .read_i32(base + u64::from(KEY_BIT_TABLE_RVA) + u64::from(virtual_key) * 4)
                .ok()?,
        )
        .ok()
        .filter(|bit| *bit < KEY_BIT_COUNT)?;
        let word = memory
            .read_u64(device + KEY_SET_OFFSET + (bit / 64) * 8)
            .ok()?;
        *down = word >> (bit % 64) & 1 == 1;
    }
    Some(keys)
}
