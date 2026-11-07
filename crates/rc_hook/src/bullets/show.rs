use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::bullet_flights::{BulletFlightTable, MAX_EXTRAPOLATION_MS};
use rc_proto::{BULLET_EFFECT_SLOTS, BULLET_TRANSFORM_FLOATS, BulletFlight, BulletLook, Message};

use crate::build_gate::VerifiedBuild;
use crate::bullets::SUMMARY_INTERVAL_MS;
use crate::bullets::calls::{create_effect, end_effect, place_effect, style_effect};
use crate::diagnostics::log_line;
use crate::net::{self, net_clock_ms};

const MILLIS_PER_SECOND: f32 = 1000.0;

static SHOWN: LazyLock<Mutex<Shown>> = LazyLock::new(Mutex::default);

#[derive(Debug)]
struct ShownBullet {
    handles: [Option<u32>; BULLET_EFFECT_SLOTS],
    flight: BulletFlight,
    received_ms: u64,
}

#[derive(Debug, Default)]
struct Shown {
    table: BulletFlightTable,
    live: BTreeMap<u16, ShownBullet>,
    kinds: Vec<[i32; BULLET_EFFECT_SLOTS]>,
    started: u32,
    ended: u32,
    timed_out: u32,
    refused: u32,
    last_summary_ms: u64,
}

impl Shown {
    fn start(
        &mut self,
        build: &VerifiedBuild,
        flight: BulletFlight,
        look: BulletLook,
        now_ms: u64,
    ) {
        let mut handles = [None; BULLET_EFFECT_SLOTS];
        for (index, handle) in handles.iter_mut().enumerate() {
            *handle = create_effect(build, look.effect_ids[index], &flight.transform);
            if let Some(handle) = *handle {
                style_effect(build, handle, &look.styles[index]);
            }
        }
        if handles.iter().all(Option::is_none) {
            self.table.forget(flight.instance);
            self.refused += 1;
            return;
        }
        if !self.kinds.contains(&look.effect_ids) {
            self.kinds.push(look.effect_ids);
            log_line(&format!(
                "bullets: showing a partner shot of kind {:?} ({} kinds seen)",
                look.effect_ids,
                self.kinds.len()
            ));
        }
        self.started += 1;
        self.live.insert(
            flight.instance,
            ShownBullet {
                handles,
                flight,
                received_ms: now_ms,
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
            let transform = extrapolated(&shown.flight, now_ms.saturating_sub(shown.received_ms));
            for handle in shown.handles.into_iter().flatten() {
                place_effect(build, handle, &transform);
            }
        }
    }

    fn summarize(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.last_summary_ms) < SUMMARY_INTERVAL_MS {
            return;
        }
        self.last_summary_ms = now_ms;
        if self.started + self.ended + self.timed_out + self.refused == 0 {
            return;
        }
        log_line(&format!(
            "bullets: last 5 s showed {} partner shots, {} ended, {} timed out, {} could not be drawn, {} in the air",
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

pub fn tick_partner_bullets(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::BulletFlight(_)));
    let mut shown = SHOWN.lock().unwrap_or_else(PoisonError::into_inner);
    if !net.view().partner_in_play() {
        for instance in shown.table.take_all() {
            shown.stop(build, instance);
        }
        return;
    }
    let now_ms = net_clock_ms();
    for message in arrived {
        let Message::BulletFlight(flight) = message else {
            continue;
        };
        if flight.ended {
            shown.table.forget(flight.instance);
            if shown.stop(build, flight.instance) {
                shown.ended += 1;
            }
            continue;
        }
        if !shown.table.note(flight.instance, now_ms) {
            shown.refused += 1;
            continue;
        }
        if let Some(live) = shown.live.get_mut(&flight.instance) {
            live.flight.transform = flight.transform;
            live.flight.velocity = flight.velocity;
            live.received_ms = now_ms;
        } else if let Some(look) = flight.look {
            shown.start(build, flight, look, now_ms);
        } else {
            shown.table.forget(flight.instance);
        }
    }
    for instance in shown.table.take_stale(now_ms) {
        if shown.stop(build, instance) {
            shown.timed_out += 1;
        }
    }
    shown.follow(build, now_ms);
    shown.summarize(now_ms);
}

fn extrapolated(flight: &BulletFlight, age_ms: u64) -> [f32; BULLET_TRANSFORM_FLOATS] {
    let seconds = age_ms.min(MAX_EXTRAPOLATION_MS) as f32 / MILLIS_PER_SECOND;
    let mut transform = flight.transform;
    for (axis, speed) in flight.velocity.into_iter().enumerate() {
        transform[axis * 4 + 3] += speed * seconds;
    }
    transform
}
