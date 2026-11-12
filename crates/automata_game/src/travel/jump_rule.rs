use std::fmt;

use crate::mode::SessionMode;
use crate::start::{PlayReadings, RUNNING_SCENE_STATE, SCENE_LOADED_FLAG, is_in_play};
use crate::travel::{
    FAST_TRAVEL_IDLE_STATE, FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP,
    FAST_TRAVEL_STATE_THE_TRAVEL_CALL_RESETS_TO_IDLE, LoadedPoints, PAUSED_GAME_VALUES,
    TravelPoint,
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
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpRefusal {
    NotHost,
    JumpUnderWay,
    SceneChangeUnderWay,
    GamePaused,
    NoWorldYet,
    TransportUnreadable,
    TransportBusy(u32),
    TransportStepping(i32),
    PointNotLoaded(TravelPoint),
}

impl JumpRole {
    /// Only the campaign has a host and a guest of a jump; in free roam every player jumps on its own.
    pub fn of_session(mode: SessionMode, connected: bool, local_slot: Option<u8>) -> Self {
        if mode.story_rules_apply() {
            Self::of_link(connected, local_slot)
        } else {
            Self::Solo
        }
    }

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
        self.stepping_step().is_some()
    }

    /// The step counter when it reads one of the steps the game's update still acts on.
    pub fn stepping_step(&self) -> Option<i32> {
        self.fast_travel_step
            .filter(|step| (0..=FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP).contains(step))
    }

    /// True when the counter was read and rests outside the steps: nothing more will come.
    pub fn transport_left_its_steps(&self) -> bool {
        self.fast_travel_step.is_some() && !self.transport_is_stepping()
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
            Self::GamePaused => write!(
                formatter,
                "the game is paused, the jump waits for the pause to end"
            ),
            Self::NoWorldYet => write!(formatter, "no world yet"),
            Self::TransportUnreadable => {
                write!(formatter, "the game's transport state cannot be read")
            }
            Self::TransportBusy(state) => {
                write!(formatter, "the game's transport is busy (state {state:#X})")
            }
            Self::TransportStepping(step) => write!(
                formatter,
                "the game's transport is in the middle of a jump of its own (step {step})"
            ),
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
    if world.paused {
        return Some(JumpRefusal::GamePaused);
    }
    if let Some(step) = world.stepping_step() {
        return Some(JumpRefusal::TransportStepping(step));
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

pub fn game_is_paused(pause_value: Option<i32>) -> bool {
    pause_value.is_some_and(|value| PAUSED_GAME_VALUES.contains(&value))
}
