use automata_game::enemy_lead::{
    ENEMY_FACING_OFFSET, ENEMY_POSITION_OFFSET, ENEMY_STATE_OFFSET, ENEMY_TARGET_HANDLE_OFFSET,
};
use automata_game::partner::STATE_START_VTABLE_OFFSET;

use crate::game_calls::GameCalls;

type EnemyStateStart = unsafe extern "system" fn(usize, i32, u32, u32, u32);

/// What the host streams of one enemy, read from its body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyPoseReading {
    pub position: [f32; 3],
    pub facing: f32,
    pub state: [i32; 4],
    pub target: u32,
}

impl GameCalls {
    pub fn read_enemy_pose(&self, body: usize) -> Option<EnemyPoseReading> {
        let at = body as u64;
        let position = self
            .read_f32s_at::<3>(at + u64::from(ENEMY_POSITION_OFFSET))
            .ok()?;
        let state = self
            .read_u32s_at::<4>(at + u64::from(ENEMY_STATE_OFFSET))
            .ok()?;
        Some(EnemyPoseReading {
            position,
            facing: self.read_f32_at(at + u64::from(ENEMY_FACING_OFFSET)).ok()?,
            state: state.map(|value| value as i32),
            target: self
                .read_u32_at(at + u64::from(ENEMY_TARGET_HANDLE_OFFSET))
                .ok()?,
        })
    }

    pub fn write_enemy_place(&self, body: usize, position: [f32; 3]) -> bool {
        self.write_f32s_at(body as u64 + u64::from(ENEMY_POSITION_OFFSET), &position)
            .is_ok()
    }

    pub fn write_enemy_facing(&self, body: usize, facing: f32) -> bool {
        self.write_f32_at(body as u64 + u64::from(ENEMY_FACING_OFFSET), facing)
            .is_ok()
    }

    pub fn write_enemy_target(&self, body: usize, handle: u32) -> bool {
        self.write_u32_at(body as u64 + u64::from(ENEMY_TARGET_HANDLE_OFFSET), handle)
            .is_ok()
    }

    /// Starts a state on an enemy through slot 18 of its own vtable, with the four ints the
    /// game's base state change writes to +0x670; false when the vtable could not be read.
    /// # Safety
    /// `body` must be a live enemy resolved from its handle this frame; game thread only.
    #[expect(
        unsafe_code,
        reason = "calls the enemy's state start through its vtable"
    )]
    pub unsafe fn start_enemy_state(&self, body: usize, state: [i32; 4]) -> bool {
        let Ok(start) = self
            .read_u64_at(body as u64)
            .and_then(|vtable| self.read_u64_at(vtable + u64::from(STATE_START_VTABLE_OFFSET)))
        else {
            return false;
        };
        if start == 0 {
            return false;
        }
        // SAFETY: slot 18 of an enemy's vtable takes the enemy, the state id and three ints
        // (the last on the stack), by the caller's contract on a live enemy.
        unsafe {
            core::mem::transmute::<usize, EnemyStateStart>(start as usize)(
                body,
                state[0],
                state[1] as u32,
                state[2] as u32,
                state[3] as u32,
            );
        }
        true
    }
}
