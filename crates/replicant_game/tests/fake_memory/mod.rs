use std::collections::BTreeMap;

use replicant_game::ui_scenes::{
    SHORT_SYSTEM_MENU_OPEN_FLAG_OFFSET, SHORT_SYSTEM_MENU_SCENE_ID, UI_LINK_NEXT_OFFSET,
    UI_MANAGER_FIRST_SCENE_LINK_OFFSET, UI_MANAGER_RVA, UI_MANAGER_SCENE_LIST_OFFSET,
    UI_SCENE_FIRST_SEQUENCE_LINK_OFFSET, UI_SCENE_ID_OFFSET,
};
use replicant_game::{GameMemory, MemoryError};

pub const FAKE_MODULE_BASE: u64 = 0x1_4000_0000;
pub const FAKE_UI_SCENE_HEAP: u64 = 0x2_0000_0000;
pub const FAKE_UI_SCENE_STRIDE: u64 = 0x1000;

#[derive(Debug, Default)]
pub struct FakeMemory {
    bytes: BTreeMap<u64, u8>,
}

impl FakeMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, address: u64, bytes: &[u8]) {
        for (offset, byte) in (0_u64..).zip(bytes) {
            self.bytes.insert(address + offset, *byte);
        }
    }

    pub fn put_at_rva(&mut self, rva: u32, bytes: &[u8]) {
        self.put(FAKE_MODULE_BASE + u64::from(rva), bytes);
    }

    pub fn get(&self, address: u64, len: usize) -> Vec<u8> {
        (0_u64..len as u64)
            .map(|offset| self.bytes[&(address + offset)])
            .collect()
    }

    pub fn put_ui_scenes(&mut self, scene_ids: &[u8], short_system_menu_open: bool) {
        let manager = FAKE_MODULE_BASE + u64::from(UI_MANAGER_RVA);
        let ring_end = manager + UI_MANAGER_SCENE_LIST_OFFSET;
        let scenes: Vec<u64> = (0..scene_ids.len() as u64)
            .map(|index| FAKE_UI_SCENE_HEAP + index * FAKE_UI_SCENE_STRIDE)
            .collect();
        self.put(ring_end, &ring_end.to_le_bytes());
        let first = scenes.first().copied().unwrap_or(ring_end);
        self.put(
            manager + UI_MANAGER_FIRST_SCENE_LINK_OFFSET,
            &first.to_le_bytes(),
        );
        for (index, (scene, scene_id)) in scenes.iter().zip(scene_ids).enumerate() {
            let next = scenes.get(index + 1).copied().unwrap_or(ring_end);
            self.put(*scene, &scene.to_le_bytes());
            self.put(scene + UI_LINK_NEXT_OFFSET, &next.to_le_bytes());
            self.put(scene + UI_SCENE_ID_OFFSET, &[*scene_id]);
            if *scene_id == SHORT_SYSTEM_MENU_SCENE_ID {
                let sequence = scene + FAKE_UI_SCENE_STRIDE / 2;
                self.put(
                    scene + UI_SCENE_FIRST_SEQUENCE_LINK_OFFSET,
                    &(sequence + 8).to_le_bytes(),
                );
                self.put(sequence + 8, &sequence.to_le_bytes());
                self.put(
                    sequence + SHORT_SYSTEM_MENU_OPEN_FLAG_OFFSET,
                    &[u8::from(short_system_menu_open)],
                );
            }
        }
    }

    pub fn get_at_rva(&self, rva: u32, len: usize) -> Vec<u8> {
        self.get(FAKE_MODULE_BASE + u64::from(rva), len)
    }
}

impl GameMemory for FakeMemory {
    fn module_base(&self) -> u64 {
        FAKE_MODULE_BASE
    }

    fn read(&self, address: u64, buffer: &mut [u8]) -> Result<(), MemoryError> {
        for (offset, slot) in (0_u64..).zip(buffer.iter_mut()) {
            *slot = *self
                .bytes
                .get(&(address + offset))
                .ok_or(MemoryError::Unreadable { address })?;
        }
        Ok(())
    }

    fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        for offset in 0_u64..bytes.len() as u64 {
            if !self.bytes.contains_key(&(address + offset)) {
                return Err(MemoryError::Unwritable { address });
            }
        }
        self.put(address, bytes);
        Ok(())
    }
}
