use crate::partner::{
    COMPUTER_CONTROLLED_FLAG_BITS, FLAG_BITS_CLEARED_ON_TAKEOVER, PLAYER_RUN_SPEED_TYPE,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyFields {
    pub flags: u32,
    pub player_controlled: i32,
    pub pad_enable: i32,
    pub script_enable: i32,
    pub computer_enable: i32,
    pub input_ignore_seconds: f32,
    pub run_speed_type: i32,
    pub dismissed: i32,
    pub health_floor_switch: i32,
}

impl BodyFields {
    /// What the tick writes while the other player drives this body; the flags keep their other bits.
    pub fn taken_over(self) -> Self {
        Self {
            flags: self.flags & !FLAG_BITS_CLEARED_ON_TAKEOVER,
            player_controlled: 1,
            pad_enable: 0,
            script_enable: 0,
            computer_enable: 0,
            input_ignore_seconds: 0.0,
            run_speed_type: PLAYER_RUN_SPEED_TYPE,
            dismissed: 0,
            health_floor_switch: 1,
        }
    }

    pub fn is_computer_controlled(self) -> bool {
        self.flags & COMPUTER_CONTROLLED_FLAG_BITS != 0
    }
}
