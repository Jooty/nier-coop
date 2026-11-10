use std::fmt;

use crate::start::{PlayReadings, RUNNING_SCENE_STATE, SCENE_LOADED_FLAG, is_in_play};
use crate::travel::{
    FAST_TRAVEL_IDLE_STATE, FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP,
    FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE, LoadedPoints, TravelPoint,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JumpRole {
    #[default]
    Solo,
    Host,
    Guest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldReadings {
    pub play: PlayReadings,
    pub fast_travel_state: Option<u32>,
    pub fast_travel_step: Option<i32>,
    pub loaded: LoadedPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpRefusal {
    NotHost,
    JumpUnderWay,
    SceneChangeUnderWay,
    NoWorldYet,
    TransportUnreadable,
    TransportBusy(u32),
    PointNotLoaded(TravelPoint),
}

impl JumpRole {
    pub fn of_link(connected: bool, local_slot: Option<u8>) -> Self {
        match (connected, local_slot) {
            (false, _) => Self::Solo,
            (true, Some(0)) => Self::Host,
            (true, _) => Self::Guest,
        }
    }
}

impl WorldReadings {
    /// True while the game's transport still counts up to the step that makes the jump.
    pub fn transport_is_stepping(&self) -> bool {
        self.fast_travel_step
            .is_some_and(|step| (0..=FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP).contains(&step))
    }
}

impl fmt::Display for JumpRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHost => write!(formatter, "only the host may jump"),
            Self::JumpUnderWay => write!(formatter, "refused while a jump is under way"),
            Self::SceneChangeUnderWay => {
                write!(formatter, "refused while a scene change is under way")
            }
            Self::NoWorldYet => write!(formatter, "no world yet"),
            Self::TransportUnreadable => {
                write!(formatter, "the game's transport state cannot be read")
            }
            Self::TransportBusy(state) => {
                write!(formatter, "the game's transport is busy (state {state:#X})")
            }
            Self::PointNotLoaded(point) => write!(
                formatter,
                "{} is not among the points the game holds now",
                point.code()
            ),
        }
    }
}

/// Why no jump at all can start now; `None` when the world and the game's transport are ready.
pub fn world_refusal(jump_under_way: bool, world: Option<&WorldReadings>) -> Option<JumpRefusal> {
    if jump_under_way {
        return Some(JumpRefusal::JumpUnderWay);
    }
    let Some(world) = world else {
        return Some(JumpRefusal::NoWorldYet);
    };
    if world.play.scene_state != RUNNING_SCENE_STATE || world.play.scene_loaded != SCENE_LOADED_FLAG
    {
        return Some(JumpRefusal::SceneChangeUnderWay);
    }
    if !is_in_play(&world.play) {
        return Some(JumpRefusal::NoWorldYet);
    }
    match world.fast_travel_state {
        None => Some(JumpRefusal::TransportUnreadable),
        Some(FAST_TRAVEL_IDLE_STATE | FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE) => None,
        Some(state) => Some(JumpRefusal::TransportBusy(state)),
    }
}

pub fn decide_local_jump(
    role: JumpRole,
    jump_under_way: bool,
    world: Option<&WorldReadings>,
    point: TravelPoint,
) -> Result<(), JumpRefusal> {
    if role == JumpRole::Guest {
        return Err(JumpRefusal::NotHost);
    }
    decide_followed_jump(jump_under_way, world, point)
}

pub fn decide_followed_jump(
    jump_under_way: bool,
    world: Option<&WorldReadings>,
    point: TravelPoint,
) -> Result<(), JumpRefusal> {
    if let Some(refusal) = world_refusal(jump_under_way, world) {
        return Err(refusal);
    }
    if world.is_some_and(|world| world.loaded.contains(point)) {
        Ok(())
    } else {
        Err(JumpRefusal::PointNotLoaded(point))
    }
}
