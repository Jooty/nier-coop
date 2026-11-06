use rc_proto::{MAX_ANIMATION_SECONDS, MAX_HP, PlayerState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSnapshot {
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub body_motion_index: Option<i32>,
    pub body_motion_frame: f32,
}

impl LocalSnapshot {
    pub fn to_player_state(&self, tick: u32) -> PlayerState {
        let max_hp = u32::try_from(self.max_hp).unwrap_or(0).min(MAX_HP);
        let hp = u32::try_from(self.hp).unwrap_or(0).min(max_hp);
        let animation_id = self
            .body_motion_index
            .and_then(|index| u16::try_from(index).ok())
            .unwrap_or(0);
        let frame = if self.body_motion_frame.is_finite() {
            self.body_motion_frame.clamp(0.0, MAX_ANIMATION_SECONDS)
        } else {
            0.0
        };
        PlayerState {
            tick,
            phase: self.phase,
            position: self.position,
            yaw_radians: self.yaw_radians,
            animation_id,
            animation_seconds: frame,
            hp,
            max_hp,
        }
    }
}
