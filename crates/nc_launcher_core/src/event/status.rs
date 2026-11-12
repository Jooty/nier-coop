use std::time::Duration;

use nc_wire::SessionRules;
use replicant_proto::{
    GameReadiness, LobbyReady, PartnerBody, PlayerSettings, RejectReason, SessionOptions,
};

use crate::STORY_RULES;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Idle,
    Hosting,
    Joining,
    Connected,
    Rejected(RejectReason),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LauncherEvent {
    Hosting { ticket: String },
    Joining,
    MemberJoined { slot: u8, name: String },
    MemberLeft { slot: u8, detail: String },
    MemberChanged { slot: u8, change: MemberChange },
    PeerRefused { reason: RejectReason },
    Rejected { reason: RejectReason },
    PeerOnOtherVersion { ours: u16, theirs: u16 },
    GameLinked,
    GameUnlinked { detail: String },
    OwnGameReadiness { readiness: GameReadiness },
    GameChangedSettings { settings: PlayerSettings },
    SessionOptionsChanged { options: SessionOptions },
    LobbyStartReceived { mode: u8 },
    MessageRefused { kind: String, reason: String },
    Failed { message: String },
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberChange {
    Readiness(GameReadiness),
    Lobby(LobbyReady),
    Body(PartnerBody),
    Character(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberStatus {
    pub slot: u8,
    pub name: String,
    pub readiness: GameReadiness,
    pub lobby: Option<LobbyReady>,
    pub body: Option<PartnerBody>,
    pub character: Option<u8>,
    pub rtt_ms: Option<u32>,
}

impl MemberStatus {
    pub fn new(slot: u8, name: String) -> Self {
        Self {
            slot,
            name,
            readiness: GameReadiness::NotLinked,
            lobby: None,
            body: None,
            character: None,
            rtt_ms: None,
        }
    }

    pub fn apply(&mut self, change: MemberChange) {
        match change {
            MemberChange::Readiness(readiness) => self.readiness = readiness,
            MemberChange::Lobby(ready) => self.lobby = Some(ready),
            MemberChange::Body(body) => self.body = Some(body),
            MemberChange::Character(index) => self.character = Some(index),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkStatus {
    pub state: ConnectionState,
    pub ticket: Option<String>,
    pub members: Vec<MemberStatus>,
    pub rules: SessionRules,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    pub game_linked: bool,
    pub guest: bool,
    pub link_port: u16,
}

impl Default for LinkStatus {
    fn default() -> Self {
        Self {
            state: ConnectionState::default(),
            ticket: None,
            members: Vec::new(),
            rules: STORY_RULES,
            rtt: None,
            direct_path: None,
            game_linked: false,
            guest: false,
            link_port: 0,
        }
    }
}

impl LinkStatus {
    pub fn partner(&self) -> Option<&MemberStatus> {
        match self.members.as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }

    pub fn member(&self, slot: u8) -> Option<&MemberStatus> {
        self.members.iter().find(|member| member.slot == slot)
    }

    pub fn partner_character(&self) -> Option<u8> {
        self.partner().and_then(|partner| partner.character)
    }

    pub fn partner_readiness(&self) -> GameReadiness {
        self.partner()
            .map_or(GameReadiness::NotLinked, |partner| partner.readiness)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetState {
    #[default]
    Idle,
    Hosting {
        ticket: String,
    },
    Joining,
    Connected {
        peer_name: String,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetStatus {
    pub state: NetState,
}
