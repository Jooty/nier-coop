use crate::partner::BUTTON_WORD_COUNT;

/// The eight press timers after the presses of one controller update, as the game sets them:
/// a pressed button's word gets the timer start, and the first two buttons clear each other.
pub fn button_words_after_presses(
    words: [u32; BUTTON_WORD_COUNT],
    pressed: u32,
    timer_start: u32,
) -> [u32; BUTTON_WORD_COUNT] {
    let mut words = words;
    for index in 0..BUTTON_WORD_COUNT {
        if pressed & (1 << index) == 0 {
            continue;
        }
        words[index] = timer_start;
        match index {
            0 => words[1] = 0,
            1 => words[0] = 0,
            _ => {}
        }
    }
    words
}
