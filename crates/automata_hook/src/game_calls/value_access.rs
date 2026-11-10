use automata_game::address::GameAddress;
use nc_hook::memory::{GameMemory, InProcessMemory, MemoryError};

use crate::game_calls::GameCalls;

impl GameCalls {
    pub fn read_i32(&self, address: GameAddress) -> Result<i32, MemoryError> {
        InProcessMemory::new(&self.build).read_i32(self.address_of(address) as u64)
    }

    pub fn read_u32(&self, address: GameAddress) -> Result<u32, MemoryError> {
        self.read_u32_at(self.address_of(address) as u64)
    }

    pub fn read_u64(&self, address: GameAddress) -> Result<u64, MemoryError> {
        InProcessMemory::new(&self.build).read_u64(self.address_of(address) as u64)
    }

    pub fn write_i32(&self, address: GameAddress, value: i32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write_i32(self.address_of(address) as u64, value)
    }

    pub fn write_u8(&self, address: GameAddress, value: u8) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write_u8(self.address_of(address) as u64, value)
    }

    pub fn read_bytes(&self, address: GameAddress, len: usize) -> Result<Vec<u8>, MemoryError> {
        self.read_bytes_at(self.address_of(address) as u64, len)
    }

    pub fn write_u32(&self, address: GameAddress, value: u32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build)
            .write(self.address_of(address) as u64, &value.to_le_bytes())
    }

    pub fn read_u32_at(&self, address: u64) -> Result<u32, MemoryError> {
        Ok(u32::from_le_bytes(
            InProcessMemory::new(&self.build).read_array(address)?,
        ))
    }

    pub fn read_i32_at(&self, address: u64) -> Result<i32, MemoryError> {
        InProcessMemory::new(&self.build).read_i32(address)
    }

    pub fn read_u64_at(&self, address: u64) -> Result<u64, MemoryError> {
        InProcessMemory::new(&self.build).read_u64(address)
    }

    pub fn read_f32_at(&self, address: u64) -> Result<f32, MemoryError> {
        InProcessMemory::new(&self.build).read_f32(address)
    }

    pub fn read_f32s_at<const N: usize>(&self, address: u64) -> Result<[f32; N], MemoryError> {
        let bytes = self.read_bytes_at(address, N * 4)?;
        Ok(core::array::from_fn(|index| {
            let start = index * 4;
            f32::from_le_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ])
        }))
    }

    pub fn read_u32s_at<const N: usize>(&self, address: u64) -> Result<[u32; N], MemoryError> {
        let bytes = self.read_bytes_at(address, N * 4)?;
        Ok(core::array::from_fn(|index| {
            let start = index * 4;
            u32::from_le_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ])
        }))
    }

    pub fn write_u32_at(&self, address: u64, value: u32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write(address, &value.to_le_bytes())
    }

    pub fn write_i32_at(&self, address: u64, value: i32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write_i32(address, value)
    }

    pub fn write_f32_at(&self, address: u64, value: f32) -> Result<(), MemoryError> {
        InProcessMemory::new(&self.build).write_f32(address, value)
    }

    pub fn write_f32s_at(&self, address: u64, values: &[f32]) -> Result<(), MemoryError> {
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        InProcessMemory::new(&self.build).write(address, &bytes)
    }

    pub fn write_u32s_at(&self, address: u64, values: &[u32]) -> Result<(), MemoryError> {
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        InProcessMemory::new(&self.build).write(address, &bytes)
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

    /// One read of the whole limit; byte by byte only when that read crosses into unreadable memory.
    pub fn read_bounded_c_string_at(&self, address: u64, max_len: usize) -> String {
        let Ok(bytes) = self.read_bytes_at(address, max_len) else {
            return self.read_c_string_at(address, max_len);
        };
        let len = bytes.iter().position(|byte| *byte == 0).unwrap_or(max_len);
        String::from_utf8_lossy(&bytes[..len]).into_owned()
    }

    pub fn address_value(&self, address: GameAddress) -> u64 {
        self.address_of(address) as u64
    }
}
