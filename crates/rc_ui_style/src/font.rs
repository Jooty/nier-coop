pub const FONT_GLYPH_RANGES: [u32; 9] = [
    0x0020, 0x00FF, 0x0100, 0x024F, 0x0370, 0x03FF, 0x0400, 0x04FF, 0,
];

pub fn font_bytes() -> &'static [u8] {
    epaint_default_fonts::UBUNTU_LIGHT
}
