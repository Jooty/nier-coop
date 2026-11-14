use std::fmt::Debug;

use nc_wire::BuildGate;
use replicant_game::build::PeIdentity;

use crate::bridge::{BridgeParts, GameBridge};
use crate::{ConnectionReport, LauncherConfig, NetError, TransportFactory};

pub trait GameLink: Debug + Sync {
    fn protocol_version(&self) -> u16;
    fn hook_builds(&self) -> Vec<BuildGate>;
    fn session_build(&self) -> BuildGate;
    fn new_bridge(&self, parts: BridgeParts) -> Box<dyn GameBridge>;
    fn run_probe(
        &self,
        factory: &TransportFactory,
        config: &LauncherConfig,
        ticket: &str,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError>;
}

pub fn build_gate_of(identity: PeIdentity) -> BuildGate {
    BuildGate {
        time_date_stamp: identity.time_date_stamp,
        size_of_image: identity.size_of_image,
    }
}
