#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFileKind {
    Slot(u32),
    Game,
    System,
}

impl SaveFileKind {
    pub fn of_file_name(name: &str) -> Option<Self> {
        match name {
            "GameData.dat" => Some(Self::Game),
            "SystemData.dat" => Some(Self::System),
            _ => {
                let digits = name.strip_prefix("SlotData_")?.strip_suffix(".dat")?;
                if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                digits.parse().ok().map(Self::Slot)
            }
        }
    }
}
