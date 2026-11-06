use crate::game::{ActorModel, GameError, Position};
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_MODEL_RVA, ACTOR_MODEL_TEXT_LEN, ENTITY_GOLD_OFFSET, ENTITY_HEALTH_OFFSET,
    ENTITY_LEVEL_OFFSET, ENTITY_MAGIC_OFFSET, ENTITY_NAME_OFFSET, ENTITY_NAME_TEXT_CAPACITY_GUESS,
    ENTITY_PLAYTIME_OFFSET, ENTITY_RVA, ENTITY_TEXT_READ_LEN, ENTITY_ZONE_OFFSET,
    ENTITY_ZONE_TEXT_CAPACITY_GUESS, INFINITE_HEALTH_ORIGINAL_BYTES, INFINITE_HEALTH_PATCH_RVA,
    INFINITE_HEALTH_PATCHED_BYTES, INFINITE_MAGIC_ORIGINAL_BYTES, INFINITE_MAGIC_PATCH_RVA,
    INFINITE_MAGIC_PATCHED_BYTES, INVENTORY_ITEMS, PLAYABLE_ACTOR_POINTER_RVA,
    PLAYABLE_ACTOR_X_OFFSET, PLAYABLE_ACTOR_Y_OFFSET, PLAYABLE_ACTOR_Z_OFFSET,
};

#[derive(Debug)]
pub struct Replicant<M> {
    memory: M,
}

impl<M: GameMemory> Replicant<M> {
    pub fn new(memory: M) -> Self {
        Self { memory }
    }

