use rc_game::party_frame::{
    AtlasRect, GaugeKind, PartyFrameTuning, PartyStatus, TUNING_FIELDS, decode_bc3_region,
    gauge_length_ratio, gauge_quads, health_fraction, parse_font, party_frame_layout,
    party_frame_visible, party_status,
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
fn layout_anchors_the_bars_right_end_and_scales_with_height() {
    let small = party_frame_layout(1920.0, 1080.0, &PartyFrameTuning::new());
    let large = party_frame_layout(3840.0, 2160.0, &PartyFrameTuning::new());
    assert!((small.life_anchor[0] - (1920.0 - 214.0 + 70.0)).abs() < 0.01);
    assert!((large.life_anchor[0] - (3840.0 - 2.0 * (214.0 - 70.0))).abs() < 0.01);
    assert!((large.bar_scale - 2.0 * small.bar_scale).abs() < 1e-6);
    assert_eq!(small.magic_anchor[0], small.life_anchor[0]);
    assert!((small.magic_anchor[1] - small.life_anchor[1] - (20.0 + 1.5)).abs() < 0.01);
    assert!(small.name_origin[0] > small.life_anchor[0]);
    assert!((small.name_origin[1] - small.own_name_origin[1] - 38.0).abs() < 0.01);
}

#[test]
fn the_life_gauge_matches_the_owners_bar_at_an_early_level() {
    let ratio = gauge_length_ratio(GaugeKind::Life, 1);
    let quads = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, ratio, 1.0);
    let length = quads.well_max[0] - quads.well_min[0];
    assert!((length - 120.0).abs() < 2.0, "length {length}");
}

#[test]
fn gauges_grow_with_level_and_are_full_at_stored_level_98() {
    for kind in [GaugeKind::Life, GaugeKind::Magic] {
        assert!(gauge_length_ratio(kind, 0) < gauge_length_ratio(kind, 50));
        assert_eq!(gauge_length_ratio(kind, 98), 1.0);
        assert_eq!(gauge_length_ratio(kind, 255), 1.0);
    }
    assert_eq!(gauge_length_ratio(GaugeKind::Life, 0), 0.24);
    assert_eq!(gauge_length_ratio(GaugeKind::Magic, 0), 0.2);
}

#[test]
fn a_longer_gauge_grows_to_the_left_from_the_same_right_end() {
    let short = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 0.25, 1.0);
    let long = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 1.0, 1.0);
    assert!((short.well_max[0] - 1000.0).abs() < 1e-3);
    assert!((long.well_max[0] - 1000.0).abs() < 1e-3);
    assert!((long.well_max[0] - long.well_min[0] - 480.0).abs() < 1e-3);
    assert!(long.base[0].min[0] < short.base[0].min[0]);
    assert!((short.base[2].max[0] - long.base[2].max[0]).abs() < 1e-3);
    assert!((short.well_min[1] - 103.0).abs() < 1e-3);
}

#[test]
fn the_base_slices_meet_and_keep_the_caps_unstretched() {
    let quads = gauge_quads(GaugeKind::Magic, [800.0, 40.0], 2.0, 0.3, 1.0);
    let [left, middle, right] = quads.base;
    assert_eq!(left.max[0], middle.min[0]);
    assert_eq!(middle.max[0], right.min[0]);
    assert!((left.max[0] - left.min[0] - 80.0).abs() < 1e-3);
    assert!((right.max[0] - right.min[0] - 2.0 * (598.0 - 510.0)).abs() < 1e-3);
    assert_eq!(right.uv_max[0], 1.0);
}

#[test]
fn the_fill_follows_health_inside_the_well() {
    let full = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 0.5, 1.0);
    let half = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 0.5, 0.5);
    let empty = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 0.5, 0.0);
    let full_fill = full.fill.unwrap();
    let half_fill = half.fill.unwrap();
    assert!((full_fill.max[0] - full.well_max[0]).abs() < 1e-3);
    assert_eq!(half_fill.max[0], half.well_max[0]);
    assert!((half_fill.max[0] - half_fill.min[0] - 120.0).abs() < 1e-3);
    assert!(half_fill.min[0] > half.well_min[0]);
    assert_eq!(half_fill.uv_max[0], 1.0);
    assert!(half_fill.uv_min[0] > 0.0);
    assert!(empty.fill.is_none());
}

