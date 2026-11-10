use replicant_proto::WeaponState;

use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_DRAW_MASK_ALL_ON, ACTOR_DRAW_MASK_OFFSET, ACTOR_DRAWN_FLAG_OFFSET, ENTITY_RVA,
    FORCED_WEAPON_ID, MAX_WEAPON_LEVEL_STORED, PLAYER_WEAPON_ALLOWED_FLAG_OFFSET,
    PLAYER_WEAPON_IN_HAND_FLAG_OFFSET, SAVE_EQUIPPED_WEAPON_ID_OFFSET,
    SAVE_FORCED_WEAPON_FLAG_OFFSET, SAVE_WEAPON_LEVELS_OFFSET, WEAPON_RECORD_GRIP_IN_HAND_OFFSET,
    WEAPON_RECORD_GRIP_ON_BACK_OFFSET, WEAPON_TABLE_LEN, WEAPON_TABLE_RVA,
};

/// The local player's weapon as the partner should see it: the equipped id (the forced one when
/// the save says so), its upgrade level, whether it is in hand, and whether the player shows it.
pub fn read_weapon_look<M: GameMemory>(
    memory: &M,
    player: u64,
) -> Result<Option<WeaponState>, GameError> {
    let save = memory.module_base() + u64::from(ENTITY_RVA);
    let id = if memory.read_i32(save + SAVE_FORCED_WEAPON_FLAG_OFFSET)? == 1 {
        FORCED_WEAPON_ID
    } else {
        memory.read_i32(save + SAVE_EQUIPPED_WEAPON_ID_OFFSET)?
    };
    let Some(id) = u8::try_from(id)
        .ok()
        .filter(|id| i32::from(*id) < WEAPON_TABLE_LEN)
    else {
        return Ok(None);
    };
    let id_offset = u64::from(id);
    let level = memory
        .read_u8(save + SAVE_WEAPON_LEVELS_OFFSET + id_offset)?
        .cast_signed()
        .clamp(0, MAX_WEAPON_LEVEL_STORED);
    let drawn = memory.read_u8(player + ACTOR_DRAWN_FLAG_OFFSET)? != 0;
    let mask_on = memory.read_u8(player + ACTOR_DRAW_MASK_OFFSET)? & ACTOR_DRAW_MASK_ALL_ON
        == ACTOR_DRAW_MASK_ALL_ON;
    let allowed = memory.read_u8(player + PLAYER_WEAPON_ALLOWED_FLAG_OFFSET)? != 0;
    Ok(Some(WeaponState {
        id,
        level: level.cast_unsigned(),
        in_hand: memory.read_u8(player + PLAYER_WEAPON_IN_HAND_FLAG_OFFSET)? != 0,
        visible: drawn && mask_on && allowed,
    }))
}

/// Where the weapon's grip sits in its attach frame, from the weapon table record.
pub fn read_weapon_grip<M: GameMemory>(
    memory: &M,
    id: u8,
    in_hand: bool,
) -> Result<Option<[f32; 3]>, GameError> {
    if i32::from(id) >= WEAPON_TABLE_LEN {
        return Ok(None);
    }
    let table = memory.module_base() + u64::from(WEAPON_TABLE_RVA);
    let record = memory.read_u64(table + u64::from(id) * 8)?;
    if record == 0 {
        return Ok(None);
    }
    let grip = record
        + if in_hand {
            WEAPON_RECORD_GRIP_IN_HAND_OFFSET
        } else {
            WEAPON_RECORD_GRIP_ON_BACK_OFFSET
        };
    Ok(Some([
        memory.read_f32(grip)?,
        memory.read_f32(grip + 4)?,
        memory.read_f32(grip + 8)?,
    ]))
}
