use crate::partner::{BUTTON_WORD_COUNT, PRESSED_BUTTON_WORD_VALUE};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodySnapshot {
    pub held: u32,
    pub pressed: u32,
    pub button_mask: u8,
    pub speed: f32,
    pub world_facing: f32,
    pub position: [f32; 3],
    pub body_facing: f32,
    pub state: u16,
    pub variant: u32,
    pub b: u32,
    pub c: u32,
    pub health: i32,
}

pub fn button_mask_of(words: [u32; BUTTON_WORD_COUNT]) -> u8 {
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| **word != 0)
        .fold(0, |mask, (index, _)| mask | (1 << index))
}

pub fn button_words_of(mask: u8) -> [u32; BUTTON_WORD_COUNT] {
    core::array::from_fn(|index| {
        if mask & (1 << index) != 0 {
            PRESSED_BUTTON_WORD_VALUE
        } else {
            0
        }
    })
}

pub fn state_id_for_wire(state: i32) -> u16 {
    u16::try_from(state).unwrap_or(0)
}
