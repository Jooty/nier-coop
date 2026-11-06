use crate::game::{GameError, Replicant, text_up_to_nul};
use crate::memory::GameMemory;
use crate::offsets::ENTITY_NAME_OFFSET;
use crate::player_name::NAME_FIELD_LEN;

impl<M: GameMemory> Replicant<M> {
    pub fn save_name_field(&self) -> Result<[u8; NAME_FIELD_LEN], GameError> {
        let address = self.entity_field(ENTITY_NAME_OFFSET);
        Ok(self.memory().read_array::<NAME_FIELD_LEN>(address)?)
    }

    pub fn save_name(&self) -> Result<String, GameError> {
        Ok(text_up_to_nul(&self.save_name_field()?))
    }

    pub fn set_save_name_field(&mut self, field: &[u8; NAME_FIELD_LEN]) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_NAME_OFFSET);
        Ok(self.memory_mut().write(address, field)?)
    }
}
