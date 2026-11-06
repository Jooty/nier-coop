use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{ASSET_NAME_READ_LEN, ASSET_NAME_SLOT_OFFSETS};
use crate::sky::record_address;

pub fn read_cstring<M: GameMemory>(memory: &M, address: u64) -> String {
    let bytes = memory
        .read_array::<ASSET_NAME_READ_LEN>(address)
        .unwrap_or([0; ASSET_NAME_READ_LEN]);
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
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
