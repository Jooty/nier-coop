use std::collections::BTreeMap;

use nc_net::PeerId;
use replicant_proto::{GameReadiness, Member, Message};

use crate::LinkStatus;

/// A player of this launcher's session other than the local one, as the session announced them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteMember {
    pub slot: u8,
    pub name: String,
    pub peer: PeerId,
}

/// Who else is in the session, by slot.
#[derive(Debug, Default)]
pub struct Members {
    by_slot: BTreeMap<u8, RemoteMember>,
}

impl Members {
    pub fn partner(&self) -> Option<&RemoteMember> {
        if self.by_slot.len() == 1 {
            return self.by_slot.values().next();
        }
        None
    }

    pub fn first(&self) -> Option<&RemoteMember> {
        self.by_slot.values().next()
    }

    pub fn all(&self) -> impl Iterator<Item = &RemoteMember> {
        self.by_slot.values()
    }

    pub fn count(&self) -> usize {
        self.by_slot.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_slot.is_empty()
    }

    pub fn admit(&mut self, slot: u8, name: String, peer: PeerId) -> bool {
        self.by_slot
            .insert(slot, RemoteMember { slot, name, peer })
            .is_none()
    }

    pub fn leave(&mut self, slot: u8) -> Option<RemoteMember> {
        self.by_slot.remove(&slot)
    }

    pub fn clear(&mut self) {
        self.by_slot.clear();
    }

    pub fn is_member(&self, slot: u8) -> bool {
        self.by_slot.contains_key(&slot)
    }

    pub fn addresses_a_member(&self, to: Option<u8>) -> bool {
        match to {
            None => !self.by_slot.is_empty(),
            Some(slot) => self.by_slot.contains_key(&slot),
        }
    }

    pub fn member_announcement(&self, slot: u8) -> Option<Message> {
        let member = self.by_slot.get(&slot)?;
        Some(Message::Member(Member {
            slot: member.slot,
            name: member.name.clone(),
        }))
    }

    pub fn write_partner_status(&self, status: &mut LinkStatus) {
        status.peer_name = self.partner().map(|partner| partner.name.clone());
        if !self.by_slot.is_empty() {
            return;
        }
        status.peer_readiness = GameReadiness::NotLinked;
        status.peer_lobby = None;
        status.peer_body = None;
        status.rtt = None;
        status.direct_path = None;
    }
}
