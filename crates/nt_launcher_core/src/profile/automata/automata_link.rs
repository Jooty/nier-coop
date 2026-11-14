use automata_game::build::SUPPORTED_PE_IDENTITIES;
use nt_wire::{BuildGate, GameMessage};

use crate::bridge::{Bridge, BridgeParts, GameBridge};
use crate::launcher_game::{GameLink, build_gate_of};
use crate::probe::run_probe;
use crate::{ConnectionReport, LauncherConfig, NetError, TransportFactory};

#[derive(Debug)]
pub struct AutomataLink;

pub const AUTOMATA_LINK: AutomataLink = AutomataLink;

impl GameLink for AutomataLink {
    fn protocol_version(&self) -> u16 {
        <automata_proto::Message as GameMessage>::PROTOCOL_VERSION
    }

    fn hook_builds(&self) -> Vec<BuildGate> {
        SUPPORTED_PE_IDENTITIES
            .iter()
            .copied()
            .map(build_gate_of)
            .collect()
    }

    fn session_build(&self) -> BuildGate {
        // Two launchers of one game always agree, whichever of the game's exes each player runs.
        build_gate_of(SUPPORTED_PE_IDENTITIES[0])
    }

    fn new_bridge(&self, parts: BridgeParts) -> Box<dyn GameBridge> {
        Box::new(Bridge::<automata_proto::Message>::new(parts))
    }

    fn run_probe(
        &self,
        factory: &TransportFactory,
        config: &LauncherConfig,
        ticket: &str,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError> {
        run_probe::<automata_proto::Message>(factory, config, ticket, seconds)
    }
}
