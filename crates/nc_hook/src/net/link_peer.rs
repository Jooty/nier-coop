use nc_net::{Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, PeerId};

#[derive(Debug, Clone)]
pub struct LinkPeer {
    pub invite: Option<Invite>,
    pub peer: PeerId,
    pub port: u16,
}

impl LinkPeer {
    pub fn launcher_on_port(port: u16) -> Self {
        Self {
            invite: Invite::new(format!("{LINK_INVITE_PREFIX}{port}")).ok(),
            peer: LINK_LAUNCHER_PEER,
            port,
        }
    }
}
