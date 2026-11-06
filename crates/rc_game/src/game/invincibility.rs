use crate::game::{GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{
    ENTITY_HEALTH_OFFSET, LOCAL_PLAYER_SLOT_INDEX, PLAYER_PARAM_SLOT_INDEX_OFFSET,
};

impl<M: GameMemory> Replicant<M> {
    pub fn health_write_hurts_local_player(
        &self,
        player_param: u64,
        requested_health: i32,
    ) -> Result<bool, GameError> {
        let slot = self
            .memory()
            .read_i32(player_param + PLAYER_PARAM_SLOT_INDEX_OFFSET)?;
        if slot != LOCAL_PLAYER_SLOT_INDEX {
            return Ok(false);
        }
        let current = self
            .memory()
            .read_i32(self.entity_field(ENTITY_HEALTH_OFFSET))?;
        Ok(current > 0 && requested_health.max(0) < current)
    }
}
