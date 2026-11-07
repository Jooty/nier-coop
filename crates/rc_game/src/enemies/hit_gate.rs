use crate::GameMemory;
use crate::offsets::{
    ACTOR_HIT_PROTECTION_SECONDS_OFFSET, ACTOR_KIND_MASK_OFFSET, ACTOR_KIND_MASK_PROTECTED_BIT,
    HIT_FLAG_IGNORES_ALL_PROTECTION, HIT_FLAG_IGNORES_HIT_PROTECTION_SECONDS,
    PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA,
};

pub fn victim_refuses_hit<M: GameMemory>(memory: &M, victim: u64, hit_flags: u32) -> bool {
    if hit_flags & HIT_FLAG_IGNORES_ALL_PROTECTION != 0 {
        return false;
    }
    let kind_mask = memory
        .read_i32(victim + ACTOR_KIND_MASK_OFFSET)
        .unwrap_or(0)
        .cast_unsigned();
    let protected_bits_active = memory
        .read_u8(memory.module_base() + u64::from(PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA))
        .is_ok_and(|byte| byte != 0);
    if kind_mask & ACTOR_KIND_MASK_PROTECTED_BIT != 0 && protected_bits_active {
        return true;
    }
    let protection_seconds = memory
        .read_f32(victim + ACTOR_HIT_PROTECTION_SECONDS_OFFSET)
        .unwrap_or(0.0);
    protection_seconds > 0.0 && hit_flags & HIT_FLAG_IGNORES_HIT_PROTECTION_SECONDS == 0
}
