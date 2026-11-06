use rc_proto::{ENDING_ACCOMPLISHED_BYTES, ENDING_COSTUME_BYTES, EndingRecords};

use crate::story_progress::{
    ENDING_ACCOMPLISHED_HEADER_OFFSET, ENDING_CLEAR_BITS_HEADER_OFFSET,
    ENDING_COSTUMES_HEADER_OFFSET, FINISH_HOUR_ACCOMPLISHED_INDEX, HEADER_BYTES,
    HEADER_CHECKSUM_OFFSET,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderTooShort;

pub fn read_ending_records(header: &[u8]) -> Option<EndingRecords> {
    let clear_bits = u32::from_le_bytes(
        header
            .get(ENDING_CLEAR_BITS_HEADER_OFFSET..ENDING_CLEAR_BITS_HEADER_OFFSET + 4)?
            .try_into()
            .ok()?,
    );
    let costumes = header
        .get(ENDING_COSTUMES_HEADER_OFFSET..ENDING_COSTUMES_HEADER_OFFSET + ENDING_COSTUME_BYTES)?
        .try_into()
        .ok()?;
    let mut accomplished = header
        .get(
            ENDING_ACCOMPLISHED_HEADER_OFFSET
                ..ENDING_ACCOMPLISHED_HEADER_OFFSET + ENDING_ACCOMPLISHED_BYTES,
        )?
        .to_vec();
    accomplished[FINISH_HOUR_ACCOMPLISHED_INDEX] = 0;
    Some(EndingRecords {
        clear_bits,
        costumes,
        accomplished,
    })
}

pub fn merge_ending_records(first: &EndingRecords, second: &EndingRecords) -> EndingRecords {
    EndingRecords {
        clear_bits: first.clear_bits | second.clear_bits,
        costumes: combine_arrays(&first.costumes, &second.costumes, or_byte),
        accomplished: combine_slices(&first.accomplished, &second.accomplished, or_byte),
    }
}

pub fn ending_records_gained(known: &EndingRecords, now: &EndingRecords) -> EndingRecords {
    EndingRecords {
        clear_bits: now.clear_bits & !known.clear_bits,
        costumes: combine_arrays(&now.costumes, &known.costumes, and_not_byte),
        accomplished: combine_slices(&now.accomplished, &known.accomplished, and_not_byte),
    }
}

pub fn ending_records_are_empty(records: &EndingRecords) -> bool {
    records.clear_bits == 0
        && records.costumes.iter().all(|&byte| byte == 0)
        && records.accomplished.iter().all(|&byte| byte == 0)
}

pub fn header_checksum(header: &[u8]) -> Option<u32> {
    let covered = header.get(..HEADER_CHECKSUM_OFFSET)?;
    Some(covered.iter().map(|&byte| u32::from(byte)).sum())
}

pub fn fix_header_checksum(file: &mut [u8]) -> Result<(), HeaderTooShort> {
    let sum = header_checksum(file).ok_or(HeaderTooShort)?;
    file.get_mut(HEADER_CHECKSUM_OFFSET..HEADER_BYTES)
        .ok_or(HeaderTooShort)?
        .copy_from_slice(&sum.to_le_bytes());
    Ok(())
}

pub fn apply_header_records(
    file: &mut [u8],
    records: &EndingRecords,
) -> Result<bool, HeaderTooShort> {
    if file.len() < HEADER_BYTES || records.accomplished.len() != ENDING_ACCOMPLISHED_BYTES {
        return Err(HeaderTooShort);
    }
    let before = file[..HEADER_BYTES].to_vec();
    file[ENDING_CLEAR_BITS_HEADER_OFFSET..ENDING_CLEAR_BITS_HEADER_OFFSET + 4]
        .copy_from_slice(&records.clear_bits.to_le_bytes());
    file[ENDING_COSTUMES_HEADER_OFFSET..ENDING_COSTUMES_HEADER_OFFSET + ENDING_COSTUME_BYTES]
        .copy_from_slice(&records.costumes);
    for (index, &value) in records.accomplished.iter().enumerate() {
        if index != FINISH_HOUR_ACCOMPLISHED_INDEX {
            file[ENDING_ACCOMPLISHED_HEADER_OFFSET + index] = value;
        }
    }
    fix_header_checksum(file)?;
    Ok(file[..HEADER_BYTES] != before[..])
}

fn combine_arrays<const N: usize>(
    first: &[u8; N],
    second: &[u8; N],
    combine: fn(u8, u8) -> u8,
) -> [u8; N] {
    std::array::from_fn(|index| combine(first[index], second[index]))
}

fn combine_slices(first: &[u8], second: &[u8], combine: fn(u8, u8) -> u8) -> Vec<u8> {
    (0..first.len().max(second.len()))
        .map(|index| {
            combine(
                first.get(index).copied().unwrap_or(0),
                second.get(index).copied().unwrap_or(0),
            )
        })
        .collect()
}

fn or_byte(first: u8, second: u8) -> u8 {
    first | second
}

fn and_not_byte(now: u8, known: u8) -> u8 {
    now & !known
}
