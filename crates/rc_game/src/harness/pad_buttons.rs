const PAD_BUTTON_BITS: [(&str, u16); 14] = [
    ("dpad_up", 0x0001),
    ("dpad_down", 0x0002),
    ("dpad_left", 0x0004),
    ("dpad_right", 0x0008),
    ("start", 0x0010),
    ("back", 0x0020),
    ("left_thumb", 0x0040),
    ("right_thumb", 0x0080),
    ("left_shoulder", 0x0100),
    ("right_shoulder", 0x0200),
    ("a", 0x1000),
    ("b", 0x2000),
    ("x", 0x4000),
    ("y", 0x8000),
];

pub fn pad_button_bit(name: &str) -> Option<u16> {
    let lowered = name.trim().to_ascii_lowercase();
    PAD_BUTTON_BITS
        .iter()
        .find(|(button, _)| *button == lowered)
        .map(|(_, bit)| *bit)
}

pub fn pad_button_names() -> Vec<&'static str> {
    PAD_BUTTON_BITS.iter().map(|(name, _)| *name).collect()
}
