use std::collections::HashMap;

use crate::hud_look::le_bytes::f32_at;
use crate::hud_look::{
    LAYOUT_EXTENSION, PALETTE_ACCENT_CHANNEL_SPREAD, PALETTE_DARK_BELOW_LUMA, PALETTE_FEWEST_USES,
    PALETTE_SMALLEST_CHANNEL, dat_entries,
};

const COLOR_STEP_BYTES: usize = 4;
const COLOR_BYTES: usize = 16;
const LUMA_WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// The colours the game's own HUD layouts use most: its light, its dark and its warning colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudPalette {
    pub light: [f32; 3],
    pub dark: [f32; 3],
    pub accent: [f32; 3],
}

/// Counts the opaque, coloured layer colours in every layout of the game's HUD container (already
/// unpacked) and picks the most used; `None` when the container does not read or holds too few.
pub fn hud_palette(hud_container: &[u8]) -> Option<HudPalette> {
    let mut uses: HashMap<[u8; 3], u32> = HashMap::new();
    for layout in dat_entries(hud_container)?
        .iter()
        .filter(|entry| entry.name.ends_with(LAYOUT_EXTENSION))
    {
        let mut at = 0;
        while at + COLOR_BYTES <= layout.bytes.len() {
            if let Some(color) = layer_color_at(layout.bytes, at) {
                *uses.entry(color).or_default() += 1;
            }
            at += COLOR_STEP_BYTES;
        }
    }
    let most_used = |keep: &dyn Fn([f32; 3]) -> bool| {
        uses.iter()
            .map(|(color, count)| (color.map(|channel| f32::from(channel) / 255.0), *count))
            .filter(|(color, count)| *count >= PALETTE_FEWEST_USES && keep(*color))
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| color)
    };
    Some(HudPalette {
        light: most_used(&|color| luma(color) >= PALETTE_DARK_BELOW_LUMA && !is_accent(color))?,
        dark: most_used(&|color| luma(color) < PALETTE_DARK_BELOW_LUMA && !is_accent(color))?,
        accent: most_used(&is_accent)?,
    })
}

fn layer_color_at(layout: &[u8], at: usize) -> Option<[u8; 3]> {
    let channel = |index: usize| f32_at(layout, at + index * 4);
    let rgb = [channel(0)?, channel(1)?, channel(2)?];
    let coloured = rgb
        .iter()
        .all(|value| *value > PALETTE_SMALLEST_CHANNEL && *value < 1.0)
        && !(rgb[0] == rgb[1] && rgb[1] == rgb[2]);
    (coloured && channel(3)? == 1.0).then(|| rgb.map(|value| (value * 255.0).round() as u8))
}

fn luma(color: [f32; 3]) -> f32 {
    color
        .iter()
        .zip(LUMA_WEIGHTS)
        .map(|(channel, weight)| channel * weight)
        .sum()
}

fn is_accent(color: [f32; 3]) -> bool {
    let brightest = color.iter().copied().fold(f32::MIN, f32::max);
    let dimmest = color.iter().copied().fold(f32::MAX, f32::min);
    brightest - dimmest > PALETTE_ACCENT_CHANNEL_SPREAD
}
