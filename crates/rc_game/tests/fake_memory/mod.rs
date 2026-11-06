use std::collections::BTreeMap;

use rc_game::{GameMemory, MemoryError};

pub const FAKE_MODULE_BASE: u64 = 0x1_4000_0000;

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
