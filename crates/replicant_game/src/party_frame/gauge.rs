use crate::party_frame::pack::{first_texture_head, pack_resource_start};
use crate::party_frame::{
    AtlasRect, BASE_FRAME_TOP_SPRITE_PIXELS, BASE_LEFT_CAP_END_SPRITE_PIXELS,
    BASE_RIGHT_CAP_START_SPRITE_PIXELS, BASE_WELL_BOTTOM_SPRITE_PIXELS,
    BASE_WELL_LEFT_SPRITE_PIXELS, BASE_WELL_TOP_SPRITE_PIXELS, BATTLE_ATLAS_SIZE,
    GAUGE_FULL_LENGTH_REFERENCE_PIXELS, GAUGE_FULL_LENGTH_STORED_LEVEL, LIFE_BAR_RECT,
    LIFE_BASE_RECT, LIFE_GAUGE_RATIO_AT_STORED_LEVEL_ZERO, MAGIC_BAR_HEIGHT_SPRITE_PIXELS,
    MAGIC_BAR_RECT, MAGIC_BASE_RECT, MAGIC_GAUGE_RATIO_AT_STORED_LEVEL_ZERO,
    TEXTURE_FORMAT_BC3_SRGB, TEXTURE_FORMAT_BC3_UNORM, decode_bc3_region,
};

pub struct SpriteImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct GaugeSprites {
    pub life_base: SpriteImage,
    pub magic_base: SpriteImage,
    pub life_bar: SpriteImage,
    pub magic_bar: SpriteImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeKind {
    Life,
    Magic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TexturedQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

/// One gauge in screen pixels: the base sprite in three slices (left cap, stretched middle, right
/// cap) and the fill inside the base's well. `well_min`/`well_max` bound the full-health fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeQuads {
    pub base: [TexturedQuad; 3],
    pub fill: Option<TexturedQuad>,
    pub well_min: [f32; 2],
    pub well_max: [f32; 2],
}

impl GaugeKind {
    pub const fn base_rect(self) -> AtlasRect {
        match self {
            Self::Life => LIFE_BASE_RECT,
            Self::Magic => MAGIC_BASE_RECT,
        }
    }

    pub const fn bar_rect(self) -> AtlasRect {
        match self {
            Self::Life => LIFE_BAR_RECT,
            Self::Magic => MAGIC_BAR_RECT,
        }
    }

    pub const fn ratio_at_stored_level_zero(self) -> f32 {
        match self {
            Self::Life => LIFE_GAUGE_RATIO_AT_STORED_LEVEL_ZERO,
            Self::Magic => MAGIC_GAUGE_RATIO_AT_STORED_LEVEL_ZERO,
        }
    }

    /// Only the life bar sits in the brush-stroke base; the magic bar is a thinner bar on its own,
    /// starting where its frame would start.
    pub const fn has_brush_frame(self) -> bool {
        matches!(self, Self::Life)
    }

