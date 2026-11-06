use crate::game::{GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{
    INFINITE_MAGIC_ORIGINAL_BYTES, INFINITE_MAGIC_PATCH_RVA, INFINITE_MAGIC_PATCHED_BYTES,
};

impl<M: GameMemory> Replicant<M> {
    pub fn infinite_magic_enabled(&self) -> Result<bool, GameError> {
        self.patch_is_applied(
            INFINITE_MAGIC_PATCH_RVA,
            INFINITE_MAGIC_ORIGINAL_BYTES,
            INFINITE_MAGIC_PATCHED_BYTES,
        )
    }
}
