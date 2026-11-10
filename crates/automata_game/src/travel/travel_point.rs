use crate::travel::{TRAVEL_POINT_CODES, TRAVEL_POINT_LABEL_PREFIX};

/// One of the destination codes the exe names; the number is its place in `TRAVEL_POINT_CODES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TravelPoint(u8);

impl TravelPoint {
    pub fn from_index(index: u32) -> Option<Self> {
        let index = u8::try_from(index).ok()?;
        (usize::from(index) < TRAVEL_POINT_CODES.len()).then_some(Self(index))
    }

    pub fn from_code(code: &str) -> Option<Self> {
        let index = TRAVEL_POINT_CODES.iter().position(|known| *known == code)?;
        Self::from_index(u32::try_from(index).ok()?)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        (0..TRAVEL_POINT_CODES.len()).filter_map(|index| Self::from_index(index as u32))
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn code(self) -> &'static str {
        TRAVEL_POINT_CODES[usize::from(self.0)]
    }

    pub fn label(self) -> String {
        format!("{TRAVEL_POINT_LABEL_PREFIX}{}", self.code())
    }
}
