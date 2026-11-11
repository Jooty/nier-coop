mod arc_asset;
mod atlas;
mod bc3;
mod font;
mod gauge;
mod layout;
mod pack;
mod status;
mod tuning;

pub use arc_asset::ArcAsset;
pub use atlas::AtlasRect;
pub use bc3::decode_bc3_region;
pub use font::{GlyphQuad, HudFont, HudGlyph, parse_font};
pub use gauge::{
    GaugeKind, GaugeQuads, GaugeSprites, SpriteImage, TexturedQuad, gauge_length_ratio,
    gauge_quads, parse_gauge_sprites,
};
pub use layout::{PartyFrameLayout, party_frame_layout};
pub use status::{PartyStatus, health_fraction, party_frame_visible, party_status};
pub use tuning::{PartyFrameTuning, TUNING_FIELDS, TuningField};

pub const REFERENCE_SCREEN_HEIGHT: f32 = 1080.0;
pub const PARTY_FRAME_PEER_TIMEOUT_MS: u64 = 3000;
pub const PARTY_FRAME_RIGHT_MARGIN_REFERENCE_PIXELS: f32 = 64.0;
pub const PARTY_FRAME_TOP_REFERENCE_PIXELS: f32 = 94.0;
pub const PARTY_FRAME_BAR_RIGHT_FROM_SCREEN_RIGHT_REFERENCE_PIXELS: f32 = 214.0;
pub const PARTY_FRAME_NAME_GAP_REFERENCE_PIXELS: f32 = 8.0;
pub const PARTY_FRAME_NAME_SIZE_REFERENCE_PIXELS: f32 = 20.0;
pub const PARTY_FRAME_STATUS_INSET_REFERENCE_PIXELS: f32 = 6.0;

pub const INIT_ARC_FILE_NAME: &str = "init.arc";
pub const GAME_DATA_FOLDER_NAME: &str = "data";
pub const HUD_FONT_EN_ASSET: ArcAsset = ArcAsset {
    archive_offset: 8_280_709_888,
    compressed_size: 1_549_552,
    decoded_size: 2_128_400,
};
pub const BATTLE_UI_ASSET: ArcAsset = ArcAsset {
    archive_offset: 8_314_136_304,
    compressed_size: 1_212_645,
    decoded_size: 4_869_520,
};

pub const PACK_MAGIC: &[u8; 4] = b"PACK";
pub const PACK_RESOURCE_START_FIELD: usize = 12;
pub const TEXTURE_HEAD_TYPE_NAME: &[u8] = b"tpGxTexHead";
pub const TEXTURE_WIDTH_FROM_TYPE_NAME: usize = 24;
pub const TEXTURE_HEIGHT_FROM_TYPE_NAME: usize = 28;
pub const TEXTURE_FORMAT_FROM_TYPE_NAME: usize = 48;
pub const TEXTURE_FORMAT_BC3_UNORM: u32 = 0x0001_1300;
pub const TEXTURE_FORMAT_BC3_SRGB: u32 = 0x0001_1400;

pub const FONT_PARAM_TYPE_NAME: &[u8] = b"tpTandaFontSdfParam";
pub const FONT_GLYPH_COUNT_FROM_TYPE_NAME: usize = 0x3C;
pub const FONT_GLYPH_TABLE_FIELD_FROM_TYPE_NAME: usize = 0x40;
pub const FONT_GLYPH_RECORD_BYTES: usize = 40;
pub const FONT_MAX_GLYPHS: usize = 8192;
pub const FONT_CHANNEL_COUNT: u32 = 4;
pub const FONT_CHANNEL_SHIFT: u32 = 8;
pub const FONT_SDF_EDGE_LOW: f32 = 0.42;
pub const FONT_SDF_EDGE_HIGH: f32 = 0.58;
pub const FONT_BASELINE_UNITS_BELOW_TOP: f32 = 40.0;
pub const FONT_CELL_UNITS: f32 = 44.0;
pub const FONT_FALLBACK_CHARACTER: char = '?';

pub const BC3_BLOCK_BYTES: usize = 16;
pub const BC3_BLOCK_PIXELS: u32 = 4;

pub const BATTLE_ATLAS_SIZE: u32 = 2048;
pub const LIFE_BASE_RECT: AtlasRect = AtlasRect::new(2, 1787, 588, 1822);
pub const MAGIC_BASE_RECT: AtlasRect = AtlasRect::new(2, 1750, 600, 1785);
pub const LIFE_BAR_RECT: AtlasRect = AtlasRect::new(2, 1975, 482, 1989);
pub const MAGIC_BAR_RECT: AtlasRect = AtlasRect::new(720, 1734, 1201, 1748);
pub const BASE_LEFT_CAP_END_SPRITE_PIXELS: f32 = 40.0;
pub const BASE_RIGHT_CAP_START_SPRITE_PIXELS: f32 = 510.0;
pub const BASE_WELL_LEFT_SPRITE_PIXELS: f32 = 37.0;
pub const BASE_WELL_TOP_SPRITE_PIXELS: f32 = 10.0;
pub const BASE_WELL_BOTTOM_SPRITE_PIXELS: f32 = 24.0;
pub const BASE_FRAME_TOP_SPRITE_PIXELS: f32 = 7.0;
pub const BASE_FRAME_BOTTOM_SPRITE_PIXELS: f32 = 27.0;
pub const GAUGE_FULL_LENGTH_REFERENCE_PIXELS: f32 = 480.0;
pub const GAUGE_FULL_LENGTH_STORED_LEVEL: f32 = 98.0;
pub const LIFE_GAUGE_RATIO_AT_STORED_LEVEL_ZERO: f32 = 0.24;
pub const MAGIC_GAUGE_RATIO_AT_STORED_LEVEL_ZERO: f32 = 0.2;
pub const MAGIC_BAR_HEIGHT_SPRITE_PIXELS: f32 = 10.0;
