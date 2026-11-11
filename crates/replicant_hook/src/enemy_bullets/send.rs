use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::enemy_bullets::{
    ENEMY_BULLET_SCAN_INTERVAL_MS, EnemyBulletCensus, EnemyBulletKinds, EnemyBulletSender,
    scan_enemy_bullets,
};
use replicant_proto::{EnemyId, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::enemy_bullets::ENEMY_BULLET_SUMMARY_INTERVAL_MS;
use crate::memory::InProcessMemory;
use crate::net::{NetThread, net_clock_ms};

static SENDING: LazyLock<Mutex<Sending>> = LazyLock::new(Mutex::default);

#[derive(Debug, Default)]
struct Sending {
    sender: EnemyBulletSender,
    kinds: EnemyBulletKinds,
    last_scan_ms: u64,
    messages: u32,
    ended: u32,
    most_in_the_air: EnemyBulletCensus,
    last_summary_ms: u64,
}

pub(super) fn send_enemy_bullets(
    build: &VerifiedBuild,
    net: &NetThread,
    enemies: Option<&HashMap<u64, EnemyId>>,
) {
    let mut sending = SENDING.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(enemies) = enemies else {
        // Not hosting a partner in play: the guest drops what it shows when the flights stop.
        let _ = sending.sender.end_all();
        return;
    };
    let now_ms = net_clock_ms();
    if now_ms.saturating_sub(sending.last_scan_ms) < ENEMY_BULLET_SCAN_INTERVAL_MS {
        return;
    }
    sending.last_scan_ms = now_ms;
    let Ok(scan) = scan_enemy_bullets(&InProcessMemory::new(build), enemies) else {
        return;
    };
    sending.most_in_the_air.keep_highest(&scan.census);
    for flight in sending.sender.update(&scan.bullets, now_ms) {
        if flight.validate().is_err() {
            continue;
        }
        if let Some(look) = flight.flight.look
            && sending.kinds.first_sighting(look.effect_ids)
        {
            log_line(&format!(
                "enemy bullets: kind {:?} from enemy {} sent",
                look.effect_ids,
                enemy_id_text(flight.enemy)
            ));
        }
        sending.messages += 1;
        if flight.flight.ended {
            sending.ended += 1;
        }
        net.send_relayed(Message::EnemyBulletFlight(flight));
    }
    if now_ms.saturating_sub(sending.last_summary_ms) >= ENEMY_BULLET_SUMMARY_INTERVAL_MS {
        sending.last_summary_ms = now_ms;
        if sending.messages > 0 {
            log_line(&format!(
                "enemy bullets: last 5 s sent {} flight messages, {} bullets ended, {} in the air",
                sending.messages,
                sending.ended,
                sending.sender.len()
            ));
            sending.messages = 0;
            sending.ended = 0;
        }
        let most = core::mem::take(&mut sending.most_in_the_air);
        if !most.is_empty() {
            log_line(&format!(
                "enemy bullets: last 5 s most in the air at once: {} normal ({} more not drawn), {} script with an effect, {} script sprite, {} script with no effect to copy, {} trace, {} past the limit",
                most.normal,
                most.normal_not_drawn,
                most.script_with_effect,
                most.script_sprite,
                most.script_without_effect,
                most.trace,
                most.past_the_limit
            ));
        }
    }
}
