use rc_game::harness::{BackbufferFormat, pixels_to_rgb};

#[test]
fn bgra_pixels_come_out_as_rgb() {
    let rows = [10, 20, 30, 255];
    assert_eq!(
        pixels_to_rgb(BackbufferFormat::Bgra8, &rows, 1, 1, 4, 1),
        [30, 20, 10]
    );
}

#[test]
fn row_padding_is_skipped() {
    let rows = [1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8, 9, 9, 9, 9];
    let rgb = pixels_to_rgb(BackbufferFormat::Rgba8, &rows, 1, 2, 8, 1);
    assert_eq!(rgb, [1, 2, 3, 5, 6, 7]);
}

#[test]
fn downscale_keeps_every_nth_pixel() {
    let rows = [1, 1, 1, 0, 2, 2, 2, 0, 3, 3, 3, 0, 4, 4, 4, 0];
    let rgb = pixels_to_rgb(BackbufferFormat::Rgba8, &rows, 4, 1, 16, 2);
    assert_eq!(rgb, [1, 1, 1, 3, 3, 3]);
}

#[test]
fn ten_bit_white_is_full_scale() {
    let packed: u32 = 0x3FF | (0x3FF << 10) | (0x3FF << 20);
    let rgb = pixels_to_rgb(BackbufferFormat::Rgb10A2, &packed.to_le_bytes(), 1, 1, 4, 1);
    assert_eq!(rgb, [255, 255, 255]);
}

#[test]
fn dxgi_bgra_is_recognised_and_unknown_is_not() {
    assert_eq!(
        BackbufferFormat::from_dxgi(87),
        Some(BackbufferFormat::Bgra8)
    );
    assert_eq!(BackbufferFormat::from_dxgi(2), None);
}
