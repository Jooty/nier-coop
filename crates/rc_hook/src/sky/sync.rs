use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rc_game::sky::{SkyBroadcast, SkyHold, read_sky, write_weather_counters};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;
use crate::sky::calls::SkyCalls;

static SYNC: LazyLock<Mutex<SkySync>> = LazyLock::new(|| Mutex::new(SkySync::default()));
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Default)]
struct SkySync {
    broadcast: SkyBroadcast,
    hold: SkyHold,
    was_host: Option<bool>,
}

pub fn tick_sky_sync(build: &VerifiedBuild) {
    let mut sync = SYNC.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(net) = net::handle() else {
        return;
    };
    let incoming = net.take_relayed(|message| matches!(message, Message::Sky(_)));
    let view = net.view();
    let Some(remote) = view.remote else {
        sync.broadcast.reset();
        sync.hold.reset();
        sync.was_host = None;
        return;
    };
    let is_host = remote.slot == 1;
    if sync.was_host != Some(is_host) {
        sync.broadcast.reset();
        sync.hold.reset();
        sync.was_host = Some(is_host);
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = game
        .current_phase()
        .ok()
        .and_then(|name| PhaseEntry::index_of(&name))
        .and_then(|index| u16::try_from(index).ok())
    else {
        return;
    };
    let Ok(reading) = read_sky(game.memory()) else {
        return;
    };
    let now_ms = u64::try_from(EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX);
    if is_host {
        let guest_phase = Some(remote.state.phase);
        if let Some(send) = sync.broadcast.next(now_ms, phase, &reading, guest_phase) {
            log_line(&format!(
                "sky: host sends ({}) phase {} env {} blend {:.2} wind {} weather {}/{}",
                send.reason,
                send.state.phase,
                send.state.env_area,
                send.state.env_blend_seconds,
                send.state.wind_area,
                send.state.weather,
                send.state.weather_offset,
            ));
            net.send_relayed(Message::Sky(send.state));
        }
        return;
    }
    for message in incoming {
        if let Message::Sky(state) = message {
            log_line(&format!(
                "sky: guest got phase {} env {} wind {} weather {}/{} (guest phase {phase}, guest env {})",
                state.phase,
                state.env_area,
                state.wind_area,
                state.weather,
                state.weather_offset,
                reading.env_area,
            ));
            sync.hold.receive(state);
        }
    }
    let apply = sync.hold.step(now_ms, phase, &reading);
    if apply.is_empty() {
        return;
    }
    let calls = SkyCalls::new(build);
    if let Some((weather, offset)) = apply.weather {
        let mut memory = InProcessMemory::new(build);
        let result = write_weather_counters(&mut memory, weather, offset);
        log_line(&format!(
            "sky: guest sets weather {weather}/{offset} (was {}/{}, ok {})",
            reading.weather,
            reading.weather_offset,
            result.is_ok()
        ));
    }
    if let Some((area, blend)) = apply.env {
        calls.set_environment_area(area, blend);
        log_line(&format!(
            "sky: guest sets env {area} blend {blend:.2} (was {})",
            reading.env_area
        ));
    }
    if let Some((area, blend)) = apply.wind {
        calls.set_wind_area(area, blend);
        log_line(&format!(
            "sky: guest sets wind {area} blend {blend:.2} (was {})",
            reading.wind_area
        ));
    }
}
