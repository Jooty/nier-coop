pub const NPC_EVENT_SCENE_SETTLE_MS: u64 = 3000;

pub fn npc_sync_settling(now_ms: u64, scene_ended_at_ms: Option<u64>) -> bool {
    scene_ended_at_ms.is_some_and(|ended| now_ms.saturating_sub(ended) < NPC_EVENT_SCENE_SETTLE_MS)
}
