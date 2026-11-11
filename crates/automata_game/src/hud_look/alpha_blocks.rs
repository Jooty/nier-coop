use crate::hud_look::le_bytes::u32_at;
use crate::hud_look::{
    BLOCK_BYTES, BLOCK_EDGE_PIXELS, DDS_FOUR_CC_OF_BLOCKS_WITH_ALPHA, DDS_FOUR_CC_OFFSET,
    DDS_HEADER_BYTES, DDS_HEIGHT_OFFSET, DDS_MAGIC, DDS_WIDTH_OFFSET,
};

const ALPHA_INDEX_BITS: u32 = 3;

/// The alpha of a rectangle of a block-compressed texture, one byte a texel, rows top to bottom.
pub fn block_compressed_alpha(
    texture: &[u8],
    left: usize,
    top: usize,
    width: usize,
    height: usize,
) -> Option<Vec<u8>> {
    if !texture.starts_with(DDS_MAGIC)
        || texture.get(DDS_FOUR_CC_OFFSET..DDS_FOUR_CC_OFFSET + 4)?
            != DDS_FOUR_CC_OF_BLOCKS_WITH_ALPHA
    {
        return None;
    }
    let texture_width = u32_at(texture, DDS_WIDTH_OFFSET)? as usize;
    let texture_height = u32_at(texture, DDS_HEIGHT_OFFSET)? as usize;
    if left + width > texture_width || top + height > texture_height {
        return None;
    }
    let blocks_per_row = texture_width.div_ceil(BLOCK_EDGE_PIXELS);
    let mut alpha = vec![0_u8; width * height];
    for row in 0..height {
        for column in 0..width {
            let (x, y) = (left + column, top + row);
            let block_at = DDS_HEADER_BYTES
                + (y / BLOCK_EDGE_PIXELS * blocks_per_row + x / BLOCK_EDGE_PIXELS) * BLOCK_BYTES;
            let block = texture.get(block_at..block_at + BLOCK_BYTES)?;
            alpha[row * width + column] = alpha_of_texel(
                block,
                y % BLOCK_EDGE_PIXELS * BLOCK_EDGE_PIXELS + x % BLOCK_EDGE_PIXELS,
            );
        }
    }
    Some(alpha)
}

fn alpha_of_texel(block: &[u8], texel: usize) -> u8 {
    let (first, second) = (u32::from(block[0]), u32::from(block[1]));
    let indices = block[2..8]
        .iter()
        .rev()
        .fold(0_u64, |bits, byte| (bits << 8) | u64::from(*byte));
    let index = ((indices >> (texel as u32 * ALPHA_INDEX_BITS)) & 0b111) as u32;
    let blended = match index {
        0 => first,
        1 => second,
        _ if first > second => ((8 - index) * first + (index - 1) * second) / 7,
        6 => 0,
        7 => 255,
        _ => ((6 - index) * first + (index - 1) * second) / 5,
    };
    blended as u8
}
