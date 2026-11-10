use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::bullet_flights::{FlightSample, smoothed_transform};
use replicant_game::enemy_bullets::{EnemyBulletBoard, EnemyBulletKinds, EnemyBulletStep};
use replicant_proto::{BULLET_EFFECT_SLOTS, BulletFlight, BulletLook, EnemyBulletFlight, Message};

use crate::build_gate::VerifiedBuild;
use crate::bullets::{create_effect, end_effect, place_effect, style_effect};
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::enemy_bullets::ENEMY_BULLET_SUMMARY_INTERVAL_MS;
use crate::net::{NetThread, net_clock_ms};

static SHOWN: LazyLock<Mutex<Shown>> = LazyLock::new(Mutex::default);

#[derive(Debug)]
struct ShownBullet {
    handles: [Option<u32>; BULLET_EFFECT_SLOTS],
    latest: FlightSample,
    previous: Option<FlightSample>,
}

#[derive(Debug, Default)]
struct Shown {
    board: EnemyBulletBoard,
    live: BTreeMap<u16, ShownBullet>,
    kinds: EnemyBulletKinds,
    started: u32,
    ended: u32,
    timed_out: u32,
    refused: u32,
    last_summary_ms: u64,
}

impl Shown {
    fn apply(&mut self, build: &VerifiedBuild, arrived: EnemyBulletFlight, now_ms: u64) {
        let flight = arrived.flight;
        match self.board.note(&flight, now_ms) {
            EnemyBulletStep::Start => {
                if let Some(look) = flight.look {
                    self.start(build, arrived, look, now_ms);
                }
            }
            EnemyBulletStep::Move => {
                if let Some(live) = self.live.get_mut(&flight.instance) {
                    live.previous = Some(live.latest);
                    live.latest = sample_of(&flight, now_ms);
                }
            }
            EnemyBulletStep::Stop => {
                if self.stop(build, flight.instance) {
                    self.ended += 1;
                }
            }
            EnemyBulletStep::Full => self.refused += 1,
            EnemyBulletStep::Ignore => {}
        }
    }

    fn start(
        &mut self,
        build: &VerifiedBuild,
        arrived: EnemyBulletFlight,
        look: BulletLook,
        now_ms: u64,
    ) {
        let flight = arrived.flight;
        let mut handles = [None; BULLET_EFFECT_SLOTS];
        for (index, handle) in handles.iter_mut().enumerate() {
            *handle = create_effect(build, look.effect_ids[index], &flight.transform);
            if let Some(handle) = *handle {
                style_effect(build, handle, &look.styles[index]);
            }
        }
        if handles.iter().all(Option::is_none) {
            self.board.forget(flight.instance);
            self.refused += 1;
            return;
        }
        if self.kinds.first_sighting(look.effect_ids) {
            log_line(&format!(
                "enemy bullets: kind {:?} from enemy {} shown",
                look.effect_ids,
                enemy_id_text(arrived.enemy)
            ));
        }
        self.started += 1;
        self.live.insert(
            flight.instance,
            ShownBullet {
                handles,
                latest: sample_of(&flight, now_ms),
                previous: None,
            },
        );
    }

    fn stop(&mut self, build: &VerifiedBuild, instance: u16) -> bool {
        let Some(shown) = self.live.remove(&instance) else {
            return false;
        };
        for handle in shown.handles.into_iter().flatten() {
            end_effect(build, handle);
        }
        true
    }

    fn follow(&self, build: &VerifiedBuild, now_ms: u64) {
        for shown in self.live.values() {
            let transform = smoothed_transform(shown.previous.as_ref(), &shown.latest, now_ms);
            for handle in shown.handles.into_iter().flatten() {
                place_effect(build, handle, &transform);
            }
        }
    }

    fn summarize(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.last_summary_ms) < ENEMY_BULLET_SUMMARY_INTERVAL_MS {
            return;
        }
        self.last_summary_ms = now_ms;
        if self.started + self.ended + self.timed_out + self.refused == 0 {
            return;
        }
        log_line(&format!(
            "enemy bullets: last 5 s showed {} enemy bullets, {} ended, {} timed out, {} could not be drawn, {} in the air",
            self.started,
            self.ended,
            self.timed_out,
            self.refused,
            self.live.len()
        ));
        self.started = 0;
        self.ended = 0;
        self.timed_out = 0;
        self.refused = 0;
    }
}

pub(super) fn show_enemy_bullets(build: &VerifiedBuild, net: &NetThread, in_play: bool) {
    let arrived = net.take_relayed(|message| matches!(message, Message::EnemyBulletFlight(_)));
    let mut shown = SHOWN.lock().unwrap_or_else(PoisonError::into_inner);
    if !in_play {
        for instance in shown.board.take_all() {
            shown.stop(build, instance);
        }
        return;
    }
    let now_ms = net_clock_ms();
    for message in arrived {
        if let Message::EnemyBulletFlight(flight) = message {
            shown.apply(build, flight, now_ms);
        }
    }
    for instance in shown.board.take_stale(now_ms) {
        if shown.stop(build, instance) {
            shown.timed_out += 1;
        }
    }
    shown.follow(build, now_ms);
    shown.summarize(now_ms);
}

fn sample_of(flight: &BulletFlight, now_ms: u64) -> FlightSample {
    FlightSample {
        transform: flight.transform,
        velocity: flight.velocity,
        received_ms: now_ms,
    }
}
