use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::offsets::ACTOR_MODEL_PACK_OFFSET;
use rc_game::power_models::{
    POWER_MODEL_KINDS, POWER_MODEL_SCAN_INTERVAL_MS, SightedHand, hands_of_player,
    power_model_pack_name,
};
use rc_game::{GameMemory, Replicant};
use rc_proto::{Message, PowerModelKind, PowerModelState};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::game_calls::GameCalls;

static LAST_SCAN_MS: AtomicU64 = AtomicU64::new(0);
static ANNOUNCED_SLOTS: Mutex<Vec<u16>> = Mutex::new(Vec::new());

pub fn tick_own_powers(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    if !net.view().partner_in_play() {
        return;
    }
    let now_ms = net_clock_ms();
    if now_ms.saturating_sub(LAST_SCAN_MS.load(Ordering::Relaxed)) < POWER_MODEL_SCAN_INTERVAL_MS {
        return;
    }
    LAST_SCAN_MS.store(now_ms, Ordering::Relaxed);
    let game = Replicant::new(InProcessMemory::new(build));
    let Ok(player) = game.playable_actor() else {
        return;
    };
    let Ok(hands) = hands_of_player(game.memory(), player) else {
        return;
    };
    announce_new_slots(&hands);
    for hand in hands {
        let state = PowerModelState {
            kind: kind_of(build, &game, hand.actor),
            instance: hand.slot,
            position: [
                hand.pose.position.x,
                hand.pose.position.y,
                hand.pose.position.z,
            ],
            yaw_radians: hand.pose.yaw,
        };
        if state.validate().is_ok() {
            net.send_relayed(Message::PowerModel(state));
        }
    }
}

fn announce_new_slots(hands: &[SightedHand]) {
    let mut announced = ANNOUNCED_SLOTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    announced.retain(|slot| hands.iter().any(|hand| hand.slot == *slot));
    for hand in hands {
        if !announced.contains(&hand.slot) {
            announced.push(hand.slot);
            log_line(&format!(
                "powers: sending your hand actor {:#x} (slot {}) at {:?}, yaw {:.2}",
                hand.actor, hand.slot, hand.pose.position, hand.pose.yaw
            ));
        }
    }
}

fn kind_of(build: &VerifiedBuild, game: &Replicant<InProcessMemory>, actor: u64) -> PowerModelKind {
    let worn = game
        .memory()
        .read_u64(actor + ACTOR_MODEL_PACK_OFFSET)
        .unwrap_or(0);
    let calls = GameCalls::new(build);
    let matched = POWER_MODEL_KINDS.into_iter().find(|kind| {
        std::ffi::CString::new(power_model_pack_name(*kind))
            .is_ok_and(|name| worn != 0 && calls.find_model_pack(&name) == worn)
    });
    matched.unwrap_or_else(|| {
        log_line(&format!(
            "powers: hand {actor:#x} wears pack {worn:#x}, not one of the known hand packs; sending the first kind"
        ));
        POWER_MODEL_KINDS[0]
    })
}
