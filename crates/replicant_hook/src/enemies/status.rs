use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_proto::EnemyId;

use crate::enemies::CombatCounts;

static SNAP_ENABLED: AtomicBool = AtomicBool::new(true);
static STATUS: Mutex<EnemySyncStatus> = Mutex::new(EnemySyncStatus::idle("starting"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemySyncRole {
    Host,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnemySyncStatus {
    pub idle_reason: Option<&'static str>,
    pub role: Option<EnemySyncRole>,
    pub local_count: usize,
    pub host_total: Option<u16>,
    pub mirrored: usize,
    pub matched: usize,
    pub unmatched_local: Vec<EnemyId>,
    pub unmatched_remote: Vec<EnemyId>,
    pub worst_error: Option<(EnemyId, f32)>,
    pub sent_parts: usize,
    pub snapped: usize,
    pub snap_failures: usize,
    pub recent_events: VecDeque<String>,
    pub combat: CombatCounts,
}

impl EnemySyncStatus {
    pub const fn idle(reason: &'static str) -> Self {
        Self {
            idle_reason: Some(reason),
            role: None,
            local_count: 0,
            host_total: None,
            mirrored: 0,
            matched: 0,
            unmatched_local: Vec::new(),
            unmatched_remote: Vec::new(),
            worst_error: None,
            sent_parts: 0,
            snapped: 0,
            snap_failures: 0,
            recent_events: VecDeque::new(),
            combat: CombatCounts::zero(),
        }
    }
}

pub fn snap_enabled() -> bool {
    SNAP_ENABLED.load(Ordering::Relaxed)
}

pub fn set_snap_enabled(enabled: bool) {
    SNAP_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn enemy_sync_status() -> EnemySyncStatus {
    STATUS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub(super) fn publish_status(status: EnemySyncStatus) {
    *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = status;
}

pub fn enemy_id_text(id: EnemyId) -> String {
    format!("{}:{}", id.group, id.ordinal)
}
