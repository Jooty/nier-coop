use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{ASSET_NAME_READ_LEN, ASSET_NAME_SLOT_OFFSETS};
use crate::sky::record_address;

fn read_cstring<M: GameMemory>(memory: &M, address: u64) -> String {
    let mut bytes = Vec::new();
    for index in 0..ASSET_NAME_READ_LEN as u64 {
        match memory.read_u8(address + index) {
            Ok(0) | Err(_) => break,
            Ok(byte) => bytes.push(byte),
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn read_asset_names<M: GameMemory>(memory: &M, area: i32) -> Result<Vec<String>, MemoryError> {
    let record = record_address(memory, area)?;
    ASSET_NAME_SLOT_OFFSETS
        .iter()
        .map(|offset| {
            let pointer = memory.read_u64(record + *offset as u64)?;
            Ok(if pointer == 0 {
                String::new()
            } else {
                read_cstring(memory, pointer)
            })
        })
        .collect()
}

pub fn write_asset_pointers<M: GameMemory>(
    memory: &mut M,
    area: i32,
    pointers: &[u64],
) -> Result<(), MemoryError> {
    let record = record_address(memory, area)?;
    for (offset, pointer) in ASSET_NAME_SLOT_OFFSETS.iter().zip(pointers) {
        memory.write(record + *offset as u64, &pointer.to_le_bytes())?;
    }
    Ok(())
}
