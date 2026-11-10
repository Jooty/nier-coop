use crate::start::CHAPTER_RECORD_SIZE;

const NAME_OFFSET: usize = 0xE;
const NAME_LEN: usize = 0x20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterRecord {
    pub chapter: u8,
    pub group: u8,
    pub route: i32,
    pub hero_mask: u8,
    pub phase_id: i16,
    pub phase_name: String,
}

impl ChapterRecord {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CHAPTER_RECORD_SIZE {
            return None;
        }
        let name_bytes = &bytes[NAME_OFFSET..NAME_OFFSET + NAME_LEN];
        let name_len = name_bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NAME_LEN);
        Some(Self {
            chapter: bytes[0],
            group: bytes[1],
            route: i32::from_le_bytes(bytes[4..8].try_into().ok()?),
            hero_mask: bytes[8],
            phase_id: i16::from_le_bytes(bytes[0xC..0xE].try_into().ok()?),
            phase_name: String::from_utf8_lossy(&name_bytes[..name_len]).into_owned(),
        })
    }
}
