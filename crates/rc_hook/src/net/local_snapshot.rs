use rc_game::puppet::LayerMotion;
use rc_proto::{
    MAX_HP, MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MotionState, PartnerActivity, PlayerState,
    WeaponState,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSnapshot {
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub body_motion: Option<LayerMotion>,
    pub weapon: Option<WeaponState>,
    pub activity: PartnerActivity,
}

impl LocalSnapshot {
    pub fn to_player_state(&self, tick: u32) -> PlayerState {
        let max_hp = u32::try_from(self.max_hp).unwrap_or(0).min(MAX_HP);
        let hp = u32::try_from(self.hp).unwrap_or(0).min(max_hp);
        PlayerState {
            tick,
            phase: self.phase,
            position: self.position,
            yaw_radians: self.yaw_radians,
            motion: self.body_motion.and_then(motion_state),
            hp,
            max_hp,
            weapon: self.weapon.filter(|weapon| weapon.validate().is_ok()),
            activity: self.activity,
        }
    }
}

fn motion_state(motion: LayerMotion) -> Option<MotionState> {
    if !motion.frame.is_finite() || !motion.speed.is_finite() {
        return None;
    }
    Some(MotionState {
        index: u16::try_from(motion.index).ok()?,
        frame: motion.frame.clamp(0.0, MAX_MOTION_FRAME),
        speed: motion
            .speed
            .clamp(-MAX_MOTION_SPEED_ABS, MAX_MOTION_SPEED_ABS),
        looping: motion.looping,
    })
}
