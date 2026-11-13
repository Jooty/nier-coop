use automata_game::build::SUPPORTED_PE_IDENTITIES;
use nc_wire::{BuildGate, GameMessage};
use replicant_game::build::{PeIdentity, SUPPORTED_PE_IDENTITY};

use crate::bridge::{Bridge, BridgeParts, GameBridge};
use crate::probe::run_probe;
use crate::profile::{AUTOMATA, GameProfile, REPLICANT};
use crate::{ConnectionReport, LauncherConfig, NetError, TransportFactory};

const ALL_GAME_WIRES: [GameWire; 2] = [GameWire::Replicant, GameWire::Automata];

/// Which game's message type the launcher's links carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameWire {
    Replicant,
    Automata,
}

impl GameWire {
    pub(crate) fn of(profile: &GameProfile) -> Self {
        if profile.game_id == AUTOMATA.game_id {
            GameWire::Automata
        } else {
            GameWire::Replicant
        }
    }

    pub(crate) fn profile(self) -> &'static GameProfile {
        match self {
            GameWire::Replicant => &REPLICANT,
            GameWire::Automata => &AUTOMATA,
        }
    }

    pub(crate) fn protocol_version(self) -> u16 {
        match self {
            GameWire::Replicant => <replicant_proto::Message as GameMessage>::PROTOCOL_VERSION,
            GameWire::Automata => <automata_proto::Message as GameMessage>::PROTOCOL_VERSION,
        }
    }

    pub(crate) fn hook_builds(self) -> Vec<BuildGate> {
        match self {
            GameWire::Replicant => vec![build_gate_of(SUPPORTED_PE_IDENTITY)],
            GameWire::Automata => SUPPORTED_PE_IDENTITIES
                .iter()
                .copied()
                .map(build_gate_of)
                .collect(),
        }
    }

    pub(crate) fn session_build(self) -> BuildGate {
        // Two launchers of one game always agree, whichever of the game's exes each player runs.
        match self {
            GameWire::Replicant => build_gate_of(SUPPORTED_PE_IDENTITY),
            GameWire::Automata => build_gate_of(SUPPORTED_PE_IDENTITIES[0]),
        }
    }

    pub(crate) fn new_bridge(self, parts: BridgeParts) -> Box<dyn GameBridge> {
        match self {
            GameWire::Replicant => Box::new(Bridge::<replicant_proto::Message>::new(parts)),
            GameWire::Automata => Box::new(Bridge::<automata_proto::Message>::new(parts)),
        }
    }

    pub(crate) fn run_probe(
        self,
        factory: &TransportFactory,
        config: &LauncherConfig,
        ticket: &str,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError> {
        match self {
            GameWire::Replicant => {
                run_probe::<replicant_proto::Message>(factory, config, ticket, seconds)
            }
            GameWire::Automata => {
                run_probe::<automata_proto::Message>(factory, config, ticket, seconds)
            }
        }
    }
}

/// The protocol number the game's links speak.
pub fn protocol_version_of(profile: &GameProfile) -> u16 {
    GameWire::of(profile).protocol_version()
}

/// The line a host shows after refusing a player whose launcher speaks another protocol version.
pub fn version_difference_text(ours: u16, theirs: u16) -> String {
    let game_of = |version: u16| {
        ALL_GAME_WIRES
            .into_iter()
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

fn build_gate_of(identity: PeIdentity) -> BuildGate {
    BuildGate {
        time_date_stamp: identity.time_date_stamp,
        size_of_image: identity.size_of_image,
    }
}
