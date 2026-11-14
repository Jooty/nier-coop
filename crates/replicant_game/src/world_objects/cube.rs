use crate::memory::GameMemory;
use crate::world_objects::CUBE_HITTABLE_FLAG_OFFSET;

pub fn cube_is_hittable<M: GameMemory>(memory: &M, actor: u64) -> bool {
    memory
        .read_u8(actor + CUBE_HITTABLE_FLAG_OFFSET)
        .is_ok_and(|flag| flag != 0)
}
