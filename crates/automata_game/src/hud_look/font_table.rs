use std::collections::HashMap;

use crate::hud_look::le_bytes::{u16_at, u32_at};
use crate::hud_look::{
    FONT_TABLE_ADVANCE_ADJUST_OFFSET, FONT_TABLE_CELL_HEIGHT_OFFSET, FONT_TABLE_GLYPH_BYTES,
    FONT_TABLE_GLYPH_COUNT_OFFSET, FONT_TABLE_GLYPHS_OFFSET_OFFSET, FONT_TABLE_LINE_HEIGHT_OFFSET,
    FONT_TABLE_MAGIC, KERNING_PAIR_BYTES,
};

/// One glyph's cell in a font texture, in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphCell {
    pub texture: u16,
    pub width: u16,
    pub height: u16,
    pub left: u16,
    pub top: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontTable {
    pub cell_height: u16,
    pub line_height: u16,
    pub advance_adjust: i16,
    pub glyphs: HashMap<u16, GlyphCell>,
}

pub fn parse_font_table(table: &[u8]) -> Option<FontTable> {
    if !table.starts_with(FONT_TABLE_MAGIC) {
        return None;
    }
    let count = usize::from(u16_at(table, FONT_TABLE_GLYPH_COUNT_OFFSET)?);
    let first = u32_at(table, FONT_TABLE_GLYPHS_OFFSET_OFFSET)? as usize;
    let mut glyphs = HashMap::with_capacity(count);
    for index in 0..count {
        let at = first + index * FONT_TABLE_GLYPH_BYTES;
        glyphs.insert(
            u16_at(table, at)?,
            GlyphCell {
                texture: u16_at(table, at + 2)?,
                width: u16_at(table, at + 4)?,
                height: u16_at(table, at + 6)?,
                left: u16_at(table, at + 8)?,
                top: u16_at(table, at + 10)?,
            },
        );
    }
    let cell_height = u16_at(table, FONT_TABLE_CELL_HEIGHT_OFFSET)?;
    (cell_height > 0 && !glyphs.is_empty()).then_some(FontTable {
        cell_height,
        line_height: u16_at(table, FONT_TABLE_LINE_HEIGHT_OFFSET)?,
        advance_adjust: u16_at(table, FONT_TABLE_ADVANCE_ADJUST_OFFSET)? as i16,
        glyphs,
    })
}

/// The pair adjustments of a font, in texels, keyed by the left and the right character.
pub fn parse_kerning_table(table: &[u8]) -> Option<HashMap<(u16, u16), i16>> {
    let count = usize::from(u16_at(table, 0)?);
    (0..count)
        .map(|index| {
            let at = 2 + index * KERNING_PAIR_BYTES;
            Some((
                (u16_at(table, at)?, u16_at(table, at + 2)?),
                u16_at(table, at + 4)? as i16,
            ))
        })
        .collect()
}
