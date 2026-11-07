use crate::enemies::ActorHandle;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_TRANSFORM_BLOCK_OFFSET, ACTOR_TRANSFORM_ROW_STRIDE,
    BLACK_DRAIN_LASER_OWNER_HANDLE_OFFSET, BLACK_DRAIN_LASER_VTABLE_RVA,
    BLACK_G_LANCE_OWNER_HANDLE_OFFSET, BLACK_G_LANCE_VTABLE_RVA, BLACK_HAND_OWNER_HANDLE_OFFSET,
    BLACK_HAND_VTABLE_RVA, BLACK_LANCE_OWNER_HANDLE_OFFSET, BLACK_LANCE_VTABLE_RVA,
    BLACK_ROUND_BLADE_OWNER_HANDLE_OFFSET, BLACK_ROUND_BLADE_VTABLE_RVA,
    BLACK_WALL_OWNER_HANDLE_OFFSET, BLACK_WALL_VTABLE_RVA, BLACK_Z_SLASH_OWNER_HANDLE_OFFSET,
    BLACK_Z_SLASH_VTABLE_RVA, BOOK_ACTOR_POINTER_RVA, NORMAL_BULLET_OWNER_HANDLE_OFFSET,
    NORMAL_BULLET_VTABLE_RVA,
};

const TRANSLATION_LANE_BYTES: u64 = 12;

pub const POWER_ACTOR_CLASSES: [PowerActorClass; 7] = [
    PowerActorClass {
        name: "hand",
        vtable_rva: BLACK_HAND_VTABLE_RVA,
        owner_handle_offset: BLACK_HAND_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "z slash",
        vtable_rva: BLACK_Z_SLASH_VTABLE_RVA,
        owner_handle_offset: BLACK_Z_SLASH_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "wall",
        vtable_rva: BLACK_WALL_VTABLE_RVA,
        owner_handle_offset: BLACK_WALL_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "drain laser",
        vtable_rva: BLACK_DRAIN_LASER_VTABLE_RVA,
        owner_handle_offset: BLACK_DRAIN_LASER_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "lance",
        vtable_rva: BLACK_LANCE_VTABLE_RVA,
        owner_handle_offset: BLACK_LANCE_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "g lance",
        vtable_rva: BLACK_G_LANCE_VTABLE_RVA,
        owner_handle_offset: BLACK_G_LANCE_OWNER_HANDLE_OFFSET,
    },
    PowerActorClass {
        name: "round blade",
        vtable_rva: BLACK_ROUND_BLADE_VTABLE_RVA,
        owner_handle_offset: BLACK_ROUND_BLADE_OWNER_HANDLE_OFFSET,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerActorClass {
    pub name: &'static str,
    pub vtable_rva: u32,
    pub owner_handle_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectSource {
    Player,
    Book,
    BulletOfPlayer,
    BulletOfOther,
    PowerOfPlayer,
    PowerOfOther,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundRoute {
    Puppet,
    Book,
}

impl EffectSource {
    /// Where the partner plays a sound from this source; None when it is not sent. A power or
    /// bullet has no actor on the partner's side, so the book stands in for it.
    pub fn sound_route(self) -> Option<SoundRoute> {
        match self {
            EffectSource::Player => Some(SoundRoute::Puppet),
            EffectSource::Book | EffectSource::PowerOfPlayer | EffectSource::BulletOfPlayer => {
                Some(SoundRoute::Book)
            }
            EffectSource::BulletOfOther | EffectSource::PowerOfOther | EffectSource::Other => None,
        }
    }
}

pub fn classify_effect_source<M: GameMemory>(
    memory: &M,
    actor: u64,
    player: u64,
) -> Result<EffectSource, GameError> {
    if actor == 0 {
        return Ok(EffectSource::Other);
    }
    if actor == player {
        return Ok(EffectSource::Player);
    }
    let base = memory.module_base();
    let book = memory.read_u64(base + u64::from(BOOK_ACTOR_POINTER_RVA))?;
    if book != 0 && actor == book {
        return Ok(EffectSource::Book);
    }
    if let Some(class) = power_actor_class(memory, actor)? {
        return Ok(match power_actor_owner(memory, actor, class) {
            Some(owner) if owner == player => EffectSource::PowerOfPlayer,
            _ => EffectSource::PowerOfOther,
        });
    }
    if !is_normal_bullet(memory, actor)? {
        return Ok(EffectSource::Other);
    }
    Ok(match bullet_owner(memory, actor) {
        Some(owner) if owner == player => EffectSource::BulletOfPlayer,
        _ => EffectSource::BulletOfOther,
    })
}

pub fn is_normal_bullet<M: GameMemory>(memory: &M, actor: u64) -> Result<bool, GameError> {
    let vtable = memory.read_u64(actor)?;
    Ok(vtable == memory.module_base() + u64::from(NORMAL_BULLET_VTABLE_RVA))
}

pub fn power_actor_class<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Option<&'static PowerActorClass>, GameError> {
    let vtable = memory.read_u64(actor)?;
    let base = memory.module_base();
    Ok(POWER_ACTOR_CLASSES
        .iter()
        .find(|class| vtable == base + u64::from(class.vtable_rva)))
}

pub fn power_actor_owner<M: GameMemory>(
    memory: &M,
    actor: u64,
    class: &PowerActorClass,
) -> Option<u64> {
    ActorHandle::read(memory, actor + class.owner_handle_offset)?.resolve(memory)
}

pub fn bullet_owner<M: GameMemory>(memory: &M, bullet: u64) -> Option<u64> {
    if !is_normal_bullet(memory, bullet).ok()? {
        return None;
    }
    ActorHandle::read(memory, bullet + NORMAL_BULLET_OWNER_HANDLE_OFFSET)?.resolve(memory)
}

pub fn read_actor_position<M: GameMemory>(memory: &M, actor: u64) -> Result<[f32; 3], GameError> {
    let first_row = actor + ACTOR_TRANSFORM_BLOCK_OFFSET + TRANSLATION_LANE_BYTES;
    let mut position = [0.0_f32; 3];
    for (row, axis) in (0_u64..).zip(position.iter_mut()) {
        *axis = memory.read_f32(first_row + row * ACTOR_TRANSFORM_ROW_STRIDE as u64)?;
    }
    Ok(position)
}