#[test]
fn the_magic_bar_starts_at_its_anchor_and_is_thinner_than_the_life_fill() {
    let life = gauge_quads(GaugeKind::Life, [1000.0, 100.0], 1.0, 0.5, 1.0);
    let magic = gauge_quads(GaugeKind::Magic, [1000.0, 100.0], 1.0, 0.5, 1.0);
    let magic_fill = magic.fill.unwrap();
    assert!((magic_fill.min[1] - 100.0).abs() < 1e-3);
    assert!(magic_fill.max[1] - magic_fill.min[1] < life.well_max[1] - life.well_min[1]);
    assert!(GaugeKind::Life.has_brush_frame());
    assert!(!GaugeKind::Magic.has_brush_frame());
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
fn bc3_region_off_the_block_grid_is_cropped_from_the_blocks_it_touches() {
    let mut data = vec![0u8; 16 * 4];
    data[16] = 255;
    data[17] = 255;
    data[24..26].copy_from_slice(&0x07E0u16.to_le_bytes());
    data[26..28].copy_from_slice(&0x07E0u16.to_le_bytes());
    let rgba = decode_bc3_region(&data, 8, AtlasRect::new(3, 1, 6, 3)).unwrap();
    assert_eq!(rgba.len(), 3 * 2 * 4);
    assert_eq!(&rgba[..4], &[0, 0, 0, 0]);
    assert_eq!(&rgba[4..8], &[0, 255, 0, 255]);
}

#[test]
fn bc3_region_outside_the_data_is_rejected() {
    let data = vec![0u8; 16];
    assert!(decode_bc3_region(&data, 4, AtlasRect::new(0, 0, 4, 8)).is_none());
    assert!(decode_bc3_region(&data, 4, AtlasRect::new(0, 0, 8, 4)).is_none());
    assert!(decode_bc3_region(&data, 4, AtlasRect::new(2, 0, 2, 4)).is_none());
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

#[test]
fn tuning_starts_at_the_defaults_and_clamps_to_each_range() {
    let mut tuning = PartyFrameTuning::new();
    for (index, field) in TUNING_FIELDS.iter().enumerate() {
        assert_eq!(tuning.get(index), field.default);
    }
    tuning.set(2, 5000.0);
    assert_eq!(tuning.bar_scale(), TUNING_FIELDS[2].max);
    tuning.set(2, f32::NAN);
    assert_eq!(tuning.bar_scale(), TUNING_FIELDS[2].max);
}

#[test]
fn tuning_text_round_trips_and_ignores_unknown_lines() {
    let mut tuning = PartyFrameTuning::new();
    tuning.set_by_key("offset_x", -42.5);
    tuning.set_by_key("name_size", 21.0);
    let text = format!("{}junk\nnot_a_key = 3\n", tuning.as_text());
    assert_eq!(PartyFrameTuning::parse_text(&text), tuning);
}

#[test]
fn the_tuning_moves_the_frame_and_the_name_rows() {
    let base = party_frame_layout(1920.0, 1080.0, &PartyFrameTuning::new());
    let mut moved = PartyFrameTuning::new();
    moved.set_by_key("offset_x", 50.0);
    moved.set_by_key("name_row_gap", 40.0);
    let layout = party_frame_layout(1920.0, 1080.0, &moved);
    assert!((layout.life_anchor[0] - base.life_anchor[0] + 20.0).abs() < 0.01);
    assert!(
        (base.name_origin[1] - base.own_name_origin[1])
            < (layout.name_origin[1] - layout.own_name_origin[1])
    );
    assert!(base.magic_anchor[1] > base.life_anchor[1]);
}

#[test]
fn the_owners_tuned_values_are_the_defaults() {
    let tuning = PartyFrameTuning::new();
    assert_eq!(tuning.offset_x(), 70.0);
    assert_eq!(tuning.offset_y(), 1.0);
    assert_eq!(tuning.bar_gap(), 1.5);
    assert_eq!(tuning.name_offset_y(), -5.5);
    assert_eq!(tuning.name_row_gap(), 38.0);
}

#[test]
fn the_frame_is_hidden_in_a_cutscene_and_back_after() {
    assert!(party_frame_visible(true, false));
    assert!(!party_frame_visible(true, true));
    assert!(!party_frame_visible(false, false));
}
