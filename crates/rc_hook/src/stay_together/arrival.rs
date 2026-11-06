use std::time::Duration;

use rc_game::arrival::{
    ArrivalGuard, ArrivalInputs, ArrivalStep, FallGuard, FallInputs, PlaceReason,
};
use rc_game::stay_together::{PARTNER_TIMEOUT_MS, WorldProbe, choose_spot, distance_between};
use rc_game::{Position, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks;
use crate::memory::InProcessMemory;
use crate::net::NetShared;
use crate::stay_together::free_to_act::game_says_free_to_act;
use crate::stay_together::world_probe::CollisionProbe;

pub(super) struct ArrivalState {
    pub(super) arrival: ArrivalGuard,
    fall: FallGuard,
}

struct FreshPartner {
    phase: u16,
    position: Position,
}

impl ArrivalState {
    pub(super) const fn new() -> Self {
        Self {
            arrival: ArrivalGuard::new(),
            fall: FallGuard::new(),
        }
    }
}

pub(super) fn keep_safe(
    state: &mut ArrivalState,
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    view: &NetShared,
    local_phase: Option<u16>,
    now_ms: u64,
) {
    let Ok(local) = game.position() else {
        return;
    };
    let probe = CollisionProbe::new(build);
    let ground_under_local = probe.floor_below(local.x, local.y, local.z).is_some();
    let partner = fresh_partner(view);
    let fall_inputs = FallInputs {
        now_ms,
        local_phase,
        local,
        ground_under_local,
        partner_y_in_same_phase: partner
            .as_ref()
            .filter(|partner| Some(partner.phase) == local_phase)
            .map(|partner| partner.position.y),
    };
    if state.fall.has_fallen(&fall_inputs) {
        recover_from_fall(state, &probe, local, partner.as_ref(), local_phase);
    }
    let arrival_inputs = ArrivalInputs {
        now_ms,
        local_phase,
        partner_phase: partner.as_ref().map(|partner| partner.phase),
        free_to_act: game_says_free_to_act(build).unwrap_or(false),
        distance_to_partner: partner.as_ref().and_then(|partner| {
            distance_between(
                [local.x, local.y, local.z],
                [partner.position.x, partner.position.y, partner.position.z],
            )
        }),
        ground_under_local,
    };
    place_after_follow(state, &probe, local, partner.as_ref(), &arrival_inputs);
}

fn fresh_partner(view: &NetShared) -> Option<FreshPartner> {
    let remote = view.remote.as_ref()?;
    if !view.partner_in_play() || remote.age() > Duration::from_millis(PARTNER_TIMEOUT_MS) {
        return None;
    }
    let [x, y, z] = remote.state.position;
    Some(FreshPartner {
        phase: remote.state.phase,
        position: Position { x, y, z },
    })
}

fn place_after_follow(
    state: &mut ArrivalState,
    probe: &CollisionProbe,
    local: Position,
    partner: Option<&FreshPartner>,
    inputs: &ArrivalInputs,
) {
    match state.arrival.step(inputs) {
        ArrivalStep::Idle | ArrivalStep::Waiting => {}
        ArrivalStep::AlreadyBesideHost => log_line("arrival: already beside the host, left alone"),
        ArrivalStep::GaveUp => log_line("arrival: gave up waiting for the host in the same map"),
        ArrivalStep::Place(reason) => {
            let Some(partner) = partner else {
                return;
            };
            match choose_spot(partner.position, local, probe) {
                Ok(spot) if game_hooks::teleport_available() => {
                    game_hooks::request_teleport(spot);
                    state.arrival.placed();
                    log_line(&format!(
                        "arrival: placed beside the host at {:.1}, {:.1}, {:.1} ({})",
                        spot.x,
                        spot.y,
                        spot.z,
                        place_reason_label(reason)
                    ));
                }
                Ok(_) => log_line("arrival: cannot place, the movement hook is not installed"),
                Err(refusal) => log_line(&format!(
                    "arrival: no spot beside the host yet ({})",
                    refusal.label()
                )),
            }
        }
    }
}

fn recover_from_fall(
    state: &ArrivalState,
    probe: &CollisionProbe,
    local: Position,
    partner: Option<&FreshPartner>,
    local_phase: Option<u16>,
) {
    if !game_hooks::teleport_available() {
        log_line("fall: below the floor but the movement hook is not installed");
        return;
    }
    let beside_partner = partner
        .filter(|partner| Some(partner.phase) == local_phase)
        .and_then(|partner| choose_spot(partner.position, local, probe).ok());
    match beside_partner.or(state.fall.last_grounded()) {
        Some(spot) => {
            game_hooks::request_teleport(spot);
            log_line(&format!(
                "fall: no ground under you, moved back to {:.1}, {:.1}, {:.1}",
                spot.x, spot.y, spot.z
            ));
        }
        None => log_line("fall: no ground under you and nowhere known to go back to"),
    }
}

fn place_reason_label(reason: PlaceReason) -> &'static str {
    match reason {
        PlaceReason::TooFarFromHost => "too far from the host",
        PlaceReason::NoGroundUnderYou => "no ground under you",
    }
}
