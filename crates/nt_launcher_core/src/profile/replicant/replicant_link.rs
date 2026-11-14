use nt_wire::{BuildGate, GameMessage};
use replicant_game::build::SUPPORTED_PE_IDENTITY;

use crate::bridge::{Bridge, BridgeParts, GameBridge};
use crate::launcher_game::{GameLink, build_gate_of};
use crate::probe::run_probe;
use crate::{ConnectionReport, LauncherConfig, NetError, TransportFactory};

#[derive(Debug)]
pub struct ReplicantLink;

pub const REPLICANT_LINK: ReplicantLink = ReplicantLink;

impl GameLink for ReplicantLink {
    fn protocol_version(&self) -> u16 {
        <replicant_proto::Message as GameMessage>::PROTOCOL_VERSION
    }

    fn hook_builds(&self) -> Vec<BuildGate> {
        vec![build_gate_of(SUPPORTED_PE_IDENTITY)]
    }

    fn session_build(&self) -> BuildGate {
        build_gate_of(SUPPORTED_PE_IDENTITY)
    }

    fn new_bridge(&self, parts: BridgeParts) -> Box<dyn GameBridge> {
        Box::new(Bridge::<replicant_proto::Message>::new(parts))
    }

    fn run_probe(
        &self,
        factory: &TransportFactory,
        config: &LauncherConfig,
        ticket: &str,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError> {
        run_probe::<replicant_proto::Message>(factory, config, ticket, seconds)
    }
}
