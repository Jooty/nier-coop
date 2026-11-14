use automata_game::hud_look::{
    block_compressed_alpha, build_hud_font, dat_entries, dat_entry_with_extension, hud_palette,
    parse_font_table, parse_kerning_table, texture_spans, unpack_crilayla,
};

const RAW_PREFIX_BYTES: usize = 0x100;

fn packed_from_bits(bits: &[bool], unpacked_body: usize, prefix: &[u8]) -> Vec<u8> {
    // The unpacker reads the stream from its last byte, most significant bit first.
    let mut padded = bits.to_vec();
    while !padded.len().is_multiple_of(8) {
        padded.push(false);
    }
    let mut stream: Vec<u8> = padded
        .chunks(8)
        .map(|byte| {
            byte.iter()
                .fold(0_u8, |value, bit| (value << 1) | u8::from(*bit))
        })
        .collect();
    stream.reverse();
    let mut packed = b"CRILAYLA".to_vec();
    packed.extend_from_slice(&(unpacked_body as u32).to_le_bytes());
    packed.extend_from_slice(&(stream.len() as u32).to_le_bytes());
    packed.extend_from_slice(&stream);
    packed.extend_from_slice(prefix);
    packed
}

fn push_bits(bits: &mut Vec<bool>, value: usize, count: u32) {
    for bit in (0..count).rev() {
        bits.push((value >> bit) & 1 == 1);
    }
}

fn literal(bits: &mut Vec<bool>, byte: u8) {
    bits.push(false);
    push_bits(bits, usize::from(byte), 8);
}

