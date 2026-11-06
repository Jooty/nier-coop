use rc_ui_style::{rgba_to_f32, rgba_with_alpha};

#[test]
fn channels_map_to_unit_range() {
    assert_eq!(rgba_to_f32([0, 0, 0, 0]), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(rgba_to_f32([255, 255, 255, 255]), [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn middle_channel_maps_proportionally() {
    let mapped = rgba_to_f32([51, 102, 153, 204]);
    for (value, expected) in mapped.iter().zip([0.2, 0.4, 0.6, 0.8]) {
        assert!((value - expected).abs() < 1e-6);
    }
}

#[test]
fn alpha_replacement_keeps_colour() {
    assert_eq!(rgba_with_alpha([1, 2, 3, 255], 9), [1, 2, 3, 9]);
}
