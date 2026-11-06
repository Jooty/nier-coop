const CHANNEL_MAX: f32 = 255.0;

pub fn rgba_to_f32(color: [u8; 4]) -> [f32; 4] {
    color.map(|channel| f32::from(channel) / CHANNEL_MAX)
}

pub fn rgba_with_alpha(color: [u8; 4], alpha: u8) -> [u8; 4] {
    [color[0], color[1], color[2], alpha]
}
