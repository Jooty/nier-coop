use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_APPLIED_DISPLACEMENT_OFFSET, ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET,
    ACTOR_OWN_VELOCITY_OFFSET, ACTOR_PENDING_DISPLACEMENT_OFFSET, ACTOR_TRANSFORM_BLOCK_OFFSET,
    ACTOR_VERTICAL_SPEED_OFFSET,
};

const POSITION_X_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0xC;
const POSITION_Z_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0x2C;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlideReading {
    pub ground_position: [f32; 2],
    pub pending_step: [f32; 3],
    pub pending_flag: u8,
    pub applied_step: [f32; 3],
    pub velocity: [f32; 3],
    pub upward_speed: f32,
}

pub fn read_slide_reading<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<SlideReading, GameError> {
    Ok(SlideReading {
        ground_position: [
            memory.read_f32(actor + POSITION_X_OFFSET)?,
            memory.read_f32(actor + POSITION_Z_OFFSET)?,
        ],
        pending_step: read_lanes(memory, actor + ACTOR_PENDING_DISPLACEMENT_OFFSET)?,
        pending_flag: memory.read_u8(actor + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET)?,
        applied_step: read_lanes(memory, actor + ACTOR_APPLIED_DISPLACEMENT_OFFSET)?,
        velocity: read_lanes(memory, actor + ACTOR_OWN_VELOCITY_OFFSET)?,
        upward_speed: memory.read_f32(actor + ACTOR_VERTICAL_SPEED_OFFSET)?,
    })
}

pub fn horizontal_distance(from: [f32; 2], to: [f32; 2]) -> f32 {
    (to[0] - from[0]).hypot(to[1] - from[1])
}

pub fn slide_check_text(
    kind: &str,
    actor: u64,
    moved_since_last_tick: Option<f32>,
    moved_in_tick: f32,
    before_cut: &SlideReading,
    after_cut: &SlideReading,
    motion: Option<(i32, f32)>,
) -> String {
    let moved =
        moved_since_last_tick.map_or_else(|| "n/a".to_owned(), |length| format!("{length:.3}"));
    let motion = motion.map_or_else(
        || "motion unreadable".to_owned(),
        |(index, frame)| format!("motion {index} frame {frame:.1}"),
    );
    format!(
        "slide check: {kind} copy {actor:#x} moved {moved} since last tick, {moved_in_tick:.3} in tick; \
         before cut step {} flag {} applied {} velocity {} upward {:.3}; \
         after cut step {} flag {} velocity {} upward {:.3}; {motion}",
        lanes_text(before_cut.pending_step),
        before_cut.pending_flag,
        lanes_text(before_cut.applied_step),
        lanes_text(before_cut.velocity),
        before_cut.upward_speed,
        lanes_text(after_cut.pending_step),
        after_cut.pending_flag,
        lanes_text(after_cut.velocity),
        after_cut.upward_speed,
    )
}

fn read_lanes<M: GameMemory>(memory: &M, address: u64) -> Result<[f32; 3], GameError> {
    Ok([
        memory.read_f32(address)?,
        memory.read_f32(address + 4)?,
        memory.read_f32(address + 8)?,
    ])
}

fn lanes_text(lanes: [f32; 3]) -> String {
    format!("({:.3} {:.3} {:.3})", lanes[0], lanes[1], lanes[2])
}
