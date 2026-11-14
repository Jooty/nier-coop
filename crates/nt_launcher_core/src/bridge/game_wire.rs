use nt_wire::BuildGate;

use crate::bridge::{BridgeParts, GameBridge};
use crate::launcher_game::LauncherGame;
use crate::profile::{ALL_GAMES, GameProfile, default_profile, profile_by_id};
use crate::{ConnectionReport, LauncherConfig, NetError, TransportFactory};

/// Which game's message type the launcher's links carry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GameWire {
    game: &'static dyn LauncherGame,
}

impl PartialEq for GameWire {
    fn eq(&self, other: &Self) -> bool {
        self.game.game_id() == other.game.game_id()
    }
}

impl Eq for GameWire {}

impl GameWire {
    pub(crate) fn of(profile: &GameProfile) -> Self {
        let game = ALL_GAMES
            .iter()
            .copied()
            .find(|game| game.game_id() == profile.game_id)
            .unwrap_or(ALL_GAMES[0]);
        Self { game }
    }

    fn of_game(game: &'static dyn LauncherGame) -> Self {
        Self { game }
    }

    pub(crate) fn default_game() -> Self {
        Self::of(default_profile())
    }

    pub(crate) fn profile(self) -> &'static GameProfile {
        profile_by_id(self.game.game_id()).unwrap_or(default_profile())
    }

    pub(crate) fn protocol_version(self) -> u16 {
        self.game.link().protocol_version()
    }

    pub(crate) fn hook_builds(self) -> Vec<BuildGate> {
        self.game.link().hook_builds()
    }

    pub(crate) fn session_build(self) -> BuildGate {
        self.game.link().session_build()
    }

    pub(crate) fn new_bridge(self, parts: BridgeParts) -> Box<dyn GameBridge> {
        self.game.link().new_bridge(parts)
    }

    pub(crate) fn run_probe(
        self,
        factory: &TransportFactory,
        config: &LauncherConfig,
        ticket: &str,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError> {
        self.game.link().run_probe(factory, config, ticket, seconds)
    }
}

/// The protocol number the game's links speak.
pub fn protocol_version_of(profile: &GameProfile) -> u16 {
    GameWire::of(profile).protocol_version()
}

/// The line a host shows after refusing a player whose launcher speaks another protocol version.
pub fn version_difference_text(ours: u16, theirs: u16) -> String {
    let game_of = |version: u16| {
        ALL_GAMES
            .iter()
            .map(|game| GameWire::of_game(*game))
            .find(|wire| wire.protocol_version() == version)
            .map(|wire| wire.profile().display_name)
    };
    match (game_of(ours), game_of(theirs)) {
        (Some(our_game), Some(their_game)) if our_game != their_game => format!(
            "The other player has {their_game} open in their launcher and you have {our_game}. Pick the same game on both sides."
        ),
        _ => format!(
            "The other player's launcher is on another release (protocol {theirs}, yours is {ours}). Both of you need the same release."
        ),
    }
}
