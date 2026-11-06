use rc_game::party_frame::{
    AtlasRect, PartyStatus, decode_bc3_region, gauge_quads, health_fraction, parse_font,
    party_frame_layout, party_status,
};

#[test]
fn an_unknown_or_silent_peer_is_disconnected() {
    assert_eq!(
        party_status(false, 0, true, false),
        PartyStatus::Disconnected
    );
    assert_eq!(
        party_status(true, 5000, true, false),
        PartyStatus::Disconnected
    );
}

#[test]
fn another_phase_beats_downed() {
    assert_eq!(party_status(true, 10, false, true), PartyStatus::OtherArea);
}

#[test]
fn downed_and_up_in_the_same_phase() {
    assert_eq!(party_status(true, 10, true, true), PartyStatus::Downed);
    assert_eq!(party_status(true, 10, true, false), PartyStatus::Up);
}

#[test]
fn health_fraction_is_clamped_and_safe_for_zero_max() {
    assert_eq!(health_fraction(5, 10), 0.5);
    assert_eq!(health_fraction(20, 10), 1.0);
    assert_eq!(health_fraction(3, 0), 0.0);
}

#[test]
fn layout_sits_at_the_right_edge_and_scales_with_height() {
    let small = party_frame_layout(1920.0, 1080.0);
    let large = party_frame_layout(3840.0, 2160.0);
    assert!(small.max[0] < 1920.0);
    assert!(large.max[0] < 3840.0);
    assert!((large.max[1] - large.min[1] - 2.0 * (small.max[1] - small.min[1])).abs() < 0.01);
    assert!((large.scale - 2.0 * small.scale).abs() < 1e-6);
    assert!(small.name_origin[0] > small.icon_center[0] + small.icon_radius);
    assert!(small.gauge_origin[1] > small.name_origin[1]);
    assert!(small.gauge_origin[0] + 520.0 * small.gauge_scale[0] <= small.max[0] + 0.01);
}

#[test]
fn gauge_fill_shrinks_with_health_and_stays_inside_the_frame_width() {
    let full = gauge_quads([100.0, 50.0], [0.5, 0.5], 1.0);
    let half = gauge_quads([100.0, 50.0], [0.5, 0.5], 0.5);
    assert_eq!(full.fill_uv_right, 1.0);
    assert!(
        (half.fill_max[0] - half.fill_min[0] - (full.fill_max[0] - full.fill_min[0]) * 0.5).abs()
            < 1e-4
    );
    assert_eq!(full.track_min[1], 50.0);
    assert!(full.line_min[1] >= full.track_max[1] - 0.01);
    assert!((full.line_max[0] - 100.0 - 260.0).abs() < 1e-4);
    let empty = gauge_quads([0.0, 0.0], [1.0, 1.0], 0.0);
    assert_eq!(empty.fill_min[0], empty.fill_max[0]);
}

#[test]
fn bc3_block_decodes_opaque_endpoints() {
    let mut data = vec![0u8; 16];
    data[0] = 255;
    data[1] = 255;
    data[8..10].copy_from_slice(&0xF800u16.to_le_bytes());
    data[10..12].copy_from_slice(&0x001Fu16.to_le_bytes());
    let rgba = decode_bc3_region(&data, 4, AtlasRect::new(0, 0, 4, 4)).unwrap();
    assert_eq!(&rgba[..4], &[255, 0, 0, 255]);
    assert_eq!(rgba.len(), 64);
}

#[test]
fn bc3_region_must_be_block_aligned_and_inside_the_data() {
    let data = vec![0u8; 16];
    assert!(decode_bc3_region(&data, 4, AtlasRect::new(0, 0, 3, 4)).is_none());
    assert!(decode_bc3_region(&data, 4, AtlasRect::new(0, 0, 4, 8)).is_none());
}

fn tiny_font_pack() -> Vec<u8> {
    let mut head = vec![0u8; 16];
    head[..4].copy_from_slice(b"PACK");
    head.extend_from_slice(b"tpGxTexHead\0");
    head.resize(16 + 24, b'&');
    for word in [2u32, 2, 1, 1, 16, 0x8000_0000, 0x10700] {
        head.extend_from_slice(&word.to_le_bytes());
    }
    let type_at = head.len();
    head.extend_from_slice(b"tpTandaFontSdfParam");
    head.resize(type_at + 0x3C, 0);
    head.extend_from_slice(&2u32.to_le_bytes());
    head.extend_from_slice(&4u32.to_le_bytes());
    let glyph = |code: u32, flags: u32, advance: f32| {
        let mut record = Vec::new();
        record.extend_from_slice(&code.to_le_bytes());
        for value in [0.0f32, 0.0, 1.0, 0.5, 1.0, -40.0, advance] {
            record.extend_from_slice(&value.to_le_bytes());
        }
        record.extend_from_slice(&10u16.to_le_bytes());
        record.extend_from_slice(&20u16.to_le_bytes());
        record.extend_from_slice(&flags.to_le_bytes());
        record
    };
    head.extend(glyph(u32::from('A'), 0x4000_0100, 30.0));
    head.extend(glyph(u32::from('?'), 0x4000_0000, 12.0));
    let resource_start = head.len() as u32;
    head[12..16].copy_from_slice(&resource_start.to_le_bytes());
    head.extend(std::iter::repeat_n([255u8, 0, 0, 0], 4).flatten());
    head
}

#[test]
fn font_pack_parses_glyphs_channels_and_coverage() {
    let font = parse_font(&tiny_font_pack()).unwrap();
    assert_eq!(font.glyph_count(), 2);
    assert_eq!(font.texture_height, 8);
    let a = font.glyph('A').unwrap();
    assert_eq!(a.uv_min[1], 0.25);
    assert_eq!(a.uv_max[1], (1.0 + 0.5) / 4.0);
    assert_eq!(font.glyph('Z').unwrap().advance, 12.0);
    assert_eq!(font.coverage_rgba[3], 255);
    assert_eq!(font.coverage_rgba[(4 * 4 + 3) as usize], 0);
}

#[test]
fn text_layout_advances_and_fits() {
    let font = parse_font(&tiny_font_pack()).unwrap();
    assert_eq!(font.text_width("AA", 0.5), 30.0);
    let quads = font.layout_text("AA", [10.0, 0.0], 1.0);
    assert_eq!(quads.len(), 2);
    assert_eq!(quads[1].min[0] - quads[0].min[0], 30.0);
    assert_eq!(quads[0].min[1], 0.0);
    assert_eq!(font.fit_text("AA", 1.0, 100.0), "AA");
    assert!(font.text_width(&font.fit_text("AAAA", 1.0, 70.0), 1.0) <= 70.0);
}

#[test]
fn a_pack_with_the_wrong_magic_is_rejected() {
    let mut bytes = tiny_font_pack();
    bytes[0] = b'X';
    assert!(parse_font(&bytes).is_none());
}
