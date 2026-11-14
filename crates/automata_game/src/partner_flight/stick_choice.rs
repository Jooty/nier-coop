/// The three stick vectors of a controller block and the gates the flight movement picks one by.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StickChoice {
    pub pad_enable: i32,
    pub mouse_enable: i32,
    pub keyboard_enable: i32,
    pub mouse_length: f32,
    pub keyboard_length: f32,
    pub pad: [f32; 2],
    pub mouse: [f32; 2],
    pub keyboard: [f32; 2],
}

/// The vector the game's flight movement reads: the mouse one first, then the keyboard one, then
/// the pad one, each only while its gate is open; none of them gives no stick.
pub fn flight_stick_in_use(choice: &StickChoice) -> [f32; 2] {
    if choice.mouse_enable != 0 && choice.mouse_length > 0.0 {
        choice.mouse
    } else if choice.keyboard_enable != 0 && choice.keyboard_length > 0.0 {
        choice.keyboard
    } else if choice.pad_enable != 0 {
        choice.pad
    } else {
        [0.0; 2]
    }
}