fn container(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    const NAME_BYTES: usize = 16;
    let count = files.len();
    let offsets_at = 32;
    let names_at = offsets_at + count * 4;
    let sizes_at = names_at + 4 + count * NAME_BYTES;
    let mut data_at = sizes_at + count * 4;
    let mut out = b"DAT\0".to_vec();
    for value in [count, offsets_at, 0, names_at, sizes_at, 0, 0] {
        out.extend_from_slice(&(value as u32).to_le_bytes());
    }
    for (_, bytes) in files {
        out.extend_from_slice(&(data_at as u32).to_le_bytes());
        data_at += bytes.len();
    }
    out.extend_from_slice(&(NAME_BYTES as u32).to_le_bytes());
    for (name, _) in files {
        let mut padded = name.as_bytes().to_vec();
        padded.resize(NAME_BYTES, 0);
        out.extend_from_slice(&padded);
    }
    for (_, bytes) in files {
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    for (_, bytes) in files {
        out.extend_from_slice(bytes);
    }
    out
}

fn font_table(cell_height: u16, advance_adjust: i16, glyphs: &[(char, [u16; 5])]) -> Vec<u8> {
    let mut table = vec![0_u8; 0x88];
    table[..4].copy_from_slice(b"FTB ");
    table[0x6E..0x70].copy_from_slice(&cell_height.to_le_bytes());
    table[0x70..0x72].copy_from_slice(&(cell_height + 4).to_le_bytes());
    table[0x72..0x74].copy_from_slice(&advance_adjust.to_le_bytes());
    table[0x7A..0x7C].copy_from_slice(&(glyphs.len() as u16).to_le_bytes());
    table[0x80..0x84].copy_from_slice(&0x88_u32.to_le_bytes());
    for (character, cell) in glyphs {
        table.extend_from_slice(&(*character as u16).to_le_bytes());
        for value in cell {
            table.extend_from_slice(&value.to_le_bytes());
        }
    }
    table
}

fn texture_list(sizes: &[usize]) -> Vec<u8> {
    let mut list = b"WTB\0".to_vec();
    let offsets_at = 20;
    let sizes_at = offsets_at + sizes.len() * 4;
    for value in [1, sizes.len(), offsets_at, sizes_at] {
        list.extend_from_slice(&(value as u32).to_le_bytes());
    }
    let mut at = 0;
    for size in sizes {
        list.extend_from_slice(&(at as u32).to_le_bytes());
        at += size;
    }
    for size in sizes {
        list.extend_from_slice(&(*size as u32).to_le_bytes());
    }
    list
}

fn texture(width: u32, height: u32, alpha_of_block: impl Fn(u32, u32) -> [u8; 8]) -> Vec<u8> {
    let mut dds = vec![0_u8; 128];
    dds[..4].copy_from_slice(b"DDS ");
    dds[12..16].copy_from_slice(&height.to_le_bytes());
    dds[16..20].copy_from_slice(&width.to_le_bytes());
    dds[84..88].copy_from_slice(b"DXT5");
    for block_y in 0..height / 4 {
        for block_x in 0..width / 4 {
            dds.extend_from_slice(&alpha_of_block(block_x, block_y));
            dds.extend_from_slice(&[0_u8; 8]);
        }
    }
    dds
}

fn solid(alpha: u8) -> [u8; 8] {
    [alpha, alpha, 0, 0, 0, 0, 0, 0]
}

fn font_containers(advance_adjust: i16, kerning: &[(char, char, i16)]) -> (Vec<u8>, Vec<u8>) {
    // An 8 by 8 texture: the left half is inked, the right half is empty.
    let dds = texture(8, 8, |block_x, _| solid(if block_x == 0 { 255 } else { 0 }));
    let glyphs = [
        ('?', [0, 4, 8, 0, 0]),
        ('E', [0, 4, 8, 0, 0]),
        ('V', [0, 4, 8, 0, 0]),
        (' ', [0, 3, 8, 4, 0]),
        ('\u{4e00}', [0, 4, 8, 0, 0]),
    ];
    let mut pairs = (kerning.len() as u16).to_le_bytes().to_vec();
    for (left, right, adjust) in kerning {
        pairs.extend_from_slice(&(*left as u16).to_le_bytes());
        pairs.extend_from_slice(&(*right as u16).to_le_bytes());
        pairs.extend_from_slice(&adjust.to_le_bytes());
    }
    let tables = container(&[
        ("font.wta", texture_list(&[dds.len()])),
        ("font.ftb", font_table(8, advance_adjust, &glyphs)),
        ("font.ktb", pairs),
    ]);
    (tables, container(&[("font.wtp", dds)]))
}

fn layout_with_colors(colors: &[([f32; 3], usize)]) -> Vec<u8> {
    let mut layout = vec![0_u8; 8];
    for (color, uses) in colors {
        for _ in 0..*uses {
            for channel in color.iter().chain([1.0_f32].iter()) {
                layout.extend_from_slice(&channel.to_le_bytes());
            }
            layout.extend_from_slice(&[0_u8; 16]);
        }
    }
    layout
}

#[test]
fn plain_bytes_come_out_in_file_order_behind_the_raw_prefix() {
    let prefix = vec![7_u8; RAW_PREFIX_BYTES];
    let mut bits = Vec::new();
    for byte in *b"cba" {
        literal(&mut bits, byte);
    }
    let unpacked = unpack_crilayla(&packed_from_bits(&bits, 3, &prefix)).unwrap();
    assert_eq!(&unpacked[..RAW_PREFIX_BYTES], &prefix[..]);
    assert_eq!(&unpacked[RAW_PREFIX_BYTES..], b"abc");
}

#[test]
fn a_copy_repeats_bytes_already_written_behind_it() {
    let mut bits = Vec::new();
    for byte in *b"123" {
        literal(&mut bits, byte);
    }
    // offset code 0 is three bytes back, length code 1 is four bytes
    bits.push(true);
    push_bits(&mut bits, 0, 13);
    push_bits(&mut bits, 1, 2);
    let unpacked = unpack_crilayla(&packed_from_bits(&bits, 7, &[0_u8; RAW_PREFIX_BYTES])).unwrap();
    assert_eq!(&unpacked[RAW_PREFIX_BYTES..], b"1321321");
}

#[test]
fn bytes_that_are_not_a_whole_packed_file_are_refused() {
    assert_eq!(unpack_crilayla(b"DAT\0 not packed"), None);
    let mut bits = Vec::new();
    literal(&mut bits, b'a');
    let cut_short = packed_from_bits(&bits, 5, &[0_u8; RAW_PREFIX_BYTES]);
    assert_eq!(unpack_crilayla(&cut_short), None);
    let without_prefix = &cut_short[..cut_short.len() - 1];
    assert_eq!(unpack_crilayla(without_prefix), None);
}

#[test]
fn a_container_lists_its_files_by_name() {
    let bytes = container(&[("a.wta", vec![1, 2]), ("b.ftb", vec![3, 4, 5])]);
    let entries = dat_entries(&bytes).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].name, "b.ftb");
    assert_eq!(entries[1].bytes, &[3, 4, 5]);
    assert_eq!(
        dat_entry_with_extension(&bytes, ".wta"),
        Some(&[1_u8, 2][..])
    );
    assert_eq!(dat_entry_with_extension(&bytes, ".uid"), None);
    assert_eq!(dat_entries(b"CRILAYLA"), None);
    assert_eq!(dat_entries(&bytes[..bytes.len() - 1]), None);
}

