use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use replicant_game::blocks::{BlockGlides, BlockOutbox, HoldAnnouncer, PartnerHolds};
use replicant_game::world_objects::{CarryLog, HoldHow, WorldObjectRoster};
use replicant_proto::WorldObjectKey;

#[derive(Default)]
pub(super) struct ObjectState {
    pub roster: WorldObjectRoster,
    pub outbox: BlockOutbox,
    pub glides: BlockGlides,
    pub announcer: HoldAnnouncer,
    pub partner_holds: PartnerHolds,
    pub carry_log: CarryLog,
    pub refusal_log: CarryLog,
    pub last_scan_ms: Option<u64>,
    pub local_hold: Option<(u16, HoldHow)>,
    pub unlisted_handle_logged: Option<u64>,
    pub partner_last_heard: HashMap<u16, [f32; 3]>,
    pub refusal_answered_ms: HashMap<u16, u64>,
    pub pending_breaks: Vec<(WorldObjectKey, u64)>,
    pub moved_ids: HashSet<u16>,
    pub last_resend_ms: Option<u64>,
}

pub(super) static STATE: LazyLock<Mutex<ObjectState>> = LazyLock::new(Mutex::default);
