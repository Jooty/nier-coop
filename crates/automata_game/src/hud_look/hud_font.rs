use std::collections::HashMap;

use crate::hud_look::{
    FONT_TABLE_EXTENSION, FontTable, GlyphCell, HUD_FONT_ATLAS_GAP_PIXELS, HUD_FONT_ATLAS_WIDTH,
    HUD_FONT_FLAT_CAPITALS, HUD_FONT_INK_ALPHA, HUD_FONT_LAST_LATIN_CODE, HUD_FONT_MISSING_GLYPH,
    HUD_FONT_PUNCTUATION_CODES, KERNING_TABLE_EXTENSION, TEXTURE_DATA_EXTENSION,
    TEXTURE_LIST_EXTENSION, block_compressed_alpha, dat_entry_with_extension, parse_font_table,
    parse_kerning_table, texture_spans,
};

const FIRST_PRINTED_CODE: u16 = 0x20;
const RGBA_BYTES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackedGlyph {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
}

/// One glyph to draw: a screen rectangle and the part of the font texture that goes in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

/// The Latin part of the game's HUD font, packed into one white texture with the glyphs in alpha.
#[derive(Debug, Clone, PartialEq)]
pub struct HudFont {
    pub coverage_rgba: Vec<u8>,
    pub texture_width: u32,
    pub texture_height: u32,
    pub cell_height: f32,
    /// Where capital letters start and end inside a cell, in cell texels from the cell's top.
    pub ink_top: f32,
    pub ink_bottom: f32,
    advance_adjust: f32,
    glyphs: HashMap<char, PackedGlyph>,
    kerning: HashMap<(u16, u16), i16>,
}

impl HudFont {
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn text_width(&self, text: &str, scale: f32) -> f32 {
        let mut right = 0.0_f32;
        self.walk(text, |pen, glyph| right = pen + glyph.width as f32);
        right * scale
    }

    pub fn layout_text(&self, text: &str, origin: [f32; 2], scale: f32) -> Vec<GlyphQuad> {
        let (width, height) = (self.texture_width as f32, self.texture_height as f32);
        let mut quads = Vec::new();
        self.walk(text, |pen, glyph| {
            let min = [origin[0] + pen * scale, origin[1]];
            quads.push(GlyphQuad {
                min,
                max: [
                    min[0] + glyph.width as f32 * scale,
                    min[1] + glyph.height as f32 * scale,
                ],
                uv_min: [glyph.left as f32 / width, glyph.top as f32 / height],
                uv_max: [
                    (glyph.left + glyph.width) as f32 / width,
                    (glyph.top + glyph.height) as f32 / height,
                ],
            });
        });
        quads
    }

    fn walk(&self, text: &str, mut place: impl FnMut(f32, &PackedGlyph)) {
        let mut pen = 0.0_f32;
        let mut previous: Option<char> = None;
        for wanted in text.chars() {
            let Some((shown, glyph)) = self.glyph_for(wanted) else {
                continue;
            };
            if let Some(previous) = previous {
                pen += self.pair_adjust(previous, shown);
            }
            place(pen, glyph);
            // A space keeps its whole cell: the adjust is the overlap of two inked cells.
            pen += if shown == ' ' {
                glyph.width as f32
            } else {
                (glyph.width as f32 + self.advance_adjust).max(1.0)
            };
            previous = Some(shown);
        }
    }

    fn glyph_for(&self, wanted: char) -> Option<(char, &PackedGlyph)> {
        self.glyphs
            .get(&wanted)
            .map(|glyph| (wanted, glyph))
            .or_else(|| {
                self.glyphs
                    .get(&HUD_FONT_MISSING_GLYPH)
                    .map(|glyph| (HUD_FONT_MISSING_GLYPH, glyph))
            })
    }

    fn pair_adjust(&self, left: char, right: char) -> f32 {
        let (Ok(left), Ok(right)) = (u16::try_from(left as u32), u16::try_from(right as u32))
        else {
            return 0.0;
        };
        self.kerning
            .get(&(left, right))
            .map_or(0.0, |adjust| f32::from(*adjust))
    }
}

