use crate::party_frame::pack::{first_texture_head, pack_resource_start};
use crate::party_frame::{
    AtlasRect, BATTLE_ATLAS_SIZE, GAUGE_FILL_GREEN_RECT, GAUGE_FILL_WHITE_RECT, GAUGE_LINE_RECT,
    GAUGE_TRACK_RECT, TEXTURE_FORMAT_BC3_SRGB, TEXTURE_FORMAT_BC3_UNORM, decode_bc3_region,
};

pub struct SpriteImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct GaugeSprites {
    pub track: SpriteImage,
    pub line: SpriteImage,
    pub fill_green: SpriteImage,
    pub fill_white: SpriteImage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeQuads {
    pub track_min: [f32; 2],
    pub track_max: [f32; 2],
    pub line_min: [f32; 2],
    pub line_max: [f32; 2],
    pub fill_min: [f32; 2],
    pub fill_max: [f32; 2],
    pub fill_uv_right: f32,
}

pub fn gauge_quads(origin: [f32; 2], scale: [f32; 2], fraction: f32) -> GaugeQuads {
    let at = |x: f32, y: f32| [origin[0] + x * scale[0], origin[1] + y * scale[1]];
    let track_center_x =
        (GAUGE_TRACK_RECT.left + GAUGE_TRACK_RECT.right) as f32 * 0.5 - GAUGE_LINE_RECT.left as f32;
    let track_center_y = GAUGE_TRACK_RECT.height() as f32 * 0.5;
    let fill_half_width = GAUGE_FILL_GREEN_RECT.width() as f32 * 0.5;
    let fill_half_height = GAUGE_FILL_GREEN_RECT.height() as f32 * 0.5;
    let fill_left = track_center_x - fill_half_width;
    let fraction = fraction.clamp(0.0, 1.0);
    let line_top = (GAUGE_LINE_RECT.top - GAUGE_TRACK_RECT.top) as f32;
    GaugeQuads {
        track_min: at((GAUGE_TRACK_RECT.left - GAUGE_LINE_RECT.left) as f32, 0.0),
        track_max: at(
            (GAUGE_TRACK_RECT.right - GAUGE_LINE_RECT.left) as f32,
            GAUGE_TRACK_RECT.height() as f32,
        ),
        line_min: at(0.0, line_top),
        line_max: at(
            GAUGE_LINE_RECT.width() as f32,
            line_top + GAUGE_LINE_RECT.height() as f32,
        ),
        fill_min: at(fill_left, track_center_y - fill_half_height),
        fill_max: at(
            fill_left + fill_half_width * 2.0 * fraction,
            track_center_y + fill_half_height,
        ),
        fill_uv_right: fraction,
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
        track: cut(GAUGE_TRACK_RECT)?,
        line: cut(GAUGE_LINE_RECT)?,
        fill_green: cut(GAUGE_FILL_GREEN_RECT)?,
        fill_white: cut(GAUGE_FILL_WHITE_RECT)?,
    })
}
