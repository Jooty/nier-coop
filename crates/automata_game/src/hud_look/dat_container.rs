use crate::hud_look::DAT_MAGIC;
use crate::hud_look::le_bytes::u32_at;

const ENTRY_COUNT_OFFSET: usize = 4;
const OFFSETS_TABLE_OFFSET: usize = 8;
const NAMES_TABLE_OFFSET: usize = 16;
const SIZES_TABLE_OFFSET: usize = 20;
const LARGEST_ENTRY_COUNT: usize = 4096;

/// One file of a game container, by the name the container gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatEntry<'a> {
    pub name: String,
    pub bytes: &'a [u8],
}

/// Every file of a game container; `None` when the bytes are not a whole container.
pub fn dat_entries(container: &[u8]) -> Option<Vec<DatEntry<'_>>> {
    if !container.starts_with(DAT_MAGIC) {
        return None;
    }
    let count = u32_at(container, ENTRY_COUNT_OFFSET)? as usize;
    if count > LARGEST_ENTRY_COUNT {
        return None;
    }
    let offsets = u32_at(container, OFFSETS_TABLE_OFFSET)? as usize;
    let names = u32_at(container, NAMES_TABLE_OFFSET)? as usize;
    let sizes = u32_at(container, SIZES_TABLE_OFFSET)? as usize;
    let name_bytes = u32_at(container, names)? as usize;
    (0..count)
        .map(|index| {
            let start = u32_at(container, offsets + index * 4)? as usize;
            let size = u32_at(container, sizes + index * 4)? as usize;
            let name_start = names + 4 + index * name_bytes;
            let name = container.get(name_start..name_start + name_bytes)?;
            let name_end = name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name_bytes);
            Some(DatEntry {
                name: String::from_utf8_lossy(&name[..name_end]).into_owned(),
                bytes: container.get(start..start.checked_add(size)?)?,
            })
        })
        .collect()
}

pub fn dat_entry_with_extension<'a>(container: &'a [u8], extension: &str) -> Option<&'a [u8]> {
    dat_entries(container)?
        .into_iter()
        .find(|entry| entry.name.ends_with(extension))
        .map(|entry| entry.bytes)
}
