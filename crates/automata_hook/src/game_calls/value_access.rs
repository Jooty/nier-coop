use automata_game::address::GameAddress;
use nc_hook::memory::{GameMemory, InProcessMemory, MemoryError};

use crate::game_calls::GameCalls;

impl GameCalls {
    pub fn read_i32(&self, address: GameAddress) -> Result<i32, MemoryError> {
        InProcessMemory::new(&self.build).read_i32(self.address_of(address) as u64)
    }

    pub fn read_u64(&self, address: GameAddress) -> Result<u64, MemoryError> {
        InProcessMemory::new(&self.build).read_u64(self.address_of(address) as u64)
    }

    pub fn write_i32(&self, address: GameAddress, value: i32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write_i32(self.address_of(address) as u64, value)
    }

    pub fn write_u32(&self, address: GameAddress, value: u32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build)
            .write(self.address_of(address) as u64, &value.to_le_bytes())
    }

    pub fn read_bytes_at(&self, address: u64, len: usize) -> Result<Vec<u8>, MemoryError> {
        let mut bytes = vec![0_u8; len];
        InProcessMemory::new(&self.build).read(address, &mut bytes)?;
        Ok(bytes)
    }

    pub fn read_c_string_at(&self, address: u64, max_len: usize) -> String {
        let memory = InProcessMemory::new(&self.build);
        let mut bytes = Vec::new();
        for offset in 0..max_len {
            match memory.read_u8(address + offset as u64) {
                Ok(0) | Err(_) => break,
                Ok(byte) => bytes.push(byte),
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub fn address_value(&self, address: GameAddress) -> u64 {
        self.address_of(address) as u64
    }
}
