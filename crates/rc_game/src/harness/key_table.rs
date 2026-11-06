const KEY_ALIASES: [(&str, &str); 6] = [
    ("esc", "escape"),
    ("return", "enter"),
    ("shift", "lshift"),
    ("ctrl", "lctrl"),
    ("alt", "lalt"),
    ("tilde", "grave"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCode {
    pub name: &'static str,
    pub direct_input_scan_code: u8,
    pub virtual_key: u8,
}

pub const KEY_CODES: [KeyCode; 70] = [
    KeyCode::new("a", 0x1E, 0x41),
    KeyCode::new("b", 0x30, 0x42),
    KeyCode::new("c", 0x2E, 0x43),
    KeyCode::new("d", 0x20, 0x44),
    KeyCode::new("e", 0x12, 0x45),
    KeyCode::new("f", 0x21, 0x46),
    KeyCode::new("g", 0x22, 0x47),
    KeyCode::new("h", 0x23, 0x48),
    KeyCode::new("i", 0x17, 0x49),
    KeyCode::new("j", 0x24, 0x4A),
    KeyCode::new("k", 0x25, 0x4B),
    KeyCode::new("l", 0x26, 0x4C),
    KeyCode::new("m", 0x32, 0x4D),
    KeyCode::new("n", 0x31, 0x4E),
    KeyCode::new("o", 0x18, 0x4F),
    KeyCode::new("p", 0x19, 0x50),
    KeyCode::new("q", 0x10, 0x51),
    KeyCode::new("r", 0x13, 0x52),
    KeyCode::new("s", 0x1F, 0x53),
    KeyCode::new("t", 0x14, 0x54),
    KeyCode::new("u", 0x16, 0x55),
    KeyCode::new("v", 0x2F, 0x56),
    KeyCode::new("w", 0x11, 0x57),
    KeyCode::new("x", 0x2D, 0x58),
    KeyCode::new("y", 0x15, 0x59),
    KeyCode::new("z", 0x2C, 0x5A),
    KeyCode::new("1", 0x02, 0x31),
    KeyCode::new("2", 0x03, 0x32),
    KeyCode::new("3", 0x04, 0x33),
    KeyCode::new("4", 0x05, 0x34),
    KeyCode::new("5", 0x06, 0x35),
    KeyCode::new("6", 0x07, 0x36),
    KeyCode::new("7", 0x08, 0x37),
    KeyCode::new("8", 0x09, 0x38),
    KeyCode::new("9", 0x0A, 0x39),
    KeyCode::new("0", 0x0B, 0x30),
    KeyCode::new("f1", 0x3B, 0x70),
    KeyCode::new("f2", 0x3C, 0x71),
    KeyCode::new("f3", 0x3D, 0x72),
    KeyCode::new("f4", 0x3E, 0x73),
    KeyCode::new("f5", 0x3F, 0x74),
    KeyCode::new("f6", 0x40, 0x75),
    KeyCode::new("f7", 0x41, 0x76),
    KeyCode::new("f8", 0x42, 0x77),
    KeyCode::new("f9", 0x43, 0x78),
    KeyCode::new("f10", 0x44, 0x79),
    KeyCode::new("f11", 0x57, 0x7A),
    KeyCode::new("f12", 0x58, 0x7B),
    KeyCode::new("enter", 0x1C, 0x0D),
    KeyCode::new("escape", 0x01, 0x1B),
    KeyCode::new("space", 0x39, 0x20),
    KeyCode::new("tab", 0x0F, 0x09),
    KeyCode::new("backspace", 0x0E, 0x08),
    KeyCode::new("lshift", 0x2A, 0xA0),
    KeyCode::new("rshift", 0x36, 0xA1),
    KeyCode::new("lctrl", 0x1D, 0xA2),
    KeyCode::new("rctrl", 0x9D, 0xA3),
    KeyCode::new("lalt", 0x38, 0xA4),
    KeyCode::new("ralt", 0xB8, 0xA5),
    KeyCode::new("up", 0xC8, 0x26),
    KeyCode::new("down", 0xD0, 0x28),
    KeyCode::new("left", 0xCB, 0x25),
    KeyCode::new("right", 0xCD, 0x27),
    KeyCode::new("insert", 0xD2, 0x2D),
    KeyCode::new("delete", 0xD3, 0x2E),
    KeyCode::new("home", 0xC7, 0x24),
    KeyCode::new("end", 0xCF, 0x23),
    KeyCode::new("pageup", 0xC9, 0x21),
    KeyCode::new("pagedown", 0xD1, 0x22),
    KeyCode::new("grave", 0x29, 0xC0),
];

impl KeyCode {
    const fn new(name: &'static str, direct_input_scan_code: u8, virtual_key: u8) -> Self {
        Self {
            name,
            direct_input_scan_code,
            virtual_key,
        }
    }
}

pub fn lookup_key(name: &str) -> Option<KeyCode> {
    let lowered = name.trim().to_ascii_lowercase();
    let canonical = KEY_ALIASES
        .iter()
        .find(|(alias, _)| *alias == lowered)
        .map_or(lowered.as_str(), |(_, canonical)| *canonical);
    KEY_CODES.iter().copied().find(|key| key.name == canonical)
}
