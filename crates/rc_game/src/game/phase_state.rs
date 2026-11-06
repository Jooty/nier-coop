use crate::game::replicant::text_up_to_nul;
use crate::game::{GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{
    ENTITY_ZONE_OFFSET, MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET, MAP_MANAGER_RVA,
    PHASE_NAME_BUFFER_LEN,
};

impl<M: GameMemory> Replicant<M> {
    pub fn current_phase(&self) -> Result<String, GameError> {
        let bytes = self
            .memory()
            .read_array::<PHASE_NAME_BUFFER_LEN>(self.entity_field(ENTITY_ZONE_OFFSET))?;
        Ok(text_up_to_nul(&bytes))
    }

    pub fn map_manager_address(&self) -> u64 {
        self.module_address(MAP_MANAGER_RVA)
    }

    pub fn map_manager_is_active(&self) -> Result<bool, GameError> {
        let pointer_address =
            self.map_manager_address() + MAP_MANAGER_ACTIVE_CONTEXT_POINTER_OFFSET;
        Ok(self.memory().read_u64(pointer_address)? != 0)
    }
}
