use std::collections::HashMap;

use crate::blocks::PARTNER_HOLD_EXPIRY_MS;

#[derive(Debug, Default)]
pub struct PartnerHolds {
    phase: Option<u16>,
    confirmed_at: HashMap<u16, u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldChange {
    Took(u16),
    LetGo(u16),
    Unconfirmed(u16),
}

impl PartnerHolds {
    pub fn holds(&self, id: u16) -> bool {
        self.confirmed_at.contains_key(&id)
    }

    pub fn held_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self.confirmed_at.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    pub fn set_local_phase(&mut self, phase: u16) -> Vec<HoldChange> {
        if self.phase == Some(phase) {
            return Vec::new();
        }
        self.phase = Some(phase);
        self.clear()
    }

    pub fn partner_reports(
        &mut self,
        now_ms: u64,
        phase: u16,
        id: u16,
        held: bool,
    ) -> Option<HoldChange> {
        if self.phase != Some(phase) {
            return None;
        }
        if held {
            let known = self.confirmed_at.insert(id, now_ms).is_some();
            return (!known).then_some(HoldChange::Took(id));
        }
        self.confirmed_at.remove(&id).map(|_| HoldChange::LetGo(id))
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<HoldChange> {
        let mut gone: Vec<u16> = self
            .confirmed_at
            .iter()
            .filter(|(_, confirmed)| now_ms.saturating_sub(**confirmed) >= PARTNER_HOLD_EXPIRY_MS)
            .map(|(id, _)| *id)
            .collect();
        gone.sort_unstable();
        for id in &gone {
            self.confirmed_at.remove(id);
        }
        gone.into_iter().map(HoldChange::Unconfirmed).collect()
    }

    pub fn clear(&mut self) -> Vec<HoldChange> {
        let mut ids = self.held_ids();
        self.confirmed_at.clear();
        ids.drain(..).map(HoldChange::Unconfirmed).collect()
    }
}
