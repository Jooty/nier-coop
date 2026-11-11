use crate::hud_look::TEXTURE_LIST_MAGIC;
use crate::hud_look::le_bytes::u32_at;

const TEXTURE_COUNT_OFFSET: usize = 8;
const OFFSETS_TABLE_OFFSET: usize = 12;
const SIZES_TABLE_OFFSET: usize = 16;
const LARGEST_TEXTURE_COUNT: usize = 256;

/// Where one texture sits inside the texture data file that goes with a texture list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSpan {
    pub offset: usize,
    pub size: usize,
}

pub fn texture_spans(texture_list: &[u8]) -> Option<Vec<TextureSpan>> {
    if !texture_list.starts_with(TEXTURE_LIST_MAGIC) {
        return None;
    }
    let count = u32_at(texture_list, TEXTURE_COUNT_OFFSET)? as usize;
    if count > LARGEST_TEXTURE_COUNT {
        return None;
    }
    let offsets = u32_at(texture_list, OFFSETS_TABLE_OFFSET)? as usize;
    let sizes = u32_at(texture_list, SIZES_TABLE_OFFSET)? as usize;
    (0..count)
        .map(|index| {
            Some(TextureSpan {
                offset: u32_at(texture_list, offsets + index * 4)? as usize,
                size: u32_at(texture_list, sizes + index * 4)? as usize,
            })
        })
        .collect()
}
