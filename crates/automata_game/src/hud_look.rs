mod alpha_blocks;
mod archive_asset;
mod crilayla;
mod dat_container;
mod font_table;
mod hud_font;
mod le_bytes;
mod palette;
mod texture_list;

pub use alpha_blocks::block_compressed_alpha;
pub use archive_asset::ArchiveAsset;
pub use crilayla::unpack_crilayla;
pub use dat_container::{DatEntry, dat_entries, dat_entry_with_extension};
pub use font_table::{FontTable, GlyphCell, parse_font_table, parse_kerning_table};
pub use hud_font::{GlyphQuad, HudFont, build_hud_font};
pub use palette::{HudPalette, hud_palette};
pub use texture_list::{TextureSpan, texture_spans};

pub const GAME_DATA_FOLDER_NAME: &str = "data";
pub const UI_ARCHIVE_FILE_NAME: &str = "data009.cpk";
pub const HUD_FONT_TABLE_ASSET: ArchiveAsset = ArchiveAsset {
    archive_offset: 304_564_224,
    stored_size: 46_228,
    unpacked_size: 91_408,
};
pub const HUD_FONT_TEXTURE_ASSET: ArchiveAsset = ArchiveAsset {
    archive_offset: 304_611_328,
    stored_size: 10_494_176,
    unpacked_size: 10_494_176,
};
pub const HUD_LAYOUT_ASSET: ArchiveAsset = ArchiveAsset {
    archive_offset: 838_457_344,
    stored_size: 102_532,
    unpacked_size: 848_304,
};
pub const DDS_HEADER_BYTES: usize = 128;
pub const DDS_MAGIC: &[u8; 4] = b"DDS ";
pub const DDS_HEIGHT_OFFSET: usize = 12;
pub const DDS_WIDTH_OFFSET: usize = 16;
pub const DDS_FOUR_CC_OFFSET: usize = 84;
pub const DDS_FOUR_CC_OF_BLOCKS_WITH_ALPHA: &[u8; 4] = b"DXT5";
pub const BLOCK_BYTES: usize = 16;
pub const BLOCK_EDGE_PIXELS: usize = 4;
pub const DAT_MAGIC: &[u8; 4] = b"DAT\0";
pub const TEXTURE_LIST_MAGIC: &[u8; 4] = b"WTB\0";
pub const FONT_TABLE_MAGIC: &[u8; 4] = b"FTB ";
pub const FONT_TABLE_CELL_HEIGHT_OFFSET: usize = 0x6E;
pub const FONT_TABLE_LINE_HEIGHT_OFFSET: usize = 0x70;
pub const FONT_TABLE_ADVANCE_ADJUST_OFFSET: usize = 0x72;
pub const FONT_TABLE_GLYPH_COUNT_OFFSET: usize = 0x7A;
pub const FONT_TABLE_GLYPHS_OFFSET_OFFSET: usize = 0x80;
pub const FONT_TABLE_GLYPH_BYTES: usize = 12;
pub const KERNING_PAIR_BYTES: usize = 6;
pub const CRILAYLA_MAGIC: &[u8; 8] = b"CRILAYLA";
pub const CRILAYLA_HEADER_BYTES: usize = 16;
pub const CRILAYLA_RAW_PREFIX_BYTES: usize = 0x100;
pub const CRILAYLA_LARGEST_UNPACKED_BYTES: usize = 64 * 1024 * 1024;
pub const FONT_TABLE_EXTENSION: &str = ".ftb";
pub const KERNING_TABLE_EXTENSION: &str = ".ktb";
pub const TEXTURE_LIST_EXTENSION: &str = ".wta";
pub const TEXTURE_DATA_EXTENSION: &str = ".wtp";
pub const LAYOUT_EXTENSION: &str = ".uid";
pub const HUD_FONT_ATLAS_WIDTH: u32 = 1024;
pub const HUD_FONT_ATLAS_GAP_PIXELS: u32 = 1;
pub const HUD_FONT_LAST_LATIN_CODE: u16 = 0x24F;
pub const HUD_FONT_PUNCTUATION_CODES: core::ops::RangeInclusive<u16> = 0x2010..=0x2026;
pub const HUD_FONT_MISSING_GLYPH: char = '?';
pub const HUD_FONT_INK_ALPHA: u8 = 40;
pub const HUD_FONT_FLAT_CAPITALS: &str = "EFHILTXZ";
pub const PALETTE_SMALLEST_CHANNEL: f32 = 0.05;
pub const PALETTE_FEWEST_USES: u32 = 8;
pub const PALETTE_DARK_BELOW_LUMA: f32 = 0.5;
pub const PALETTE_ACCENT_CHANNEL_SPREAD: f32 = 0.4;
