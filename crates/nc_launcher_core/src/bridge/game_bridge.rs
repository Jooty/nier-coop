use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nc_net::LocalLinkTransport;
use replicant_proto::{LobbyReady, PartnerBody, PlayerSettings, SessionOptions};

use crate::{LauncherConfig, LauncherEvent, LinkStatus, NetError, TransportFactory};

/// Everything a bridge keeps when the launcher changes to another game's messages.
pub(crate) struct BridgeParts {
    pub(crate) link: LocalLinkTransport,
    pub(crate) factory: TransportFactory,
    pub(crate) config: LauncherConfig,
    pub(crate) status: Arc<Mutex<LinkStatus>>,
    pub(crate) events: Sender<LauncherEvent>,
    pub(crate) options: SessionOptions,
    pub(crate) settings: Option<PlayerSettings>,
    pub(crate) own_lobby: Option<LobbyReady>,
    pub(crate) own_body: Option<PartnerBody>,
    pub(crate) own_character: Option<u8>,
}

pub(crate) trait GameBridge: Send {
    fn host(&mut self) -> Result<String, NetError>;
    fn join(&mut self, ticket: &str) -> Result<(), NetError>;
    fn stop(&mut self);
    fn set_options(&mut self, options: SessionOptions);
    fn set_settings(&mut self, settings: PlayerSettings);
    fn set_lobby_ready(&mut self, ready: LobbyReady);
    fn set_lobby_body(&mut self, body: PartnerBody);
    fn set_lobby_character(&mut self, index: u8);
    fn quit_game(&mut self);
    fn start_lobby(&mut self, mode: u8);
    fn set_name(&mut self, name: &str);
    fn pump(&mut self, now: Instant);
    fn into_parts(self: Box<Self>) -> BridgeParts;
}

impl BridgeParts {
    pub(crate) fn new(
        link: LocalLinkTransport,
        factory: TransportFactory,
        config: LauncherConfig,
        status: Arc<Mutex<LinkStatus>>,
        events: Sender<LauncherEvent>,
    ) -> Self {
        Self {
            link,
            factory,
            config,
            status,
            events,
            options: SessionOptions::default(),
            settings: None,
            own_lobby: None,
            own_body: None,
            own_character: None,
        }
    }
}
