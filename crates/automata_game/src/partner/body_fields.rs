use crate::partner::{
    COMPUTER_CONTROLLED_FLAG_BITS, DISMISSED_BY_THE_ENGINE, FLAG_BITS_CLEARED_ON_TAKEOVER,
    FOLLOWING_PARTNER_RUN_SPEED_TYPE, PLAYER_RUN_SPEED_TYPE,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GiveBack {
    pub made_by_mod: bool,
    pub mark_dismissed: bool,
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
        }
    }

    /// What is written when the body goes back to the game; `self` is what was saved at takeover.
    pub fn given_back(self, current: BodyFields, give_back: GiveBack) -> Self {
        Self {
            // The game's own partner follows the hero only with this run speed type.
            run_speed_type: if give_back.made_by_mod {
                FOLLOWING_PARTNER_RUN_SPEED_TYPE
            } else {
                self.run_speed_type
            },
            // A dismissal the engine started stays as the engine set it.
            dismissed: if give_back.mark_dismissed {
                DISMISSED_BY_THE_ENGINE
            } else {
                current.dismissed
            },
            ..self
        }
    }

    pub fn is_computer_controlled(self) -> bool {
        self.flags & COMPUTER_CONTROLLED_FLAG_BITS != 0
    }
}

/// False when the held body is the one the local player controls now: after the game's own
/// switch to the partner, the saved partner fields would freeze the player.
pub fn gives_fields_back(held_body_handle: u32, possessed_handle: Option<u32>) -> bool {
    possessed_handle != Some(held_body_handle)
}
