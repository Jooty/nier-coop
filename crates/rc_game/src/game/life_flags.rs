use crate::game::{GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{PLAYER_PARAM_ARRAY_RVA, PLAYER_PARAM_LIFE_FLAGS_OFFSET};

impl<M: GameMemory> Replicant<M> {
    pub fn life_flag(&self, bit: u8) -> Result<bool, GameError> {
        Ok(self.memory().read_u8(self.life_flags_address())? & bit != 0)
    }

    pub fn set_life_flag(&mut self, bit: u8, on: bool) -> Result<(), GameError> {
        let address = self.life_flags_address();
        let current = self.memory().read_u8(address)?;
        let wanted = if on { current | bit } else { current & !bit };
        if wanted != current {
            self.memory_mut().write_u8(address, wanted)?;
        }
        Ok(())
    }

    fn life_flags_address(&self) -> u64 {
        self.module_address(PLAYER_PARAM_ARRAY_RVA + PLAYER_PARAM_LIFE_FLAGS_OFFSET)
    }
}