/// Builds the font from the game's two font containers (the table one, already unpacked, and the
/// texture one); the error says which part of them did not read.
pub fn build_hud_font(
    table_container: &[u8],
    texture_container: &[u8],
) -> Result<HudFont, &'static str> {
    let table = dat_entry_with_extension(table_container, FONT_TABLE_EXTENSION)
        .and_then(parse_font_table)
        .ok_or("the font table did not parse")?;
    let kerning = dat_entry_with_extension(table_container, KERNING_TABLE_EXTENSION)
        .and_then(parse_kerning_table)
        .unwrap_or_default();
    let spans = dat_entry_with_extension(table_container, TEXTURE_LIST_EXTENSION)
        .and_then(texture_spans)
        .ok_or("the font's texture list did not parse")?;
    let texture_data = dat_entry_with_extension(texture_container, TEXTURE_DATA_EXTENSION)
        .ok_or("the font's texture file is not a container with texture data")?;
    let mut wanted = wanted_cells(&table);
    wanted.sort_unstable_by_key(|(code, _)| *code);
    let mut glyphs = HashMap::with_capacity(wanted.len());
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let (mut pen_x, mut pen_y, mut row_height) = (0_u32, 0_u32, 0_u32);
    for (code, cell) in wanted {
        let (width, height) = (u32::from(cell.width), u32::from(cell.height));
        if width == 0 || height == 0 || width > HUD_FONT_ATLAS_WIDTH {
            continue;
        }
        if pen_x + width > HUD_FONT_ATLAS_WIDTH {
            pen_x = 0;
            pen_y += row_height + HUD_FONT_ATLAS_GAP_PIXELS;
            row_height = 0;
        }
        let span = spans
            .get(usize::from(cell.texture))
            .ok_or("a glyph names a texture the list does not have")?;
        let texture = texture_data
            .get(span.offset..span.offset + span.size)
            .ok_or("a font texture lies outside the texture file")?;
        let alpha = block_compressed_alpha(
            texture,
            usize::from(cell.left),
            usize::from(cell.top),
            usize::from(cell.width),
            usize::from(cell.height),
        )
        .ok_or("a font texture is not the block format with alpha, or a glyph lies outside it")?;
        let bottom = (pen_y + height) as usize;
        if rows.len() < bottom {
            rows.resize(bottom, vec![0_u8; HUD_FONT_ATLAS_WIDTH as usize]);
        }
        for (row, texels) in alpha.chunks_exact(width as usize).enumerate() {
            let start = pen_x as usize;
            rows[pen_y as usize + row][start..start + width as usize].copy_from_slice(texels);
        }
        if let Some(character) = char::from_u32(u32::from(code)) {
            glyphs.insert(
                character,
                PackedGlyph {
                    left: pen_x,
                    top: pen_y,
                    width,
                    height,
                },
            );
        }
        pen_x += width + HUD_FONT_ATLAS_GAP_PIXELS;
        row_height = row_height.max(height);
    }
    if glyphs.is_empty() {
        return Err("the font has no Latin glyph");
    }
    let (ink_top, ink_bottom) = capital_ink_rows(&glyphs, &rows, f32::from(table.cell_height));
    let mut coverage_rgba =
        Vec::with_capacity(rows.len() * HUD_FONT_ATLAS_WIDTH as usize * RGBA_BYTES);
    for alpha in rows.iter().flatten() {
        coverage_rgba.extend_from_slice(&[u8::MAX, u8::MAX, u8::MAX, *alpha]);
    }
    Ok(HudFont {
        coverage_rgba,
        texture_width: HUD_FONT_ATLAS_WIDTH,
        texture_height: rows.len() as u32,
        cell_height: f32::from(table.cell_height),
        ink_top,
        ink_bottom,
        advance_adjust: f32::from(table.advance_adjust),
        glyphs,
        kerning,
    })
}

fn wanted_cells(table: &FontTable) -> Vec<(u16, GlyphCell)> {
    table
        .glyphs
        .iter()
        .filter(|(code, _)| {
            (FIRST_PRINTED_CODE..=HUD_FONT_LAST_LATIN_CODE).contains(*code)
                || HUD_FONT_PUNCTUATION_CODES.contains(*code)
        })
        .map(|(code, cell)| (*code, *cell))
        .collect()
}

fn capital_ink_rows(
    glyphs: &HashMap<char, PackedGlyph>,
    rows: &[Vec<u8>],
    cell_height: f32,
) -> (f32, f32) {
    let mut inked: Option<(u32, u32)> = None;
    for glyph in HUD_FONT_FLAT_CAPITALS
        .chars()
        .filter_map(|capital| glyphs.get(&capital))
    {
        for row in 0..glyph.height {
            let texels = &rows[(glyph.top + row) as usize]
                [glyph.left as usize..(glyph.left + glyph.width) as usize];
            if texels.iter().any(|alpha| *alpha > HUD_FONT_INK_ALPHA) {
                let (top, bottom) = inked.unwrap_or((row, row + 1));
                inked = Some((top.min(row), bottom.max(row + 1)));
            }
        }
    }
    inked.map_or((0.0, cell_height), |(top, bottom)| {
        (top as f32, bottom as f32)
    })
}
