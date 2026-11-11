use std::collections::HashMap;

use crate::party_frame::pack::{
    find_bytes, first_texture_head, pack_resource_start, read_f32, read_u16, read_u32,
};
use crate::party_frame::{
    FONT_BASELINE_UNITS_BELOW_TOP, FONT_CHANNEL_COUNT, FONT_CHANNEL_SHIFT, FONT_FALLBACK_CHARACTER,
    FONT_GLYPH_COUNT_FROM_TYPE_NAME, FONT_GLYPH_RECORD_BYTES,
    FONT_GLYPH_TABLE_FIELD_FROM_TYPE_NAME, FONT_MAX_GLYPHS, FONT_PARAM_TYPE_NAME,
    FONT_SDF_EDGE_HIGH, FONT_SDF_EDGE_LOW,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudGlyph {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub advance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

pub struct HudFont {
    pub texture_width: u32,
    pub texture_height: u32,
    pub coverage_rgba: Vec<u8>,
    glyphs: HashMap<char, HudGlyph>,
}

impl HudFont {
    pub fn glyph(&self, character: char) -> Option<&HudGlyph> {
        self.glyphs
            .get(&character)
            .or_else(|| self.glyphs.get(&FONT_FALLBACK_CHARACTER))
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn text_width(&self, text: &str, scale: f32) -> f32 {
        text.chars()
            .filter_map(|character| self.glyph(character))
            .map(|glyph| glyph.advance * scale)
            .sum()
    }

    pub fn layout_text(&self, text: &str, origin: [f32; 2], scale: f32) -> Vec<GlyphQuad> {
        let baseline = origin[1] + FONT_BASELINE_UNITS_BELOW_TOP * scale;
        let mut pen = origin[0];
        let mut quads = Vec::new();
        for character in text.chars() {
            let Some(glyph) = self.glyph(character) else {
                continue;
            };
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                let min = [
                    pen + glyph.offset[0] * scale,
                    baseline + glyph.offset[1] * scale,
                ];
                quads.push(GlyphQuad {
                    min,
                    max: [
                        min[0] + glyph.size[0] * scale,
                        min[1] + glyph.size[1] * scale,
                    ],
                    uv_min: glyph.uv_min,
                    uv_max: glyph.uv_max,
                });
            }
            pen += glyph.advance * scale;
        }
        quads
    }

    pub fn fit_text(&self, text: &str, scale: f32, max_width: f32) -> String {
        if self.text_width(text, scale) <= max_width {
            return text.to_owned();
        }
        let mut kept: Vec<char> = text.chars().collect();
        while !kept.is_empty() {
            kept.pop();
            let candidate: String = kept.iter().collect::<String>() + "..";
            if self.text_width(&candidate, scale) <= max_width {
                return candidate;
            }
        }
        String::new()
    }
}

pub fn parse_font(bytes: &[u8]) -> Option<HudFont> {
    let resource_start = pack_resource_start(bytes)?;
    let head = first_texture_head(bytes)?;
    let atlas_len = (head.width as usize)
        .checked_mul(head.height as usize)?
        .checked_mul(4)?;
    let atlas = bytes.get(resource_start..resource_start.checked_add(atlas_len)?)?;
    let type_at = find_bytes(bytes, FONT_PARAM_TYPE_NAME)?;
    let count = read_u32(bytes, type_at + FONT_GLYPH_COUNT_FROM_TYPE_NAME)? as usize;
    if count == 0 || count > FONT_MAX_GLYPHS {
        return None;
    }
    let table_field = type_at + FONT_GLYPH_TABLE_FIELD_FROM_TYPE_NAME;
    let table = table_field + read_u32(bytes, table_field)? as usize;
    let mut glyphs = HashMap::new();
    for index in 0..count {
        let at = table + index * FONT_GLYPH_RECORD_BYTES;
        let code = read_u32(bytes, at)?;
        let flags = read_u32(bytes, at + 36)?;
        let channel = ((flags >> FONT_CHANNEL_SHIFT) & 3) as f32;
        let v_of = |v: f32| (channel + v) / FONT_CHANNEL_COUNT as f32;
        let glyph = HudGlyph {
            uv_min: [read_f32(bytes, at + 4)?, v_of(read_f32(bytes, at + 8)?)],
            uv_max: [read_f32(bytes, at + 12)?, v_of(read_f32(bytes, at + 16)?)],
            offset: [read_f32(bytes, at + 20)?, read_f32(bytes, at + 24)?],
            size: [
                f32::from(read_u16(bytes, at + 32)?),
                f32::from(read_u16(bytes, at + 34)?),
            ],
            advance: read_f32(bytes, at + 28)?,
        };
        if let Some(character) = char::from_u32(code) {
            glyphs.entry(character).or_insert(glyph);
        }
    }
    Some(HudFont {
        texture_width: head.width,
        texture_height: head.height * FONT_CHANNEL_COUNT,
        coverage_rgba: coverage_atlas(atlas, head.width, head.height),
        glyphs,
    })
}

fn coverage_atlas(atlas: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixels_per_channel = width as usize * height as usize;
    let mut out = vec![255u8; pixels_per_channel * FONT_CHANNEL_COUNT as usize * 4];
    for channel in 0..FONT_CHANNEL_COUNT as usize {
        for pixel in 0..pixels_per_channel {
            let distance = f32::from(atlas[pixel * 4 + channel]) / 255.0;
            let t = ((distance - FONT_SDF_EDGE_LOW) / (FONT_SDF_EDGE_HIGH - FONT_SDF_EDGE_LOW))
                .clamp(0.0, 1.0);
            let coverage = t * t * (3.0 - 2.0 * t);
            out[(channel * pixels_per_channel + pixel) * 4 + 3] = (coverage * 255.0).round() as u8;
        }
    }
    out
}
