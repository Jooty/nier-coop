use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::bullet_flights::{FlightSample, smoothed_transform};
use replicant_game::enemy_bullets::{
    EnemyBulletBoard, EnemyBulletKinds, EnemyBulletStep, ShownEffectTally, SpriteBulletKinds,
    first_sprite_frame, next_sprite_frame,
};
use replicant_proto::{
    BULLET_EFFECT_SLOTS, BulletFlight, BulletLook, BulletSprite, EnemyBulletFlight, Message,
};

use crate::build_gate::VerifiedBuild;
use crate::bullets::{create_effect, end_effect, place_effect, style_effect};
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::enemy_bullets::ENEMY_BULLET_SUMMARY_INTERVAL_MS;
use crate::enemy_bullets::sprite_draw::{
    SpriteToDraw, set_sprites_to_draw, take_sprite_draw_counts,
};
use crate::net::{NetThread, net_clock_ms};

static SHOWN: LazyLock<Mutex<Shown>> = LazyLock::new(Mutex::default);

#[derive(Debug)]
struct ShownSprite {
    sprite: BulletSprite,
    frame: u16,
}

#[derive(Debug)]
struct ShownBullet {
    handles: [Option<u32>; BULLET_EFFECT_SLOTS],
    effect_id: i32,
    sprite: Option<ShownSprite>,
    latest: FlightSample,
    previous: Option<FlightSample>,
}

#[derive(Debug, Default)]
struct Shown {
    board: EnemyBulletBoard,
    live: BTreeMap<u16, ShownBullet>,
    kinds: EnemyBulletKinds,
    sprite_kinds: SpriteBulletKinds,
    shown_effects: ShownEffectTally,
    started: u32,
    started_sprites: u32,
    ended: u32,
    timed_out: u32,
    refused: u32,
    last_summary_ms: u64,
}

impl Shown {
    fn apply(&mut self, build: &VerifiedBuild, arrived: EnemyBulletFlight, now_ms: u64) {
        let flight = arrived.flight;
        match self.board.note(&arrived, now_ms) {
            EnemyBulletStep::Start => {
                if let Some(sprite) = arrived.sprite {
                    self.start_sprite(arrived, sprite, now_ms);
                } else if let Some(look) = flight.look {
                    self.start(build, arrived, look, now_ms);
                }
            }
            EnemyBulletStep::Move => {
                if let Some(live) = self.live.get_mut(&flight.instance) {
                    live.previous = Some(live.latest);
                    live.latest = sample_of(&flight, now_ms);
                    if let (Some(shown), Some(sprite)) = (live.sprite.as_mut(), arrived.sprite) {
                        shown.sprite = sprite;
                    }
                    if live.sprite.is_none() {
                        self.shown_effects
                            .note_move(live.effect_id, &flight.transform);
                    }
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
        self.shown_effects
            .note_start(look.effect_ids[0], &flight.transform);
        self.live.insert(
            flight.instance,
            ShownBullet {
                handles,
                effect_id: look.effect_ids[0],
                sprite: None,
                latest: sample_of(&flight, now_ms),
                previous: None,
            },
        );
    }

    fn start_sprite(&mut self, arrived: EnemyBulletFlight, sprite: BulletSprite, now_ms: u64) {
        let flight = arrived.flight;
        if self.sprite_kinds.first_sighting(&sprite) {
            log_line(&format!(
                "enemy bullets: sprite kind {} of size {} from enemy {} goes to the game's sprite batch on each draw",
                sprite.kind,
                sprite.size,
                enemy_id_text(arrived.enemy)
            ));
        }
        self.started += 1;
        self.started_sprites += 1;
        self.live.insert(
            flight.instance,
            ShownBullet {
                handles: [None; BULLET_EFFECT_SLOTS],
                effect_id: 0,
                sprite: Some(ShownSprite {
                    sprite,
                    frame: first_sprite_frame(flight.instance),
                }),
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

    fn follow(&mut self, build: &VerifiedBuild, now_ms: u64) {
        let mut sprites = Vec::new();
        for shown in self.live.values_mut() {
            let transform = smoothed_transform(shown.previous.as_ref(), &shown.latest, now_ms);
            for handle in shown.handles.into_iter().flatten() {
                place_effect(build, handle, &transform);
            }
            if let Some(sprite) = shown.sprite.as_mut() {
                sprite.frame = next_sprite_frame(sprite.frame);
                sprites.push(SpriteToDraw {
                    position: [transform[3], transform[7], transform[11]],
                    sprite: sprite.sprite,
                    frame: sprite.frame,
                });
            }
        }
        set_sprites_to_draw(sprites);
    }

    fn summarize(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.last_summary_ms) < ENEMY_BULLET_SUMMARY_INTERVAL_MS {
            return;
        }
        self.last_summary_ms = now_ms;
        self.summarize_sprite_draws();
        if let Some(line) = self.shown_effects.take_summary() {
            log_line(&line);
        }
        if self.started + self.ended + self.timed_out + self.refused == 0 {
            return;
        }
        log_line(&format!(
            "enemy bullets: last 5 s showed {} enemy bullets ({} of them sprites), {} ended, {} timed out, {} could not be drawn, {} in the air",
            self.started,
            self.started_sprites,
            self.ended,
            self.timed_out,
            self.refused,
            self.live.len()
        ));
        self.started = 0;
        self.started_sprites = 0;
        self.ended = 0;
        self.timed_out = 0;
        self.refused = 0;
    }

    fn summarize_sprite_draws(&self) {
        let counts = take_sprite_draw_counts();
        let sprites_in_the_air = self
            .live
            .values()
            .filter(|shown| shown.sprite.is_some())
            .count();
        if sprites_in_the_air == 0 && counts.draws == 0 {
            return;
        }
        // Handed but not taken: off screen, too near the camera, or the batch was full.
        log_line(&format!(
            "enemy bullets: last 5 s the bullet draw ran {} times with host sprites ({} found no sprite batch), handed the batch {} sprites and it took {}, {} sprites in the air now",
            counts.draws,
            counts.draws_without_a_batch,
            counts.handed,
            counts.taken,
            sprites_in_the_air
        ));
    }
}

pub(super) fn show_enemy_bullets(build: &VerifiedBuild, net: &NetThread, in_play: bool) {
    let arrived = net.take_relayed(|message| matches!(message, Message::EnemyBulletFlight(_)));
    let mut shown = SHOWN.lock().unwrap_or_else(PoisonError::into_inner);
    if !in_play {
        for instance in shown.board.take_all() {
            shown.stop(build, instance);
        }
        set_sprites_to_draw(Vec::new());
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