    const fn fill_rows_sprite_pixels(self) -> (f32, f32) {
        match self {
            Self::Life => (BASE_WELL_TOP_SPRITE_PIXELS, BASE_WELL_BOTTOM_SPRITE_PIXELS),
            Self::Magic => (
                BASE_FRAME_TOP_SPRITE_PIXELS,
                BASE_FRAME_TOP_SPRITE_PIXELS + MAGIC_BAR_HEIGHT_SPRITE_PIXELS,
            ),
        }
    }
}

/// The game's own length rule (`CNierLifeGauge`, `CNierMagicGauge`): the gauge's share of its full
/// length grows linearly with the level stored in the save block and is full at stored level 98.
pub fn gauge_length_ratio(kind: GaugeKind, stored_level: u8) -> f32 {
    let start = kind.ratio_at_stored_level_zero();
    let progress = f32::from(stored_level) / GAUGE_FULL_LENGTH_STORED_LEVEL;
    (progress * (1.0 - start) + start).clamp(0.0, 1.0)
}

/// `well_right_top` is the right end of the well (the anchor the gauge grows away from, to the
/// left) and the top of the black frame; `scale` is screen pixels per sprite pixel.
pub fn gauge_quads(
    kind: GaugeKind,
    well_right_top: [f32; 2],
    scale: f32,
    length_ratio: f32,
    fill_fraction: f32,
) -> GaugeQuads {
    let base_width = kind.base_rect().width() as f32;
    let base_height = kind.base_rect().height() as f32;
    let bar_width = kind.bar_rect().width() as f32;
    let well_length = GAUGE_FULL_LENGTH_REFERENCE_PIXELS * length_ratio.clamp(0.0, 1.0);
    let shrink = (GAUGE_FULL_LENGTH_REFERENCE_PIXELS - well_length)
        .min(BASE_RIGHT_CAP_START_SPRITE_PIXELS - BASE_LEFT_CAP_END_SPRITE_PIXELS);
    let well_right = BASE_WELL_LEFT_SPRITE_PIXELS + GAUGE_FULL_LENGTH_REFERENCE_PIXELS - shrink;
    let left = well_right_top[0] - well_right * scale;
    let top = well_right_top[1] - BASE_FRAME_TOP_SPRITE_PIXELS * scale;
    let at = |x: f32, y: f32| [left + x * scale, top + y * scale];
    let slice = |from: f32, to: f32, uv_from: f32, uv_to: f32| TexturedQuad {
        min: at(from, 0.0),
        max: at(to, base_height),
        uv_min: [uv_from / base_width, 0.0],
        uv_max: [uv_to / base_width, 1.0],
    };
    let middle_end = BASE_RIGHT_CAP_START_SPRITE_PIXELS - shrink;
    let base = [
        slice(
            0.0,
            BASE_LEFT_CAP_END_SPRITE_PIXELS,
            0.0,
            BASE_LEFT_CAP_END_SPRITE_PIXELS,
        ),
        slice(
            BASE_LEFT_CAP_END_SPRITE_PIXELS,
            middle_end,
            BASE_LEFT_CAP_END_SPRITE_PIXELS,
            BASE_RIGHT_CAP_START_SPRITE_PIXELS,
        ),
        slice(
            middle_end,
            base_width - shrink,
            BASE_RIGHT_CAP_START_SPRITE_PIXELS,
            base_width,
        ),
    ];
    let well_left = BASE_WELL_LEFT_SPRITE_PIXELS;
    let (fill_top, fill_bottom) = kind.fill_rows_sprite_pixels();
    let fill_length = (well_right - well_left) * fill_fraction.clamp(0.0, 1.0);
    let fill = (fill_length > 0.0).then(|| TexturedQuad {
        min: at(well_right - fill_length, fill_top),
        max: at(well_right, fill_bottom),
        uv_min: [1.0 - (fill_length / bar_width).min(1.0), 0.0],
        uv_max: [1.0, 1.0],
    });
    GaugeQuads {
        base,
        fill,
        well_min: at(well_left, fill_top),
        well_max: at(well_right, fill_bottom),
    }
}

pub fn parse_gauge_sprites(battle_pack: &[u8]) -> Option<GaugeSprites> {
    let head = first_texture_head(battle_pack)?;
    let bc3 = head.format == TEXTURE_FORMAT_BC3_UNORM || head.format == TEXTURE_FORMAT_BC3_SRGB;
    if !bc3 || head.width != BATTLE_ATLAS_SIZE || head.height != BATTLE_ATLAS_SIZE {
        return None;
    }
    let start = pack_resource_start(battle_pack)?;
    let atlas_bytes = (BATTLE_ATLAS_SIZE * BATTLE_ATLAS_SIZE) as usize;
    let atlas = battle_pack.get(start..start.checked_add(atlas_bytes)?)?;
    let cut = |rect: AtlasRect| {
        Some(SpriteImage {
            width: rect.width(),
            height: rect.height(),
            rgba: decode_bc3_region(atlas, BATTLE_ATLAS_SIZE, rect)?,
        })
    };
    Some(GaugeSprites {
        life_base: cut(LIFE_BASE_RECT)?,
        magic_base: cut(MAGIC_BASE_RECT)?,
        life_bar: cut(LIFE_BAR_RECT)?,
        magic_bar: cut(MAGIC_BAR_RECT)?,
    })
}
