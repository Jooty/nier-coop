use std::sync::PoisonError;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use rc_game::marker::{ActivityGate, read_world_flags};
use rc_game::offsets::BODY_MOTION_LAYER;
use rc_game::puppet::{read_layer_motion, read_weapon_look};
use rc_game::readiness::{REPORT_MIN_INTERVAL_MS, classify, read_play_checks};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::PartnerActivity;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{AutoConfig, LocalSnapshot, SEND_INTERVAL, handle, start_link};

static ACTIVITY_CLOCK: OnceLock<Instant> = OnceLock::new();
static LAST_PUBLISHED_AT: Mutex<Option<Instant>> = Mutex::new(None);
static ACTIVITY_GATE: Mutex<ActivityGate> = Mutex::new(ActivityGate::new());
static LAST_SENT_ACTIVITY: Mutex<PartnerActivity> = Mutex::new(PartnerActivity::Playing);
static LAST_REPORTED_AT: Mutex<Option<Instant>> = Mutex::new(None);

pub fn report_readiness(build: &VerifiedBuild) {
    let Some(net) = handle() else {
        return;
    };
    let now = Instant::now();
    let mut last = LAST_REPORTED_AT
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if last.is_some_and(|reported| {
        now.saturating_duration_since(reported).as_millis() < u128::from(REPORT_MIN_INTERVAL_MS)
    }) {
        return;
    }
    *last = Some(now);
    let game = Replicant::new(InProcessMemory::new(build));
    net.report_readiness(classify(read_play_checks(&game)));
}

pub fn publish_local_snapshot(build: &VerifiedBuild) {
    let Some(net) = handle() else {
        return;
    };
    if !net.wants_snapshots() {
        return;
    }
    let now = Instant::now();
    let mut last = LAST_PUBLISHED_AT
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if last.is_some_and(|published| now.saturating_duration_since(published) < SEND_INTERVAL) {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    if let Some(snapshot) = read_snapshot(&game) {
        *last = Some(now);
        crate::player_names::log_local_name_once(build);
        net.publish(snapshot);
    }
}

pub fn start_auto() {
    crate::frame_tuning::load_frame_tuning();
    if let Ok(Some(config)) = AutoConfig::load_beside_game() {
        crate::event_log::set_event_log_enabled(config.event_log_enabled());
    }
    match AutoConfig::load_beside_game() {
        Ok(Some(AutoConfig {
            link: Some(port), ..
        })) => {
            log_line(&format!("net: launcher link on port {port}"));
            start_link(port);
        }
        Ok(Some(_)) => log_line("net: replicant-coop.toml has no link port, co-op is off"),
        Ok(None) => log_line("net: no replicant-coop.toml, co-op is off"),
        Err(error) => log_line(&format!("net: {error}")),
    }
}

fn read_snapshot(game: &Replicant<InProcessMemory>) -> Option<LocalSnapshot> {
    let phase = u16::try_from(PhaseEntry::index_of(&game.current_phase().ok()?)?).ok()?;
    let position = game.position().ok()?;
    let actor = game.playable_actor().ok();
    let motion = actor
        .and_then(|actor| read_layer_motion(game.memory(), actor, BODY_MOTION_LAYER).ok())
        .flatten();
    let weapon = actor.and_then(|actor| read_weapon_look(game.memory(), actor).ok().flatten());
    Some(LocalSnapshot {
        phase,
        position: [position.x, position.y, position.z],
        yaw_radians: game.yaw_radians().ok()?,
        hp: game.health().ok()?,
        max_hp: game.max_health().ok()?,
        body_motion: motion,
        weapon,
        activity: settled_activity(game, motion.map(|m| (m.index, m.frame)), phase),
        mp: game.magic().unwrap_or(0.0),
        max_mp: game.max_magic().unwrap_or(0.0),
        level: game.level().unwrap_or(0),
    })
}

fn settled_activity(
    game: &Replicant<InProcessMemory>,
    motion: Option<(i32, f32)>,
    phase: u16,
) -> PartnerActivity {
    let flags = read_world_flags(game.memory()).ok();
    let raw = flags.map(|flags| flags.activity()).unwrap_or_default();
    let now_ms = ACTIVITY_CLOCK
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64;
    let settled = ACTIVITY_GATE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .settle(raw, motion, now_ms);
    let mut last = LAST_SENT_ACTIVITY
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if *last != settled {
        log_line(&format!(
            "activity: sending {settled:?} (was {:?}, raw {raw:?}, control mask {:#x}, event scene {}, phase {phase})",
            *last,
            flags.map_or(0, |flags| flags.control_mask),
            flags.is_some_and(|flags| flags.event_scene),
        ));
        *last = settled;
    }
    settled
}
