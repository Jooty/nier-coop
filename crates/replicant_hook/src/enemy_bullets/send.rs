use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::enemy_bullets::{
    ENEMY_BULLET_SCAN_INTERVAL_MS, EnemyBulletCensus, EnemyBulletKinds, EnemyBulletSender,
    FlightRefusals, LaserBeamTally, SpriteBulletKinds, UncopiedTally, scan_enemy_bullets,
};
use replicant_proto::{EnemyId, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::enemy_bullets::ENEMY_BULLET_SUMMARY_INTERVAL_MS;
use crate::enemy_bullets::script_effects::{set_shared_enemy_actors, take_summary_line};
use crate::memory::InProcessMemory;
use crate::net::{NetThread, net_clock_ms};

static SENDING: LazyLock<Mutex<Sending>> = LazyLock::new(Mutex::default);

#[derive(Debug, Default)]
struct Sending {
    sender: EnemyBulletSender,
    kinds: EnemyBulletKinds,
    sprite_kinds: SpriteBulletKinds,
    last_scan_ms: u64,
    messages: u32,
    sprite_messages: u32,
    ended: u32,
    most_in_the_air: EnemyBulletCensus,
    uncopied: UncopiedTally,
    beams: LaserBeamTally,
    refusals: FlightRefusals,
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
        set_shared_enemy_actors(core::iter::empty());
        return;
    };
    set_shared_enemy_actors(enemies.keys().copied());
    let now_ms = net_clock_ms();
    if now_ms.saturating_sub(sending.last_scan_ms) < ENEMY_BULLET_SCAN_INTERVAL_MS {
        return;
    }
    sending.last_scan_ms = now_ms;
    let memory = InProcessMemory::new(build);
    let Ok(scan) = scan_enemy_bullets(&memory, enemies) else {
        return;
    };
    sending.most_in_the_air.keep_highest(&scan.census);
    for line in sending.uncopied.add_scan(&scan.uncopied) {
        log_line(&line);
    }
    for line in sending.beams.add_scan(&scan.bullets) {
        log_line(&line);
    }
    for flight in sending.sender.update(&scan.bullets, now_ms) {
        if let Err(error) = flight.validate() {
            sending.refusals.note(error.to_string());
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
        if let Some(sprite) = flight.sprite
            && sending.sprite_kinds.first_sighting(&sprite)
        {
            log_line(&format!(
                "enemy bullets: sprite kind {} of size {} (alpha {}, angle {}) from enemy {} sent",
                sprite.kind,
                sprite.size,
                sprite.alpha,
                sprite.angle,
                enemy_id_text(flight.enemy)
            ));
        }
        sending.messages += 1;
        sending.beams.note_sent(flight.flight.instance);
        if flight.sprite.is_some() {
            sending.sprite_messages += 1;
        }
        if flight.flight.ended {
            sending.ended += 1;
        }
        net.send_relayed(Message::EnemyBulletFlight(flight));
    }
    if now_ms.saturating_sub(sending.last_summary_ms) >= ENEMY_BULLET_SUMMARY_INTERVAL_MS {
        sending.last_summary_ms = now_ms;
        if sending.messages > 0 {
            log_line(&format!(
                "enemy bullets: last 5 s sent {} flight messages ({} of them for sprites), {} bullets ended, {} in the air",
                sending.messages,
                sending.sprite_messages,
                sending.ended,
                sending.sender.len()
            ));
            sending.messages = 0;
            sending.sprite_messages = 0;
            sending.ended = 0;
        }
        let most = core::mem::take(&mut sending.most_in_the_air);
        if !most.is_empty() {
            log_line(&format!(
                "enemy bullets: last 5 s most in the air at once: {} normal ({} more not drawn), {} script with an effect, {} script sprite ({} more not drawn yet), {} script with no effect to copy, {} trace, {} past the limit",
                most.normal,
                most.normal_not_drawn,
                most.script_with_effect,
                most.script_sprite,
                most.script_sprite_not_drawn,
                most.script_without_effect,
                most.trace,
                most.past_the_limit
            ));
        }
        if let Some(line) = sending.uncopied.take_summary() {
            log_line(&line);
        }
        if let Some(line) = sending.beams.take_summary() {
            log_line(&line);
        }
        if let Some(line) = sending.refusals.take_summary() {
            log_line(&line);
        }
        if let Some(line) = take_summary_line() {
            log_line(&line);
        }
    }
}
