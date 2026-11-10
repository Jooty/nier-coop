use crate::memory::GameMemory;
use crate::offsets::{
    PARTNER_KAINE_ACTOR_RVA, PARTNER_THIRD_MEMBER_ACTOR_RVA, PARTNER_VARIANT_POINTER_RVA,
};

pub fn partner_actors<M: GameMemory>(memory: &M) -> Vec<u64> {
    let base = memory.module_base();
    let variant = memory
        .read_u64(base + u64::from(PARTNER_VARIANT_POINTER_RVA))
        .unwrap_or(0);
    [
        base + u64::from(PARTNER_KAINE_ACTOR_RVA),
        base + u64::from(PARTNER_THIRD_MEMBER_ACTOR_RVA),
        variant,
    ]
    .into_iter()
    .filter(|actor| *actor != 0)
    .collect()
}
