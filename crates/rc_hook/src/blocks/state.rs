use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use rc_game::blocks::{BlockOutbox, BlockRoster, StateTrail};

use crate::blocks::hold::LocalHold;

pub(super) struct BlockState {
    pub roster: BlockRoster,
    pub outbox: BlockOutbox,
    pub hold: LocalHold,
    pub idle_snapshots: HashMap<u16, Vec<u8>>,
    pub trail: Option<(u16, StateTrail)>,
    pub read_backs: Vec<(u16, [f32; 3], u64)>,
    pub last_scan_ms: Option<u64>,
}

pub(super) static STATE: LazyLock<Mutex<BlockState>> = LazyLock::new(|| {
    Mutex::new(BlockState {
        roster: BlockRoster::default(),
        outbox: BlockOutbox::default(),
        hold: LocalHold::default(),
        idle_snapshots: HashMap::new(),
        trail: None,
        read_backs: Vec::new(),
        last_scan_ms: None,
    })
});
