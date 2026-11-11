use std::collections::HashMap;

use crate::blocks::{
    BLOCK_GLIDE_LANDED_DISTANCE, BLOCK_GLIDE_LOG_INTERVAL_MS, BLOCK_GLIDE_MAX_FRAME_MS,
    BLOCK_GLIDE_SNAP_DISTANCE, BLOCK_GLIDE_TIME_CONSTANT_MS, distance_between,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlideStep {
    pub id: u16,
    pub to: [f32; 3],
    pub landed: bool,
    pub log_due: bool,
}

#[derive(Debug)]
struct Glide {
    target: [f32; 3],
    last_logged_ms: u64,
}

#[derive(Debug, Default)]
pub struct BlockGlides {
    glides: HashMap<u16, Glide>,
    last_advance_ms: Option<u64>,
}

impl BlockGlides {
    pub fn set_target(&mut self, now_ms: u64, id: u16, target: [f32; 3]) {
        match self.glides.get_mut(&id) {
            Some(glide) => glide.target = target,
            None => {
                self.glides.insert(
                    id,
                    Glide {
                        target,
                        last_logged_ms: now_ms,
                    },
                );
            }
        }
    }

    pub fn target_of(&self, id: u16) -> Option<[f32; 3]> {
        self.glides.get(&id).map(|glide| glide.target)
    }

    pub fn advance(
        &mut self,
        now_ms: u64,
        position_of: impl Fn(u16) -> Option<[f32; 3]>,
    ) -> Vec<GlideStep> {
        let frame_ms = self
            .last_advance_ms
            .map_or(BLOCK_GLIDE_MAX_FRAME_MS / 4, |last| {
                now_ms.saturating_sub(last)
            })
            .clamp(1, BLOCK_GLIDE_MAX_FRAME_MS);
        self.last_advance_ms = Some(now_ms);
        let mut steps = Vec::new();
        self.glides.retain(|id, glide| {
            let Some(current) = position_of(*id) else {
                return false;
            };
            let to = glide_toward(current, glide.target, frame_ms);
            let landed = to == glide.target;
            let log_due =
                now_ms.saturating_sub(glide.last_logged_ms) >= BLOCK_GLIDE_LOG_INTERVAL_MS;
            if log_due {
                glide.last_logged_ms = now_ms;
            }
            steps.push(GlideStep {
                id: *id,
                to,
                landed,
                log_due,
            });
            !landed
        });
        steps.sort_by_key(|step| step.id);
        steps
    }
}

pub fn glide_toward(current: [f32; 3], target: [f32; 3], frame_ms: u64) -> [f32; 3] {
    let remaining = distance_between(current, target);
    if remaining <= BLOCK_GLIDE_LANDED_DISTANCE || remaining >= BLOCK_GLIDE_SNAP_DISTANCE {
        return target;
    }
    let share = 1.0 - (-(frame_ms as f32) / BLOCK_GLIDE_TIME_CONSTANT_MS).exp();
    std::array::from_fn(|axis| current[axis] + (target[axis] - current[axis]) * share)
}
