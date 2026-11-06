use crate::party_frame::{AtlasRect, BC3_BLOCK_BYTES, BC3_BLOCK_PIXELS};

pub fn decode_bc3_region(data: &[u8], atlas_width: u32, rect: AtlasRect) -> Option<Vec<u8>> {
    let block = BC3_BLOCK_PIXELS;
    if !atlas_width.is_multiple_of(block)
        || rect.right <= rect.left
        || rect.bottom <= rect.top
        || rect.right > atlas_width
    {
        return None;
    }
    let blocks_per_row = (atlas_width / block) as usize;
    let out_width = rect.width() as usize;
    let mut out = vec![0u8; out_width * rect.height() as usize * 4];
    // The rect need not sit on the 4x4 grid: decode every block it touches, keep its pixels.
    for block_y in (rect.top / block)..rect.bottom.div_ceil(block) {
        for block_x in (rect.left / block)..rect.right.div_ceil(block) {
            let start = (block_y as usize * blocks_per_row + block_x as usize) * BC3_BLOCK_BYTES;
            let encoded = data.get(start..start + BC3_BLOCK_BYTES)?;
            let pixels = decode_bc3_block(encoded);
            for (row, row_pixels) in pixels.iter().enumerate() {
                for (column, pixel) in row_pixels.iter().enumerate() {
                    let x = block_x * block + column as u32;
                    let y = block_y * block + row as u32;
                    if x < rect.left || x >= rect.right || y < rect.top || y >= rect.bottom {
                        continue;
                    }
                    let at = ((y - rect.top) as usize * out_width + (x - rect.left) as usize) * 4;
                    out[at..at + 4].copy_from_slice(pixel);
                }
            }
        }
    }
    Some(out)
}

fn decode_bc3_block(encoded: &[u8]) -> [[[u8; 4]; 4]; 4] {
    let alpha_palette = alpha_palette(encoded[0], encoded[1]);
    let alpha_bits = u64::from_le_bytes([
        encoded[2], encoded[3], encoded[4], encoded[5], encoded[6], encoded[7], 0, 0,
    ]);
    let color0 = u16::from_le_bytes([encoded[8], encoded[9]]);
    let color1 = u16::from_le_bytes([encoded[10], encoded[11]]);
    let colors = color_palette(color0, color1);
    let color_bits = u32::from_le_bytes([encoded[12], encoded[13], encoded[14], encoded[15]]);
    let mut pixels = [[[0u8; 4]; 4]; 4];
    for (row, row_pixels) in pixels.iter_mut().enumerate() {
        for (column, pixel) in row_pixels.iter_mut().enumerate() {
            let index = row * 4 + column;
            let color = colors[((color_bits >> (index * 2)) & 3) as usize];
            let alpha = alpha_palette[((alpha_bits >> (index * 3)) & 7) as usize];
            *pixel = [color[0], color[1], color[2], alpha];
        }
    }
    pixels
}

fn alpha_palette(alpha0: u8, alpha1: u8) -> [u8; 8] {
    let a0 = u32::from(alpha0);
    let a1 = u32::from(alpha1);
    let mix =
        |weight0: u32, weight1: u32, divisor: u32| ((a0 * weight0 + a1 * weight1) / divisor) as u8;
    if alpha0 > alpha1 {
        [
            alpha0,
            alpha1,
            mix(6, 1, 7),
            mix(5, 2, 7),
            mix(4, 3, 7),
            mix(3, 4, 7),
            mix(2, 5, 7),
            mix(1, 6, 7),
        ]
    } else {
        [
            alpha0,
            alpha1,
            mix(4, 1, 5),
            mix(3, 2, 5),
            mix(2, 3, 5),
            mix(1, 4, 5),
            0,
            255,
        ]
    }
}

fn color_palette(color0: u16, color1: u16) -> [[u8; 3]; 4] {
    let first = expand_565(color0);
    let second = expand_565(color1);
    let mix = |weight0: u32, weight1: u32| {
        let mut out = [0u8; 3];
        for channel in 0..3 {
            out[channel] = ((u32::from(first[channel]) * weight0
                + u32::from(second[channel]) * weight1)
                / 3) as u8;
        }
        out
    };
    [first, second, mix(2, 1), mix(1, 2)]
}

fn expand_565(color: u16) -> [u8; 3] {
    let red = u32::from(color >> 11) & 31;
    let green = u32::from(color >> 5) & 63;
    let blue = u32::from(color) & 31;
    [
        ((red * 255 + 15) / 31) as u8,
        ((green * 255 + 31) / 63) as u8,
        ((blue * 255 + 15) / 31) as u8,
    ]
}
