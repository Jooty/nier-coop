use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::npcs::read_npc_body_layer;
use rc_game::offsets::ACTOR_MODEL_PACK_OFFSET;
use rc_game::power_models::{
    POWER_MODEL_KINDS, POWER_MODEL_SCAN_INTERVAL_MS, PowerCastLog, SightedHand, hands_of_player,
    power_model_pack_name,
};
use rc_game::{GameMemory, Replicant};
use rc_proto::{
    Message, MotionState, PowerModelEnded, PowerModelKind, PowerModelMotion, PowerModelState,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::game_calls::GameCalls;

static LAST_SCAN_MS: AtomicU64 = AtomicU64::new(0);
static OWN_CASTS: LazyLock<Mutex<PowerCastLog>> = LazyLock::new(Mutex::default);

pub fn tick_own_powers(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    if !net.view().partner_in_play() {
        OWN_CASTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .end_missing(&[], 0);
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
    let mut casts = OWN_CASTS.lock().unwrap_or_else(PoisonError::into_inner);
    let present: Vec<u16> = hands.iter().map(|hand| hand.slot).collect();
    for ended in casts.end_missing(&present, now_ms) {
        log_line(&ended.describe("your"));
        net.send_relayed(Message::PowerEnded(PowerModelEnded {
            instance: ended.instance,
            lasted_ms: u32::try_from(ended.lasted_ms).unwrap_or(u32::MAX),
        }));
    }
    for hand in hands {
        let kind = kind_of(build, &game, hand.actor);
        if casts.begin(hand.slot, power_model_pack_name(kind), now_ms) {
            announce(&hand);
        }
        let state = PowerModelState {
            kind,
            instance: hand.slot,
            position: [
                hand.pose.position.x,
                hand.pose.position.y,
                hand.pose.position.z,
            ],
            yaw_radians: hand.pose.yaw,
        };
        if state.validate().is_err() {
            continue;
        }
        net.send_relayed(Message::PowerModel(state));
        let Some(motion) = hand_motion(&game, hand.actor) else {
            continue;
        };
        if casts.motion(hand.slot, motion.index) {
            log_line(&format!(
                "powers: your hand #{} plays motion {} from frame {:.1} (looping {})",
                hand.slot, motion.index, motion.frame, motion.looping
            ));
        }
        let message = PowerModelMotion {
            instance: hand.slot,
            motion,
        };
        if message.validate().is_ok() {
            net.send_relayed(Message::PowerMotion(message));
        }
    }
}

fn announce(hand: &SightedHand) {
    log_line(&format!(
        "powers: sending your hand actor {:#x} (slot {}) at {:?}, yaw {:.2}",
        hand.actor, hand.slot, hand.pose.position, hand.pose.yaw
    ));
}

fn hand_motion(game: &Replicant<InProcessMemory>, actor: u64) -> Option<MotionState> {
    let layer = read_npc_body_layer(game.memory(), actor).ok().flatten()?;
    Some(MotionState {
        index: u16::try_from(layer.motion.index).ok()?,
        frame: layer.motion.frame.max(0.0),
        speed: layer.motion.speed,
        looping: layer.motion.looping,
        name_hash: 0,
    })
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
