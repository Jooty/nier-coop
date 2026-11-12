use nc_net::PeerId;
use replicant_proto::{GameReadiness, Member, Message};

use crate::LinkStatus;

/// The other player in this launcher's session, as the session announced them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemoteMember {
    pub(crate) slot: u8,
    pub(crate) name: String,
    pub(crate) peer: PeerId,
}

/// Who else is in the session. A session holds two players, so there is at most one remote member.
#[derive(Debug, Default)]
pub(crate) struct Members {
    partner: Option<RemoteMember>,
}

impl Members {
    pub(crate) fn partner(&self) -> Option<&RemoteMember> {
        self.partner.as_ref()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.partner.is_none()
    }

    pub(crate) fn admit(&mut self, slot: u8, name: String, peer: PeerId) -> bool {
        if self
            .partner
            .as_ref()
            .is_some_and(|partner| partner.slot != slot)
        {
            return false;
        }
        self.partner = Some(RemoteMember { slot, name, peer });
        true
    }

    pub(crate) fn leave(&mut self, slot: u8) -> Option<RemoteMember> {
        if self.partner.as_ref()?.slot != slot {
            return None;
        }
        self.partner.take()
    }

    pub(crate) fn clear(&mut self) {
        self.partner = None;
    }

    pub(crate) fn is_member(&self, slot: u8) -> bool {
        self.partner
            .as_ref()
            .is_some_and(|partner| partner.slot == slot)
    }

    pub(crate) fn addresses_a_member(&self, to: Option<u8>) -> bool {
        match (to, self.partner.as_ref()) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(slot), Some(partner)) => slot == partner.slot,
        }
    }

    pub(crate) fn member_announcement(&self) -> Option<Message> {
        let partner = self.partner.as_ref()?;
        Some(Message::Member(Member {
            slot: partner.slot,
            name: partner.name.clone(),
        }))
    }

    pub(crate) fn write_partner_status(&self, status: &mut LinkStatus) {
        status.peer_name = self.partner.as_ref().map(|partner| partner.name.clone());
        if self.partner.is_some() {
            return;
        }
        status.peer_readiness = GameReadiness::NotLinked;
        status.peer_lobby = None;
        status.peer_body = None;
        status.rtt = None;
        status.direct_path = None;
    }
}
