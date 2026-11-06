use crate::memory::GameMemory;
use crate::offsets::{
    ATTACK_RECORD_ATTACK_KIND_OFFSET, ATTACK_RECORD_ATTACKER_HANDLE_OFFSET,
    ATTACK_RECORD_FLAGS_OFFSET, ATTACK_RECORD_HIT_ID_OFFSET, ATTACK_RECORD_HP_DAMAGE_OFFSET,
    ATTACK_RECORD_OWNER_HANDLE_OFFSET, ATTACK_RECORD_SECOND_GAUGE_OFFSET,
    ATTACK_RECORD_SOURCE_CLASS_OFFSET, ATTACK_RECORD_THIRD_GAUGE_OFFSET,
    HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT,
};

/// The fields of one attack record (the queue record `DamageMan` hands to a victim) that co-op
/// sends or checks. Damage numbers are the attacker's, before the victim's defence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttackRecord {
    pub attacker: ActorHandle,
    pub owner: ActorHandle,
    pub source_class: i32,
    pub hit_id: u32,
    pub hp_damage: i32,
    pub second_gauge: i32,
    pub third_gauge: i32,
    pub flags: u32,
    pub attack_kind: i32,
}

/// An actor handle as the game stores it: a slot index in the handle pool and that slot's
/// generation when the handle was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActorHandle {
    pub index: u32,
    pub generation: u32,
}

impl AttackRecord {
    pub fn read<M: GameMemory>(memory: &M, record: u64) -> Option<Self> {
        let int = |offset: u64| memory.read_i32(record + offset).ok();
        Some(Self {
            attacker: ActorHandle::read(memory, record + ATTACK_RECORD_ATTACKER_HANDLE_OFFSET)?,
            owner: ActorHandle::read(memory, record + ATTACK_RECORD_OWNER_HANDLE_OFFSET)?,
            source_class: int(ATTACK_RECORD_SOURCE_CLASS_OFFSET)?,
            hit_id: int(ATTACK_RECORD_HIT_ID_OFFSET)?.cast_unsigned(),
            hp_damage: int(ATTACK_RECORD_HP_DAMAGE_OFFSET)?,
            second_gauge: int(ATTACK_RECORD_SECOND_GAUGE_OFFSET)?,
            third_gauge: int(ATTACK_RECORD_THIRD_GAUGE_OFFSET)?,
            flags: int(ATTACK_RECORD_FLAGS_OFFSET)?.cast_unsigned(),
            attack_kind: int(ATTACK_RECORD_ATTACK_KIND_OFFSET)?,
        })
    }

    /// The actors behind the record that are still alive: the attacker (a body, a bullet or an
    /// effect) and the owner of a bullet or effect.
    pub fn sources<M: GameMemory>(&self, memory: &M) -> [Option<u64>; 2] {
        [self.attacker.resolve(memory), self.owner.resolve(memory)]
    }
}

impl ActorHandle {
    pub fn read<M: GameMemory>(memory: &M, address: u64) -> Option<Self> {
        Some(Self {
            index: memory.read_i32(address).ok()?.cast_unsigned(),
            generation: memory.read_i32(address + 4).ok()?.cast_unsigned(),
        })
    }

    /// The actor this handle names, or `None` when the slot is out of range, empty or reused.
    pub fn resolve<M: GameMemory>(&self, memory: &M) -> Option<u64> {
        if self.index >= HANDLE_POOL_SLOT_COUNT {
            return None;
        }
        let base = memory.module_base();
        let slot = u64::from(self.index);
        let actor = memory
            .read_u64(base + u64::from(HANDLE_POOL_POINTERS_RVA) + slot * 8)
            .ok()?;
        let generation = memory
            .read_i32(base + u64::from(HANDLE_POOL_GENERATIONS_RVA) + slot * 4)
            .ok()?;
        (actor != 0 && generation.cast_unsigned() == self.generation).then_some(actor)
    }
}
