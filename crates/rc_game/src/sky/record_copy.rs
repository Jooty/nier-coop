use std::collections::BTreeMap;

use rc_proto::{MAX_SKY_RECORD_CHUNK_BYTES, SkyRecordChunk};

use crate::memory::{GameMemory, MemoryError};
use crate::offsets::{
    ASSET_NAME_SLOT_OFFSETS, ENVIRONMENT_MANAGER_RVA, ENVIRONMENT_RECORD_BYTES,
    ENVIRONMENT_TABLE_POINTER_OFFSET,
};

#[derive(Debug, Default)]
pub struct RecordAssembler {
    key: Option<(u16, i32)>,
    parts: BTreeMap<u16, Vec<u8>>,
}

impl RecordAssembler {
    pub fn receive(&mut self, chunk: &SkyRecordChunk) -> Option<Vec<u8>> {
        if self.key != Some((chunk.phase, chunk.area)) {
            self.key = Some((chunk.phase, chunk.area));
            self.parts.clear();
        }
        self.parts.insert(chunk.offset, chunk.bytes.clone());
        let mut record = Vec::with_capacity(ENVIRONMENT_RECORD_BYTES);
        for (offset, bytes) in &self.parts {
            if usize::from(*offset) != record.len() {
                return None;
            }
            record.extend_from_slice(bytes);
        }
        (record.len() == ENVIRONMENT_RECORD_BYTES).then_some(record)
    }
}

pub fn split_record(phase: u16, area: i32, record: &[u8]) -> Vec<SkyRecordChunk> {
    record
        .chunks(MAX_SKY_RECORD_CHUNK_BYTES)
        .enumerate()
        .map(|(index, bytes)| SkyRecordChunk {
            phase,
            area,
            offset: u16::try_from(index * MAX_SKY_RECORD_CHUNK_BYTES).unwrap_or(u16::MAX),
            bytes: bytes.to_vec(),
        })
        .collect()
}

pub fn merge_record(host: &[u8], local: &[u8]) -> Vec<u8> {
    let mut merged = host.to_vec();
    for offset in ASSET_NAME_SLOT_OFFSETS {
        if let (Some(kept), Some(target)) = (
            local.get(offset..offset + 8),
            merged.get_mut(offset..offset + 8),
        ) {
            target.copy_from_slice(kept);
        }
    }
    merged
}

pub(crate) fn record_address<M: GameMemory>(memory: &M, area: i32) -> Result<u64, MemoryError> {
    let environment = memory.module_base() + u64::from(ENVIRONMENT_MANAGER_RVA);
    let table = memory.read_u64(environment + ENVIRONMENT_TABLE_POINTER_OFFSET)?;
    let area = u64::try_from(area).map_err(|_| MemoryError::Unreadable { address: table })?;
    if table == 0 {
        return Err(MemoryError::Unreadable { address: 0 });
    }
    Ok(table + area * ENVIRONMENT_RECORD_BYTES as u64)
}

pub fn read_table_record<M: GameMemory>(memory: &M, area: i32) -> Result<Vec<u8>, MemoryError> {
    let address = record_address(memory, area)?;
    Ok(memory
        .read_array::<ENVIRONMENT_RECORD_BYTES>(address)?
        .to_vec())
}

pub fn write_table_record<M: GameMemory>(
    memory: &mut M,
    area: i32,
    record: &[u8],
) -> Result<(), MemoryError> {
    let address = record_address(memory, area)?;
    memory.write(address, record)
}
