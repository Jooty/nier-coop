use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rc_game::effects::MAX_PUPPET_EFFECTS_PER_TICK;
use rc_game::effects::{LiveEffects, PUPPET_EFFECT_MAX_LIFETIME_MS, finish_effect, sound_gain};
use rc_game::puppet::actor_is_live;
use rc_proto::{EffectPlay, Message};

use crate::build_gate::VerifiedBuild;
use crate::effects::calls::{
    PlayedEffect, align_anchor_offset, distance_to_listener, play_effect, play_sound,
};
use crate::effects::hooks::{send_ended_effects, set_sending};
use crate::effects::report::{
    DROPPED_WITHOUT_PUPPET, PLAY_FAILED, PLAYED_EFFECTS, PLAYED_SOUNDS, PLAYED_STOPS,
    SILENCED_BY_DISTANCE, count, log_limited, log_summary,
};
use crate::effects::{
    MAX_PENDING_EVENTS, PLAYBACK_DELAY, RETAINED_BLOCK_EXTRA_MS, SUMMARY_INTERVAL_MS,
};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::first_actor_address;

static MIRROR: LazyLock<Mutex<Mirror>> = LazyLock::new(|| Mutex::new(Mirror::new()));

struct Mirror {
    pending: VecDeque<(Instant, Message)>,
    live: LiveEffects,
    retained_blocks: VecDeque<(u64, Vec<u64>)>,
    last_summary_ms: u64,
}

impl Mirror {
    fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            live: LiveEffects::new(),
            retained_blocks: VecDeque::new(),
            last_summary_ms: 0,
        }
    }

    fn finish_effects(&mut self, build: &VerifiedBuild, handles: &[u32]) {
        let mut memory = InProcessMemory::new(build);
        for handle in handles {
            let _ = finish_effect(&mut memory, *handle);
        }
    }

    fn play_aligned_effect(
        &mut self,
        build: &VerifiedBuild,
        puppet: u64,
        effect: &EffectPlay,
    ) -> Result<PlayedEffect, String> {
        align_anchor_offset(build, puppet);
        play_effect(build, puppet, effect)
    }

    fn expire(&mut self, build: &VerifiedBuild, now_ms: u64) {
        let expired = self.live.take_expired(now_ms);
        self.finish_effects(build, &expired);
        let block_life_ms = PUPPET_EFFECT_MAX_LIFETIME_MS + RETAINED_BLOCK_EXTRA_MS;
        while self
            .retained_blocks
            .front()
            .is_some_and(|(started, _)| now_ms.saturating_sub(*started) >= block_life_ms)
        {
            self.retained_blocks.pop_front();
        }
    }

    fn play(&mut self, build: &VerifiedBuild, puppet: u64, message: Message, now_ms: u64) {
        match message {
            Message::Effect(effect) => match self.play_aligned_effect(build, puppet, &effect) {
                Ok(played) => {
                    self.retained_blocks.push_back((now_ms, played.block));
                    if let Some(handle) = played.handle {
                        count(&PLAYED_EFFECTS);
                        log_limited(&format!(
                            "effects: puppet {puppet:#x} effect {} -> handle {handle:#x} (sender {:#x})",
                            effect.effect, effect.handle
                        ));
                        if let Some(evicted) = self.live.track(handle, effect.handle, now_ms) {
                            self.finish_effects(build, &[evicted]);
                        }
                    } else {
                        count(&PLAY_FAILED);
                        log_limited(&format!(
                            "effects: puppet {puppet:#x} effect {} returned no handle",
                            effect.effect
                        ));
                    }
                }
                Err(error) => {
                    count(&PLAY_FAILED);
                    log_limited(&format!("effects: cannot play {} ({error})", effect.effect));
                }
            },
            Message::Sound(sound) => {
                let gain = distance_to_listener(build, puppet).map_or(1.0, sound_gain);
                if gain <= 0.0 {
                    count(&SILENCED_BY_DISTANCE);
                } else if play_sound(build, puppet, &sound.sound, gain) {
                    count(&PLAYED_SOUNDS);
                    log_limited(&format!(
                        "effects: puppet {puppet:#x} sound {}",
                        sound.sound
                    ));
                } else {
                    count(&PLAY_FAILED);
                    log_limited(&format!(
                        "effects: puppet sound {} returned nothing",
                        sound.sound
                    ));
                }
            }
            Message::EffectStop(stop) => {
                if let Some(puppet_handle) = self.live.take_by_sender(stop.handle) {
                    self.finish_effects(build, &[puppet_handle]);
                    count(&PLAYED_STOPS);
                    log_limited(&format!(
                        "effects: puppet stop sender handle {:#x} -> puppet handle {puppet_handle:#x}",
                        stop.handle
                    ));
                }
            }
            _ => {}
        }
    }
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let active = net.view().partner_in_play();
    set_sending(active);
    if active {
        send_ended_effects(build);
    }
    let arrived = net.take_relayed(|message| {
        matches!(
            message,
            Message::Effect(_) | Message::Sound(_) | Message::EffectStop(_)
        )
    });
    let mut mirror = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    let now_ms = net_clock_ms();
    mirror.expire(build, now_ms);
    if active {
        for message in arrived {
            if mirror.pending.len() >= MAX_PENDING_EVENTS {
                mirror.pending.pop_front();
                count(&DROPPED_WITHOUT_PUPPET);
            }
            mirror.pending.push_back((now, message));
        }
        play_due(build, &mut mirror, now, now_ms);
    } else {
        mirror.pending.clear();
        let all = mirror.live.take_all();
        mirror.finish_effects(build, &all);
    }
    if now_ms.saturating_sub(mirror.last_summary_ms) >= SUMMARY_INTERVAL_MS {
        mirror.last_summary_ms = now_ms;
        log_summary(mirror.live.len());
    }
}

fn play_due(build: &VerifiedBuild, mirror: &mut Mirror, now: Instant, now_ms: u64) {
    let puppet = first_actor_address()
        .filter(|actor| actor_is_live(&InProcessMemory::new(build), *actor).unwrap_or(false));
    let mut played = 0;
    while played < MAX_PUPPET_EFFECTS_PER_TICK
        && mirror
            .pending
            .front()
            .is_some_and(|(arrived, _)| now.saturating_duration_since(*arrived) >= PLAYBACK_DELAY)
    {
        let Some((_, message)) = mirror.pending.pop_front() else {
            break;
        };
        match puppet {
            Some(puppet) => {
                mirror.play(build, puppet, message, now_ms);
                played += 1;
            }
            None => count(&DROPPED_WITHOUT_PUPPET),
        }
    }
}
