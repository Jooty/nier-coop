use replicant_proto::PartnerBody;

use crate::puppet::BodyPackLoad;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnResult {
    Created(u32),
    AlreadyAlive { id: u32, body: PartnerBody },
}

#[derive(Debug, Clone, Copy)]
pub struct GhostBook {
    pub puppet_id: Option<u32>,
    pub puppet_body: Option<PartnerBody>,
    pub pack_body: Option<PartnerBody>,
    pub pack_load: BodyPackLoad,
}

impl GhostBook {
    pub const fn new() -> Self {
        Self {
            puppet_id: None,
            puppet_body: None,
            pack_body: None,
            pack_load: BodyPackLoad::new(),
        }
    }

    pub fn forget_puppet(&mut self) {
        *self = Self::new();
    }

    pub fn reset_pack_state(&mut self) {
        self.pack_body = None;
        self.pack_load = BodyPackLoad::new();
    }

    pub fn note_spawn(&mut self, wanted: PartnerBody, result: SpawnResult) {
        let (id, body) = match result {
            SpawnResult::Created(id) => (id, wanted),
            SpawnResult::AlreadyAlive { id, body } => (id, body),
        };
        self.puppet_id = Some(id);
        self.puppet_body = Some(body);
        self.reset_pack_state();
    }

    pub fn note_respawn_request(&mut self, despawn_failed: bool) {
        self.puppet_body = None;
        self.reset_pack_state();
        if !despawn_failed {
            self.puppet_id = None;
        }
    }
}

impl Default for GhostBook {
    fn default() -> Self {
        Self::new()
    }
}
