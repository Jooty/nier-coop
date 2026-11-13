use automata_game::downed::DownedSignal;
use automata_game::event_follow::EventId;
use automata_game::load_gate::{LoadSignal, StoryPlace};
use automata_game::partner::{BodySnapshot, PodAim, RecordedStart};
use serde::{Deserialize, Serialize};

use crate::wire::{MAX_FACING_ABS_RADIANS, MAX_POSITION_ABS, MAX_SPEED, MAX_STATE_ID, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub sequence: u32,
    pub held: u32,
    pub pressed: u32,
    pub speed: f32,
    pub world_facing: f32,
    pub position: [f32; 3],
    pub body_facing: f32,
    pub state: u16,
    pub last_start: Option<RecordedStart>,
    pub health: i32,
    pub max_health: i32,
    pub free: bool,
    pub phase_id: i32,
    pub paused: bool,
    pub jump_pending: bool,
    pub load: LoadSignal,
    pub pod_aim: Option<PodAim>,
    pub downed: DownedSignal,
    pub place: StoryPlace,
    pub playing_event: Option<EventId>,
}

impl PlayerState {
    pub fn from_snapshot(snapshot: &BodySnapshot, sequence: u32) -> Self {
        Self {
            sequence,
            held: snapshot.held,
            pressed: snapshot.pressed,
            speed: snapshot.speed,
            world_facing: snapshot.world_facing,
            position: snapshot.position,
            body_facing: snapshot.body_facing,
            state: snapshot.state,
            last_start: snapshot.last_start,
            health: snapshot.health,
            max_health: snapshot.max_health,
            free: snapshot.free,
            phase_id: snapshot.phase_id,
            paused: snapshot.paused,
            jump_pending: snapshot.jump_pending,
            load: LoadSignal::default(),
            pod_aim: snapshot.pod_aim,
            downed: snapshot.downed,
            place: StoryPlace::default(),
            playing_event: None,
        }
    }

    pub fn to_snapshot(&self) -> BodySnapshot {
        BodySnapshot {
            held: self.held,
            pressed: self.pressed,
            speed: self.speed,
            world_facing: self.world_facing,
            position: self.position,
            body_facing: self.body_facing,
            state: self.state,
            last_start: self.last_start,
            health: self.health,
            max_health: self.max_health,
            free: self.free,
            phase_id: self.phase_id,
            paused: self.paused,
            jump_pending: self.jump_pending,
            pod_aim: self.pod_aim,
            downed: self.downed,
        }
    }

    pub fn validate(&self) -> Result<(), WireError> {
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        if !is_facing(self.world_facing) {
            return Err(WireError::OutOfRange {
                field: "world_facing",
            });
        }
        if !is_facing(self.body_facing) {
            return Err(WireError::OutOfRange {
                field: "body_facing",
            });
        }
        if !(0.0..=MAX_SPEED).contains(&self.speed) {
            return Err(WireError::OutOfRange { field: "speed" });
        }
        if self.state > MAX_STATE_ID {
            return Err(WireError::OutOfRange { field: "state" });
        }
        if self
            .last_start
            .is_some_and(|start| start.ids.state > MAX_STATE_ID)
        {
            return Err(WireError::OutOfRange {
                field: "last_start",
            });
        }
        if self.pod_aim.is_some_and(|aim| aim.checked().is_none()) {
            return Err(WireError::OutOfRange { field: "pod_aim" });
        }
        if !self.downed.is_valid() {
            return Err(WireError::OutOfRange { field: "downed" });
        }
        if !self.place.name_is_valid() {
            return Err(WireError::OutOfRange { field: "place" });
        }
        if self.playing_event.is_some_and(|id| !id.is_valid()) {
            return Err(WireError::OutOfRange {
                field: "playing_event",
            });
        }
        Ok(())
    }
}

fn is_facing(radians: f32) -> bool {
    radians.is_finite() && radians.abs() <= MAX_FACING_ABS_RADIANS
}
