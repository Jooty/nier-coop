use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use replicant_game::blocks::{
    BlockGlides, BlockOutbox, BlockRoster, HoldAnnouncer, PartnerHolds, StateTrail,
};

use crate::blocks::hold::LocalHold;

pub(super) struct BlockState {
    pub roster: BlockRoster,
    pub outbox: BlockOutbox,
    pub glides: BlockGlides,
    pub hold: LocalHold,
    pub idle_snapshots: HashMap<u16, Vec<u8>>,
    pub trail: Option<(u16, StateTrail)>,
    pub read_backs: Vec<(u16, [f32; 3], u64)>,
    pub last_scan_ms: Option<u64>,
    pub partner_holds: PartnerHolds,
    pub announcer: HoldAnnouncer,
    pub logged_local_hold: Option<u16>,
    pub logged_both_holding: HashSet<u16>,
}

pub(super) static STATE: LazyLock<Mutex<BlockState>> = LazyLock::new(|| {
    Mutex::new(BlockState {
        roster: BlockRoster::default(),
        outbox: BlockOutbox::default(),
        glides: BlockGlides::default(),
        hold: LocalHold::default(),
        idle_snapshots: HashMap::new(),
        trail: None,
        read_backs: Vec::new(),
        last_scan_ms: None,
        partner_holds: PartnerHolds::default(),
        announcer: HoldAnnouncer::default(),
        logged_local_hold: None,
        logged_both_holding: HashSet::new(),
    })
});
