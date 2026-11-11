use std::ffi::CString;

use replicant_game::offsets::ACTOR_MODEL_PACK_OFFSET;
use replicant_game::puppet::{BodyInfo, BodyPackLoad, BodyPackStep, release_requested_pack_wanted};
use replicant_game::{GameMemory, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::puppet::error::PuppetError;
use crate::puppet::game_calls::GameCalls;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyPackReady {
    Waiting,
    Ready { pack: u64, requested_by_us: bool },
}

pub(super) fn advance(
    build: &VerifiedBuild,
    info: &BodyInfo,
    load: &mut BodyPackLoad,
) -> Result<BodyPackReady, PuppetError> {
    let name = CString::new(info.pack_name).map_err(|_| PuppetError::NameHasNul)?;
    let calls = GameCalls::new(build);
    let pack = calls.find_model_pack(&name);
    match load.step(pack != 0, net_clock_ms()) {
        BodyPackStep::Request => {
            let slot =
                calls.request_model_pack_of_kind(&name, info.request_kind, info.request_slot_hint);
            log_line(&format!(
                "ghost body {}: requested (slot {slot:#x})",
                info.pack_name
            ));
            Ok(BodyPackReady::Waiting)
        }
        BodyPackStep::Wait => Ok(BodyPackReady::Waiting),
        BodyPackStep::Spawn => {
            if load.requested_by_us() {
                log_line(&format!("ghost body {}: loaded", info.pack_name));
            }
            Ok(BodyPackReady::Ready {
                pack,
                requested_by_us: load.requested_by_us(),
            })
        }
    }
}

pub(super) fn release_if_ours(
    build: &VerifiedBuild,
    info: &BodyInfo,
    pack: u64,
    requested_by_us: bool,
) -> bool {
    let game = Replicant::new(InProcessMemory::new(build));
    let local_wears_pack = game.playable_actor().ok().and_then(|player| {
        game.memory()
            .read_u64(player + ACTOR_MODEL_PACK_OFFSET)
            .ok()
    }) == Some(pack);
    if !release_requested_pack_wanted(requested_by_us, local_wears_pack) || pack == 0 {
        return false;
    }
    GameCalls::new(build).release_model_pack(pack);
    log_line(&format!("ghost body {}: released", info.pack_name));
    true
}
