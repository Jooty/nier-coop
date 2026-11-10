use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_DRAW_MASK_OFFSET, ACTOR_DRAWN_FLAG_OFFSET,
    ACTOR_LIFE_STATE_OFFSET, ACTOR_MODEL_PACK_OFFSET, ACTOR_SCRIPT_GROUP_OFFSET,
    PLAYABLE_ACTOR_X_OFFSET, PLAYABLE_ACTOR_Y_OFFSET, PLAYABLE_ACTOR_Z_OFFSET,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorDrawReport {
    pub position: [f32; 3],
    pub drawn_flag: u8,
    pub draw_mask: u8,
    pub class_id: i32,
    pub script_group: i32,
    pub life_state: i32,
    pub model_pack: u64,
}

impl ActorDrawReport {
    pub fn summary(&self) -> String {
        format!(
            "at ({:.1}, {:.1}, {:.1}), drawn flag {}, draw mask {:#x}, class {:#x}, group {}, life state {}, model pack {:#x}",
            self.position[0],
            self.position[1],
            self.position[2],
            self.drawn_flag,
            self.draw_mask,
            self.class_id,
            self.script_group,
            self.life_state,
            self.model_pack
        )
    }
}

pub fn read_actor_draw_report<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<ActorDrawReport, GameError> {
    Ok(ActorDrawReport {
        position: [
            memory.read_f32(actor + PLAYABLE_ACTOR_X_OFFSET)?,
            memory.read_f32(actor + PLAYABLE_ACTOR_Y_OFFSET)?,
            memory.read_f32(actor + PLAYABLE_ACTOR_Z_OFFSET)?,
        ],
        drawn_flag: memory.read_u8(actor + ACTOR_DRAWN_FLAG_OFFSET)?,
        draw_mask: memory.read_u8(actor + ACTOR_DRAW_MASK_OFFSET)?,
        class_id: memory.read_i32(actor + ACTOR_CLASS_ID_OFFSET)?,
        script_group: memory.read_i32(actor + ACTOR_SCRIPT_GROUP_OFFSET)?,
        life_state: memory.read_i32(actor + ACTOR_LIFE_STATE_OFFSET)?,
        model_pack: memory.read_u64(actor + ACTOR_MODEL_PACK_OFFSET)?,
    })
}

pub fn distance_between(a: [f32; 3], b: [f32; 3]) -> f32 {
    let [dx, dy, dz] = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
