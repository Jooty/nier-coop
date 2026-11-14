use nt_ui_style::{
    BAD, BORDER, BUTTON, BUTTON_ACTIVE, BUTTON_HOVERED, DISABLED, FONT_GLYPH_RANGES, FONT_SIZE_PX,
    FRAME_ACTIVE, FRAME_BACKGROUND, FRAME_HOVERED, GOOD, INPUT_BACKGROUND,
    OVERLAY_WINDOW_BACKGROUND_ALPHA, ROUNDING_PX, STRIPE_BACKGROUND, TEXT, WARN, WINDOW_BACKGROUND,
    font_bytes, rgba_to_f32,
};

#[test]
fn launcher_palette_values_are_unchanged() {
    assert_eq!(WINDOW_BACKGROUND, [15, 15, 15, 255]);
    assert_eq!(FRAME_BACKGROUND, [41, 74, 122, 255]);
    assert_eq!(FRAME_HOVERED, [66, 150, 250, 255]);
    assert_eq!(BUTTON, [40, 96, 160, 255]);
    assert_eq!(BUTTON_HOVERED, [66, 150, 250, 255]);
    assert_eq!(BUTTON_ACTIVE, [15, 135, 250, 255]);
    assert_eq!(BORDER, [70, 70, 82, 255]);
    assert_eq!(TEXT, [235, 235, 235, 255]);
    assert_eq!(GOOD, [110, 220, 110, 255]);
    assert_eq!(BAD, [240, 100, 100, 255]);
    assert_eq!(WARN, [240, 200, 90, 255]);
    assert_eq!(INPUT_BACKGROUND, [10, 10, 10, 255]);
    assert_eq!(STRIPE_BACKGROUND, [24, 24, 24, 255]);
}

#[test]
fn every_colour_is_opaque() {
    let colours = [
        WINDOW_BACKGROUND,
        INPUT_BACKGROUND,
        STRIPE_BACKGROUND,
        FRAME_BACKGROUND,
        FRAME_HOVERED,
        FRAME_ACTIVE,
        BUTTON,
        BUTTON_HOVERED,
        BUTTON_ACTIVE,
        BORDER,
        TEXT,
        DISABLED,
        GOOD,
        BAD,
        WARN,
    ];
    assert!(colours.iter().all(|colour| colour[3] == 255));
}

#[test]
fn button_states_are_distinct_from_each_other() {
    assert_ne!(BUTTON, BUTTON_HOVERED);
    assert_ne!(BUTTON_HOVERED, BUTTON_ACTIVE);
    assert_ne!(BUTTON, BUTTON_ACTIVE);
}

#[test]
fn text_is_readable_on_the_window_background() {
    assert!(TEXT[0] > WINDOW_BACKGROUND[0] + 150);
    assert!(DISABLED[0] < TEXT[0]);
}

#[test]
fn overlay_alpha_is_about_three_quarters() {
    let alpha = rgba_to_f32([0, 0, 0, OVERLAY_WINDOW_BACKGROUND_ALPHA])[3];
    assert!((alpha - 0.75).abs() < 0.01);
}

#[test]
fn rounding_is_square_and_font_is_positive() {
    let sizes = [ROUNDING_PX, FONT_SIZE_PX];
    assert_eq!(sizes[0].to_bits(), 0.0_f32.to_bits());
    assert!(sizes[1].is_sign_positive());
}

#[test]
fn font_bytes_are_a_truetype_file() {
    let bytes = font_bytes();
    assert!(bytes.len() > 1000);
    assert!(bytes.starts_with(&[0, 1, 0, 0]) || bytes.starts_with(b"OTTO"));
}

#[test]
fn glyph_ranges_are_zero_terminated_pairs_in_order() {
    assert_eq!(FONT_GLYPH_RANGES.len() % 2, 1);
    assert_eq!(FONT_GLYPH_RANGES.last(), Some(&0));
    let ranges = &FONT_GLYPH_RANGES[..FONT_GLYPH_RANGES.len() - 1];
    for pair in ranges.chunks(2) {
        assert!(pair[0] <= pair[1]);
    }
    assert!(ranges.windows(2).all(|window| window[0] <= window[1]));
    assert_eq!(ranges[0], 0x20);
    assert!(ranges[1] >= 0xFF);
}
