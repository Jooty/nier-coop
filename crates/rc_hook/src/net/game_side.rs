use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::Instant;

use rc_game::{PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{AutoConfig, LocalSnapshot, SEND_INTERVAL, handle, start_link};

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
    match AutoConfig::load_from_folder(&folder) {
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
