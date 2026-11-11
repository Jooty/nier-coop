use std::f32::consts::{PI, TAU};

use replicant_proto::{
    MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MotionState, NO_MOTION_NAME_HASH, RiddenBoar,
};

use crate::game::{GameError, Position};
use crate::memory::GameMemory;
use crate::npcs::read_npc_body_layer;
use crate::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    PLAYER_ACTION_STATE_OFFSET, RIDDEN_BOAR_CLASS_ID, RIDE_BOAR_STATE_BOAR_HANDLE_OFFSET,
    RIDE_BOAR_STATE_VTABLE_RVA,
};
use crate::puppet::{LayerMotion, PuppetPose, actor_is_live, actor_of_handle};

pub const RIDDEN_BOAR_PACK_SETTLE_MS: u64 = 1500;
pub const RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS: u64 = 1000;

/// Whether the boar's model has stayed loaded long enough to make a boar that wears it.
#[derive(Debug, Clone, Copy, Default)]
pub struct BoarPackSettle {
    found_since_ms: Option<u64>,
    looked_up_at_ms: Option<u64>,
}

impl BoarPackSettle {
    /// Whether to look the model up now: every tick while the partner rides, so the boar is
    /// made at once, and only now and then otherwise, so its settle time is already over.
    pub fn lookup_due(&mut self, partner_rides: bool, now_ms: u64) -> bool {
        let due = partner_rides
            || self
                .looked_up_at_ms
                .is_none_or(|at| now_ms.saturating_sub(at) >= RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS);
        if due {
            self.looked_up_at_ms = Some(now_ms);
        }
        due
    }

    pub fn settled(&mut self, pack_found: bool, loading: bool, now_ms: u64) -> bool {
        if !pack_found || loading {
            self.found_since_ms = None;
            return false;
        }
        let since = *self.found_since_ms.get_or_insert(now_ms);
        now_ms.saturating_sub(since) >= RIDDEN_BOAR_PACK_SETTLE_MS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoarStep {
    Nothing,
    Remove,
    Wait,
    Make,
    Follow,
}

/// What the copy does this tick with the boar under the partner's body. A boar is made only
/// while the partner rides, the model has settled and the game is not closing.
pub fn boar_step(partner_rides: bool, shown: bool, model_settled: bool, closing: bool) -> BoarStep {
    match (partner_rides, shown) {
        (false, false) => BoarStep::Nothing,
        (false, true) => BoarStep::Remove,
        (true, true) => BoarStep::Follow,
        (true, false) if model_settled && !closing => BoarStep::Make,
        (true, false) => BoarStep::Wait,
    }
}

/// The boar the player rides (getting on and off included), or None. It is the game's own test:
/// the player's action state is the ride state, and that state's handle names a live boar.
pub fn read_ridden_boar_actor<M: GameMemory>(
    memory: &M,
    player: u64,
) -> Result<Option<u64>, GameError> {
    let state = memory.read_u64(player + PLAYER_ACTION_STATE_OFFSET)?;
    if state == 0 {
        return Ok(None);
    }
    let ride_state_vtable = memory.module_base() + u64::from(RIDE_BOAR_STATE_VTABLE_RVA);
    if memory.read_u64(state)? != ride_state_vtable {
        return Ok(None);
    }
    let handle = memory.read_array::<8>(state + RIDE_BOAR_STATE_BOAR_HANDLE_OFFSET)?;
    let index = u32::from_le_bytes([handle[0], handle[1], handle[2], handle[3]]);
    let generation = u32::from_le_bytes([handle[4], handle[5], handle[6], handle[7]]);
    let Some(boar) = actor_of_handle(memory, index, generation)? else {
        return Ok(None);
    };
    let is_boar = actor_is_live(memory, boar)?
        && memory.read_i32(boar + ACTOR_CLASS_ID_OFFSET)? == RIDDEN_BOAR_CLASS_ID;
    Ok(is_boar.then_some(boar))
}

pub fn read_ridden_boar<M: GameMemory>(
    memory: &M,
    player: u64,
) -> Result<Option<RiddenBoar>, GameError> {
    let Some(boar) = read_ridden_boar_actor(memory, player)? else {
        return Ok(None);
    };
    let pose_of = |actor: u64| -> Result<PuppetPose, GameError> {
        let block = memory
            .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)?;
        Ok(PuppetPose::from_transform_block(&block))
    };
    let (offset_from_rider, yaw_from_rider_radians) =
        boar_seen_from_rider(pose_of(player)?, pose_of(boar)?);
    let motion = read_npc_body_layer(memory, boar)
        .ok()
        .flatten()
        .and_then(|layer| motion_state(layer.motion));
    Ok(Some(RiddenBoar {
        offset_from_rider,
        yaw_from_rider_radians,
        motion,
    }))
}

/// Where the boar stands in the rider's own frame: the offset turned back by the rider's yaw, and
/// the boar's yaw counted from the rider's.
pub fn boar_seen_from_rider(rider: PuppetPose, boar: PuppetPose) -> ([f32; 3], f32) {
    let (sin, cos) = rider.yaw.sin_cos();
    let x = boar.position.x - rider.position.x;
    let z = boar.position.z - rider.position.z;
    (
        [
            cos * x + sin * z,
            boar.position.y - rider.position.y,
            cos * z - sin * x,
        ],
        wrapped_angle(boar.yaw - rider.yaw),
    )
}

/// The inverse of [`boar_seen_from_rider`]: the boar's world pose under the partner's body.
pub fn boar_pose_under_rider(rider: PuppetPose, boar: &RiddenBoar) -> PuppetPose {
    let (sin, cos) = rider.yaw.sin_cos();
    let [x, y, z] = boar.offset_from_rider;
    PuppetPose {
        position: Position {
            x: rider.position.x + cos * x - sin * z,
            y: rider.position.y + y,
            z: rider.position.z + sin * x + cos * z,
        },
        yaw: wrapped_angle(rider.yaw + boar.yaw_from_rider_radians),
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
        name_hash: NO_MOTION_NAME_HASH,
    })
}

fn wrapped_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}
