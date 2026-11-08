use crate::game::Position;
use crate::view_follow::{SCENE_TELEPORT_JUMP_UNITS, SCENE_TELEPORT_WINDOW_MS};

#[derive(Debug, Default)]
pub struct SceneWatch {
    opened_at_ms: Option<u64>,
    last: Option<Position>,
}

impl SceneWatch {
    pub const fn new() -> Self {
        Self {
            opened_at_ms: None,
            last: None,
        }
    }

    pub fn open(&mut self, now_ms: u64) {
        self.opened_at_ms = Some(now_ms);
    }

    pub fn reset(&mut self) {
        self.opened_at_ms = None;
        self.last = None;
    }

    pub fn observe(&mut self, now_ms: u64, position: Position) -> bool {
        let before = self.last.replace(position);
        let Some(opened) = self.opened_at_ms else {
            return false;
        };
        if now_ms.saturating_sub(opened) > SCENE_TELEPORT_WINDOW_MS {
            self.opened_at_ms = None;
            return false;
        }
        let Some(before) = before else {
            return false;
        };
        let jumped = distance(before, position) >= SCENE_TELEPORT_JUMP_UNITS;
        if jumped {
            self.opened_at_ms = None;
        }
        jumped
    }
}

fn distance(from: Position, to: Position) -> f32 {
    let (dx, dy, dz) = (to.x - from.x, to.y - from.y, to.z - from.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}