#[test]
fn a_texture_list_gives_each_texture_its_place_in_the_data_file() {
    let spans = texture_spans(&texture_list(&[100, 40])).unwrap();
    assert_eq!((spans[0].offset, spans[0].size), (0, 100));
    assert_eq!((spans[1].offset, spans[1].size), (100, 40));
    assert_eq!(texture_spans(b"DAT\0"), None);
}

#[test]
fn a_font_table_gives_cell_sizes_and_the_advance_adjust() {
    let table = parse_font_table(&font_table(42, -7, &[('A', [2, 21, 42, 300, 500])])).unwrap();
    assert_eq!((table.cell_height, table.line_height), (42, 46));
    assert_eq!(table.advance_adjust, -7);
    let cell = table.glyphs[&('A' as u16)];
    assert_eq!((cell.texture, cell.width, cell.height), (2, 21, 42));
    assert_eq!((cell.left, cell.top), (300, 500));
    assert_eq!(parse_font_table(b"WTB\0"), None);
}

#[test]
fn a_kerning_table_is_keyed_by_the_pair() {
    let mut pairs = 1_u16.to_le_bytes().to_vec();
    for value in ['A' as u16, 'V' as u16, (-3_i16) as u16] {
        pairs.extend_from_slice(&value.to_le_bytes());
    }
    let kerning = parse_kerning_table(&pairs).unwrap();
    assert_eq!(kerning[&('A' as u16, 'V' as u16)], -3);
}

#[test]
fn texture_alpha_is_read_texel_by_texel_from_the_blocks() {
    // first 200, second 100, and a different index for texel 1 (second) and texel 4 (a blend)
    let indices: u64 = (1 << 3) | (2 << 12);
    let mut block = [200_u8, 100, 0, 0, 0, 0, 0, 0];
    block[2..8].copy_from_slice(&indices.to_le_bytes()[..6]);
    let dds = texture(4, 4, |_, _| block);
    let alpha = block_compressed_alpha(&dds, 0, 0, 4, 4).unwrap();
    assert_eq!(alpha[0], 200);
    assert_eq!(alpha[1], 100);
    assert_eq!(alpha[4], ((6 * 200 + 100) / 7) as u8);
    assert_eq!(block_compressed_alpha(&dds, 2, 0, 4, 4), None);
}

#[test]
fn alpha_with_the_smaller_value_first_has_the_two_fixed_ends() {
    let indices: u64 = 6 | (7 << 3);
    let mut block = [10_u8, 250, 0, 0, 0, 0, 0, 0];
    block[2..8].copy_from_slice(&indices.to_le_bytes()[..6]);
    let dds = texture(4, 4, |_, _| block);
    let alpha = block_compressed_alpha(&dds, 0, 0, 2, 1).unwrap();
    assert_eq!(alpha, vec![0, 255]);
}

