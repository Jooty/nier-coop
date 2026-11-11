pub const NPC_EVENT_SCENE_SETTLE_MS: u64 = 3000;

pub fn npc_sync_settling(now_ms: u64, scene_ended_at_ms: Option<u64>) -> bool {
    scene_ended_at_ms.is_some_and(|ended| now_ms.saturating_sub(ended) < NPC_EVENT_SCENE_SETTLE_MS)
}

#[derive(Debug, Default)]
pub struct EventSceneClock {
    in_scene: bool,
    ended_at_ms: Option<u64>,
}

impl EventSceneClock {
    pub fn observe(&mut self, now_ms: u64, in_scene: bool) {
        if self.in_scene && !in_scene {
            self.ended_at_ms = Some(now_ms);
        }
        self.in_scene = in_scene;
    }

    pub fn settling(&self, now_ms: u64) -> bool {
        !self.in_scene && npc_sync_settling(now_ms, self.ended_at_ms)
    }
}
