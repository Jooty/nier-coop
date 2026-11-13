use std::sync::PoisonError;

use replicant_game::part_pickups::{PartnerPickupVerdict, partner_pickup_verdict};
use replicant_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::net::{self, NetState};
use crate::part_pickups::detour::current_phase_index;
use crate::part_pickups::state::PENDING;
use crate::story_relay::shared_story_events;

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::PartPickup(_)));
    let connected = net.view().state == NetState::Connected && shared_story_events();
    let phase = current_phase_index(build);
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    if !connected {
        pending.clear();
    }
    let dropped = pending.drop_other_phases(phase);
    if dropped > 0 {
        log_line(&format!(
            "part pickups: {dropped} partner pickups of another zone dropped"
        ));
    }
    for message in arrived {
        let Message::PartPickup(pickup) = message else {
            continue;
        };
        let verdict = partner_pickup_verdict(&pickup, connected, phase);
        if verdict != PartnerPickupVerdict::Queue {
            log_line(&format!(
                "part pickups: the partner's part {} in zone {} not taken ({verdict:?}), this zone is {phase:?}",
                pickup.part_group, pickup.phase
            ));
            continue;
        }
        let queued = pending.queue(pickup);
        log_line(&format!(
            "part pickups: the partner picked part {} in zone {}, {}",
            pickup.part_group,
            pickup.phase,
            if queued {
                "hidden here when the actor next counts its parts"
            } else {
                "already waiting"
            }
        ));
    }
}
