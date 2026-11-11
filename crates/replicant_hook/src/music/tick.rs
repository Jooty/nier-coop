use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use replicant_game::marker::read_world_flags;
use replicant_game::music::{
    GuestSync, LocalMusic, MusicChannel, MusicSender, MusicStep, read_music_channel,
};
use replicant_game::offsets::BGM_CHANNEL_STATE_EVENT_POSTED;
use replicant_game::stay_together::FollowRole;
use replicant_proto::{Message, MusicState, PartnerActivity};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;
use crate::music::calls::{
    music_init_position_ms, play, seek, set_layer_volume, set_master_volume, source_position_ms,
    stop,
};
use crate::music::hooks::set_host_owns_local_music;
use crate::music::report::{log_limited, take_ignored_calls};
use crate::music::{DIAGNOSTIC_INTERVAL, PLAY_FADE_SECONDS, VOLUME_RAMP_SECONDS};
use crate::net::{self, NetShared, NetState, net_clock_ms};

const SEEK_CHECK_DELAY_MS: u64 = 250;

static STATE: LazyLock<Mutex<MusicTick>> = LazyLock::new(|| Mutex::new(MusicTick::new()));

struct SeekCheck {
    asked_ms: u32,
    asked_at_ms: u64,
}

struct MusicTick {
    sender: MusicSender,
    guest: GuestSync,
    was_connected: bool,
    was_controlling: bool,
    last_logged: Option<MusicState>,
    last_diagnostic: Instant,
    seek_check: Option<SeekCheck>,
}

impl MusicTick {
    fn new() -> Self {
        Self {
            sender: MusicSender::new(),
            guest: GuestSync::new(),
            was_connected: false,
            was_controlling: false,
            last_logged: None,
            last_diagnostic: Instant::now(),
            seek_check: None,
        }
    }
}

impl MusicTick {
    fn host_tick(&mut self, build: &VerifiedBuild, view: &NetShared) {
        let Some(net) = net::handle() else {
            return;
        };
        let memory = InProcessMemory::new(build);
        let in_menu = read_world_flags(&memory)
            .map(|flags| flags.activity() == PartnerActivity::InMenu)
            .unwrap_or(true);
        if in_menu || !view.effective_local_readiness().is_in_play() {
            return;
        }
        let Ok(channel) = read_music_channel(&memory) else {
            return;
        };
        let position_ms = channel_position_ms(build, &channel).unwrap_or(0);
        let state = channel.to_wire(position_ms);
        if self.sender.should_send(net_clock_ms(), &state) {
            net.send_relayed(Message::Music(state));
            self.log_host_change(&state, &channel);
        }
        if self.last_diagnostic.elapsed() >= DIAGNOSTIC_INTERVAL {
            self.last_diagnostic = Instant::now();
            log_limited(&format!(
                "music: host track {} state {} playing id {} event position {:?} ms, music_init position {:?} ms, master {:.2} layers {:?}",
                channel.track,
                channel.state,
                channel.playing_id,
                channel_position_ms(build, &channel),
                music_init_position_ms(build),
                channel.master_volume,
                channel.layer_volumes,
            ));
        }
    }

    fn log_host_change(&mut self, state: &MusicState, channel: &MusicChannel) {
        let changed = self.last_logged.is_none_or(|last| {
            last.track != state.track
                || last.stopping != state.stopping
                || last.master_volume != state.master_volume
                || last.layer_volumes != state.layer_volumes
        });
        if changed {
            self.last_logged = Some(*state);
            log_limited(&format!(
                "music: host sends track {} at {} ms (channel state {}, track volume {:.2}, stopping {}, master {:.2}, layers {:?})",
                state.track,
                state.position_ms,
                channel.state,
                state.track_volume,
                state.stopping,
                state.master_volume,
                state.layer_volumes,
            ));
        }
    }

