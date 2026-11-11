use replicant_game::puppet::LayerMotion;
use replicant_proto::{
    MAX_HP, MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MotionState, PartnerActivity, PlayerState,
    RiddenBoar, WeaponState,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSnapshot {
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub body_motion: Option<LayerMotion>,
    pub body_motion_hash: u32,
    pub weapon: Option<WeaponState>,
    pub activity: PartnerActivity,
    pub mp: f32,
    pub max_mp: f32,
    pub level: i32,
    pub ridden_boar: Option<RiddenBoar>,
}

impl LocalSnapshot {
    pub fn to_player_state(&self, tick: u32) -> PlayerState {
        let max_hp = u32::try_from(self.max_hp).unwrap_or(0).min(MAX_HP);
        let hp = u32::try_from(self.hp).unwrap_or(0).min(max_hp);
        let max_mp = whole_magic(self.max_mp);
        let mp = whole_magic(self.mp).min(max_mp);
        PlayerState {
            tick,
            phase: self.phase,
            position: self.position,
            yaw_radians: self.yaw_radians,
            motion: self
                .body_motion
                .and_then(|motion| motion_state(motion, self.body_motion_hash)),
            hp,
            max_hp,
            weapon: self.weapon.filter(|weapon| weapon.validate().is_ok()),
            activity: self.activity,
            mp,
            max_mp,
            level: u8::try_from(self.level.clamp(0, i32::from(u8::MAX))).unwrap_or(0),
            ridden_boar: self.ridden_boar.filter(|boar| boar.validate().is_ok()),
        }
    }
}

fn motion_state(motion: LayerMotion, name_hash: u32) -> Option<MotionState> {
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
        name_hash,
    })
}

fn whole_magic(value: f32) -> u16 {
    if value.is_finite() {
        value.round().clamp(0.0, f32::from(u16::MAX)) as u16
    } else {
        0
    }
}
