use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::Instant;

use rc_game::{PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{
    AutoConfig, AutoMode, LocalSnapshot, NetCommand, SEND_INTERVAL, handle, set_player_name,
};

static LAST_PUBLISHED_AT: Mutex<Option<Instant>> = Mutex::new(None);

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
        net.publish(snapshot);
    }
}

pub fn start_auto() {
    let folder = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf));
    let Some(folder) = folder else {
        return;
    };
    let config = match AutoConfig::load_from_folder(&folder) {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(error) => {
            log_line(&format!("net: {error}"));
            return;
        }
    };
    if let Some(name) = config.name.clone() {
        set_player_name(name);
    }
    let Some(net) = handle() else {
        return;
    };
    match (config.mode, config.ticket) {
        (None, _) | (Some(AutoMode::Join), None) => return,
        (Some(AutoMode::Host), _) => net.send_command(NetCommand::Host),
        (Some(AutoMode::Join), Some(ticket)) => net.send_command(NetCommand::Join(ticket)),
    }
    log_line(&format!("net: auto mode {:?}", config.mode));
}

fn read_snapshot(game: &Replicant<InProcessMemory>) -> Option<LocalSnapshot> {
    let phase = u16::try_from(PhaseEntry::index_of(&game.current_phase().ok()?)?).ok()?;
    let position = game.position().ok()?;
    let motion = game.body_motion().ok().flatten();
    Some(LocalSnapshot {
        phase,
        position: [position.x, position.y, position.z],
        yaw_radians: game.yaw_radians().ok()?,
        hp: game.health().ok()?,
        max_hp: game.max_health().ok()?,
        body_motion_index: motion.map(|motion| motion.index),
        body_motion_frame: motion.map_or(0.0, |motion| motion.frame),
    })
}