    pub fn memory(&self) -> &M {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut M {
        &mut self.memory
    }

    pub fn gold(&self) -> Result<i32, GameError> {
        Ok(self
            .memory
            .read_i32(self.entity_field(ENTITY_GOLD_OFFSET))?)
    }

    pub fn set_gold(&mut self, value: i32) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_GOLD_OFFSET);
        Ok(self.memory.write_i32(address, value)?)
    }

    pub fn health(&self) -> Result<i32, GameError> {
        Ok(self
            .memory
            .read_i32(self.entity_field(ENTITY_HEALTH_OFFSET))?)
    }

    pub fn set_health(&mut self, value: i32) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_HEALTH_OFFSET);
        Ok(self.memory.write_i32(address, value)?)
    }

    pub fn magic(&self) -> Result<f32, GameError> {
        Ok(self
            .memory
            .read_f32(self.entity_field(ENTITY_MAGIC_OFFSET))?)
    }

    pub fn set_magic(&mut self, value: f32) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_MAGIC_OFFSET);
        Ok(self.memory.write_f32(address, value)?)
    }

    pub fn level(&self) -> Result<i32, GameError> {
        Ok(self
            .memory
            .read_i32(self.entity_field(ENTITY_LEVEL_OFFSET))?)
    }

    pub fn set_level(&mut self, value: i32) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_LEVEL_OFFSET);
        Ok(self.memory.write_i32(address, value)?)
    }

    pub fn playtime(&self) -> Result<f64, GameError> {
        Ok(self
            .memory
            .read_f64(self.entity_field(ENTITY_PLAYTIME_OFFSET))?)
    }

    pub fn set_playtime(&mut self, value: f64) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_PLAYTIME_OFFSET);
        Ok(self.memory.write_f64(address, value)?)
    }

    pub fn zone(&self) -> Result<String, GameError> {
        self.read_text(self.entity_field(ENTITY_ZONE_OFFSET))
    }

    pub fn set_zone(&mut self, value: &str) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_ZONE_OFFSET);
        self.write_text(address, value, ENTITY_ZONE_TEXT_CAPACITY_GUESS)
    }

    pub fn name(&self) -> Result<String, GameError> {
        self.read_text(self.entity_field(ENTITY_NAME_OFFSET))
    }

    pub fn set_name(&mut self, value: &str) -> Result<(), GameError> {
        let address = self.entity_field(ENTITY_NAME_OFFSET);
        self.write_text(address, value, ENTITY_NAME_TEXT_CAPACITY_GUESS)
    }

    pub fn position(&self) -> Result<Position, GameError> {
        let actor = self.playable_actor()?;
        Ok(Position {
            x: self.memory.read_f32(actor + PLAYABLE_ACTOR_X_OFFSET)?,
            y: self.memory.read_f32(actor + PLAYABLE_ACTOR_Y_OFFSET)?,
            z: self.memory.read_f32(actor + PLAYABLE_ACTOR_Z_OFFSET)?,
        })
    }

    pub fn set_position(&mut self, position: Position) -> Result<(), GameError> {
        let actor = self.playable_actor()?;
        self.memory
            .write_f32(actor + PLAYABLE_ACTOR_X_OFFSET, position.x)?;
        self.memory
            .write_f32(actor + PLAYABLE_ACTOR_Y_OFFSET, position.y)?;
        self.memory
            .write_f32(actor + PLAYABLE_ACTOR_Z_OFFSET, position.z)?;
        Ok(())
    }

    pub fn actor_model_name(&self) -> Result<String, GameError> {
        let address = self.module_address(ACTOR_MODEL_RVA);
        let bytes = self.memory.read_array::<ACTOR_MODEL_TEXT_LEN>(address)?;
        Ok(text_up_to_nul(&bytes))
    }

    pub fn set_actor_model(&mut self, model: ActorModel) -> Result<(), GameError> {
        let address = self.module_address(ACTOR_MODEL_RVA);
        Ok(self.memory.write(address, &model.bytes())?)
    }

    pub fn item_count(&self, name: &str) -> Result<u8, GameError> {
        Ok(self.memory.read_u8(self.item_address(name)?)?)
    }

    pub fn set_item_count(&mut self, name: &str, count: u8) -> Result<(), GameError> {
        let address = self.item_address(name)?;
        Ok(self.memory.write_u8(address, count)?)
    }

    pub fn remove_item(&mut self, name: &str) -> Result<(), GameError> {
        self.set_item_count(name, 0)
    }

    pub fn set_infinite_health(&mut self, enabled: bool) -> Result<(), GameError> {
        self.apply_patch(
            INFINITE_HEALTH_PATCH_RVA,
            INFINITE_HEALTH_ORIGINAL_BYTES,
            INFINITE_HEALTH_PATCHED_BYTES,
            enabled,
        )
    }

    pub fn set_infinite_magic(&mut self, enabled: bool) -> Result<(), GameError> {
        self.apply_patch(
            INFINITE_MAGIC_PATCH_RVA,
            INFINITE_MAGIC_ORIGINAL_BYTES,
            INFINITE_MAGIC_PATCHED_BYTES,
            enabled,
        )
    }

    pub(super) fn module_address(&self, rva: u32) -> u64 {
        self.memory.module_base() + u64::from(rva)
    }

    pub(super) fn entity_field(&self, offset: u32) -> u64 {
        self.module_address(ENTITY_RVA + offset)
    }

    fn playable_actor(&self) -> Result<u64, GameError> {
        match self
            .memory
            .read_u64(self.module_address(PLAYABLE_ACTOR_POINTER_RVA))?
        {
            0 => Err(GameError::PlayableActorNotSpawned),
            actor => Ok(actor),
        }
    }

    fn item_address(&self, name: &str) -> Result<u64, GameError> {
        let (_, rva) = INVENTORY_ITEMS
            .iter()
            .find(|(item_name, _)| *item_name == name)
            .ok_or_else(|| GameError::UnknownItem(name.to_owned()))?;
        Ok(self.module_address(*rva))
    }

    fn read_text(&self, address: u64) -> Result<String, GameError> {
        let bytes = self.memory.read_array::<ENTITY_TEXT_READ_LEN>(address)?;
        Ok(text_up_to_nul(&bytes))
    }

    fn write_text(&mut self, address: u64, text: &str, capacity: usize) -> Result<(), GameError> {
        let len = text.len() + 1;
        if len > capacity {
            return Err(GameError::TextTooLong { len, capacity });
        }
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        Ok(self.memory.write(address, &bytes)?)
    }

    fn apply_patch<const N: usize>(
        &mut self,
        rva: u32,
        original: [u8; N],
        patched: [u8; N],
        enabled: bool,
    ) -> Result<(), GameError> {
        let address = self.module_address(rva);
        let current = self.memory.read_array::<N>(address)?;
        if current != original && current != patched {
            return Err(GameError::UnexpectedBytes { rva });
        }
        let wanted = if enabled { patched } else { original };
        Ok(self.memory.write(address, &wanted)?)
    }

    pub(super) fn patch_is_applied<const N: usize>(
        &self,
        rva: u32,
        original: [u8; N],
        patched: [u8; N],
    ) -> Result<bool, GameError> {
        let current = self.memory.read_array::<N>(self.module_address(rva))?;
        if current == patched {
            return Ok(true);
        }
        if current == original {
            return Ok(false);
        }
        Err(GameError::UnexpectedBytes { rva })
    }
}

pub(super) fn text_up_to_nul(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