    fn guest_tick(&mut self, build: &VerifiedBuild, view: &NetShared, arrived: Vec<Message>) {
        let now_ms = net_clock_ms();
        for message in arrived {
            if let Message::Music(state) = message {
                self.guest.receive(state, now_ms);
            }
        }
        let memory = InProcessMemory::new(build);
        let paused = read_world_flags(&memory)
            .map(|flags| flags.activity().world_paused())
            .unwrap_or(true);
        let fresh = self.guest.is_fresh(now_ms);
        let controlling = fresh && !paused && view.effective_local_readiness().is_in_play();
        set_host_owns_local_music(controlling);
        if self.was_controlling != controlling {
            self.was_controlling = controlling;
            log_limited(&format!(
                "music: the host's music {} (fresh {fresh}, local pause {paused})",
                if controlling {
                    "now drives this game"
                } else {
                    "no longer drives this game, the game's own music is allowed"
                }
            ));
        }
        if !fresh {
            self.guest.forget();
        }
        if !controlling {
            return;
        }
        let Ok(channel) = read_music_channel(&memory) else {
            return;
        };
        let local = LocalMusic {
            channel,
            position_ms: channel_position_ms(build, &channel),
        };
        let latency_ms = view
            .rtt
            .map_or(0, |rtt| u64::try_from(rtt.as_millis() / 2).unwrap_or(0));
        let steps = self.guest.plan(&local, now_ms, latency_ms);
        for step in steps {
            self.apply_step(build, &local, step, now_ms);
        }
        self.check_seek_result(&local, now_ms);
        if self.last_diagnostic.elapsed() >= DIAGNOSTIC_INTERVAL {
            self.last_diagnostic = Instant::now();
            log_limited(&format!(
                "music: guest track {} state {} at {:?} ms, host expected {:?} ms, {} local music calls ignored, music_init position {:?} ms",
                channel.track,
                channel.state,
                local.position_ms,
                self.guest.expected_position_ms(now_ms, latency_ms),
                take_ignored_calls(),
                music_init_position_ms(build),
            ));
        }
    }

    fn apply_step(&mut self, build: &VerifiedBuild, local: &LocalMusic, step: MusicStep, now: u64) {
        match step {
            MusicStep::Play { track, volume } => {
                play(build, track, volume, PLAY_FADE_SECONDS);
                log_limited(&format!(
                    "music: guest plays the host's track {track} (volume {volume:.2}, was track {})",
                    local.channel.track
                ));
            }
            MusicStep::Stop { fade_seconds } => {
                stop(build, fade_seconds);
                log_limited(&format!(
                    "music: guest stops its track {} (fade {fade_seconds:.1} s)",
                    local.channel.track
                ));
            }
            MusicStep::Seek { position_ms } => {
                let asked = i32::try_from(position_ms).unwrap_or(i32::MAX);
                let result = seek(
                    build,
                    local.channel.playing_id,
                    local.channel.game_object,
                    asked,
                );
                log_limited(&format!(
                    "music: guest seeks track {} to {position_ms} ms (was at {:?} ms, playing id {}, result {result:?})",
                    local.channel.track, local.position_ms, local.channel.playing_id
                ));
                self.seek_check = Some(SeekCheck {
                    asked_ms: position_ms,
                    asked_at_ms: now,
                });
            }
            MusicStep::SeekGaveUp { event_track, state } => log_limited(&format!(
                "music: guest cannot seek: the playing event belongs to track {event_track} and the channel is in state {state}"
            )),
            MusicStep::Master { volume } => {
                set_master_volume(build, volume, VOLUME_RAMP_SECONDS);
                log_limited(&format!("music: guest master volume {volume:.2}"));
            }
            MusicStep::Layer { layer, volume } => {
                set_layer_volume(build, i32::from(layer), volume, VOLUME_RAMP_SECONDS);
                log_limited(&format!("music: guest layer {layer} volume {volume:.2}"));
            }
        }
    }

    fn check_seek_result(&mut self, local: &LocalMusic, now_ms: u64) {
        let Some(check) = &self.seek_check else {
            return;
        };
        let waited = now_ms.saturating_sub(check.asked_at_ms);
        if waited < SEEK_CHECK_DELAY_MS {
            return;
        }
        let expected = u64::from(check.asked_ms) + waited;
        log_limited(&format!(
            "music: seek check: asked for {} ms, the event is at {:?} ms {waited} ms later (expected about {expected} ms)",
            check.asked_ms, local.position_ms
        ));
        self.seek_check = None;
    }
}

fn channel_position_ms(build: &VerifiedBuild, channel: &MusicChannel) -> Option<u32> {
    let ready = channel.state == BGM_CHANNEL_STATE_EVENT_POSTED
        && channel.has_track()
        && channel.event_track == channel.track;
    if ready {
        source_position_ms(build, channel.playing_id)
    } else {
        None
    }
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let arrived = net.take_relayed(|message| matches!(message, Message::Music(_)));
    let mut tick = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let connected = view.state == NetState::Connected;
    if connected && !tick.was_connected {
        tick.sender.send_next_now();
    }
    tick.was_connected = connected;
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Host if connected => {
            set_host_owns_local_music(false);
            tick.host_tick(build, &view);
        }
        FollowRole::Guest if connected => tick.guest_tick(build, &view, arrived),
        _ => {
            set_host_owns_local_music(false);
            tick.guest.forget();
            tick.was_controlling = false;
        }
    }
}