#[test]
fn the_font_packs_latin_glyphs_only_and_keeps_their_ink() {
    let (tables, textures) = font_containers(0, &[]);
    let font = build_hud_font(&tables, &textures).unwrap();
    assert_eq!(font.glyph_count(), 4);
    assert_eq!(font.cell_height, 8.0);
    assert_eq!((font.ink_top, font.ink_bottom), (0.0, 8.0));
    let quads = font.layout_text("E", [10.0, 20.0], 2.0);
    assert_eq!(quads.len(), 1);
    assert_eq!(quads[0].min, [10.0, 20.0]);
    assert_eq!(quads[0].max, [18.0, 36.0]);
    let texel = |u: f32, v: f32| {
        let x = (u * font.texture_width as f32) as usize;
        let y = (v * font.texture_height as f32) as usize;
        font.coverage_rgba[(y * font.texture_width as usize + x) * 4 + 3]
    };
    assert_eq!(texel(quads[0].uv_min[0], quads[0].uv_min[1]), 255);
    let space = font.layout_text(" ", [0.0, 0.0], 1.0);
    assert_eq!(texel(space[0].uv_min[0], space[0].uv_min[1]), 0);
}

#[test]
fn letters_overlap_by_the_adjust_and_a_space_keeps_its_cell() {
    let (tables, textures) = font_containers(-1, &[]);
    let font = build_hud_font(&tables, &textures).unwrap();
    let quads = font.layout_text("E E", [0.0, 0.0], 1.0);
    let lefts: Vec<f32> = quads.iter().map(|quad| quad.min[0]).collect();
    assert_eq!(lefts, vec![0.0, 3.0, 6.0]);
    assert_eq!(font.text_width("E E", 1.0), 10.0);
    assert_eq!(font.text_width("E E", 2.0), 20.0);
}

#[test]
fn a_kerned_pair_moves_the_second_letter() {
    let (tables, textures) = font_containers(0, &[('E', 'V', -2)]);
    let font = build_hud_font(&tables, &textures).unwrap();
    assert_eq!(font.layout_text("EV", [0.0, 0.0], 1.0)[1].min[0], 2.0);
    assert_eq!(font.layout_text("VE", [0.0, 0.0], 1.0)[1].min[0], 4.0);
}

#[test]
fn a_character_the_font_does_not_pack_is_drawn_as_the_missing_glyph() {
    let (tables, textures) = font_containers(0, &[]);
    let font = build_hud_font(&tables, &textures).unwrap();
    assert_eq!(
        font.layout_text("\u{4e00}", [0.0, 0.0], 1.0),
        font.layout_text("?", [0.0, 0.0], 1.0)
    );
}

#[test]
fn a_font_whose_files_do_not_read_says_which_part() {
    let (tables, textures) = font_containers(0, &[]);
    assert!(build_hud_font(&textures, &textures).is_err());
    assert!(build_hud_font(&tables, &tables).is_err());
}

#[test]
fn the_palette_is_the_most_used_light_dark_and_warning_layer_colour() {
    let light = [0.8, 0.75, 0.7];
    let dark = [0.3, 0.3, 0.25];
    let accent = [0.9, 0.4, 0.3];
    let hud = container(&[
        (
            "hud0001.uid",
            layout_with_colors(&[(light, 20), (dark, 6), ([0.6, 0.7, 0.8], 9)]),
        ),
        (
            "hud0002.uid",
            layout_with_colors(&[(dark, 6), (accent, 10), ([0.1, 0.1, 0.9], 8)]),
        ),
        ("hud.wta", layout_with_colors(&[([0.2, 0.9, 0.2], 50)])),
    ]);
    let palette = hud_palette(&hud).unwrap();
    let close = |found: [f32; 3], wanted: [f32; 3]| {
        found
            .iter()
            .zip(wanted)
            .all(|(found, wanted)| (found - wanted).abs() < 0.003)
    };
    assert!(close(palette.light, light));
    assert!(close(palette.dark, dark));
    assert!(close(palette.accent, accent));
}

#[test]
fn greys_see_through_colours_and_rare_colours_are_not_the_palette() {
    let hud = container(&[(
        "hud0001.uid",
        layout_with_colors(&[([0.5, 0.5, 0.5], 40), ([0.8, 0.75, 0.7], 3)]),
    )]);
    assert_eq!(hud_palette(&hud), None);
    assert_eq!(hud_palette(b"not a container"), None);
}
