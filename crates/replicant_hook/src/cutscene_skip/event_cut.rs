use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::scene_follow::{
    EVENT_CUT_AUDIO_NAME_RVA, EVENT_CUT_EVENT_NAME_RVA, EVENT_CUT_LOADED_BYTE_RVA,
    EVENT_CUT_NAME_BUFFER_BYTES, LocalCut, UnplayedStarts, cut_load_call, cut_name_from_buffer,
    host_scene_cut,
};
use replicant_game::stay_together::FollowRole;
use replicant_proto::SceneCut;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, net_clock_ms};
use crate::story_relay::{RelayPath, submit_relay_call};

static UNPLAYED: Mutex<UnplayedStarts> = Mutex::new(UnplayedStarts::new());
static UNPLAYED_START_ENDED_MS: Mutex<Option<u64>> = Mutex::new(None);

type CutBuffer = [u8; EVENT_CUT_NAME_BUFFER_BYTES];

pub(super) fn host_cut(build: &VerifiedBuild) -> Option<SceneCut> {
    let (loaded, event, audio) = read_cut(build)?;
    host_scene_cut(loaded, &event, &audio)
}

pub(super) fn local_cut(build: &VerifiedBuild) -> LocalCut {
    match read_cut(build) {
        Some((loaded, event, _)) => LocalCut {
            loaded,
            event: cut_name_from_buffer(&event),
        },
        None => LocalCut {
            loaded: false,
            event: None,
        },
    }
}

pub(super) fn queue_cut_load(phase: u16, cut: &SceneCut, local: &LocalCut) -> bool {
    let accepted = submit_relay_call(&cut_load_call(phase, cut), RelayPath::SceneFollow);
    log_line(&format!(
        "scene follow: your game has {} loaded, the host's scene plays cut {} (audio {}), loading it first (handed to the relay queue {accepted})",
        describe_local(local),
        cut.event,
        if cut.audio.is_empty() {
            "none"
        } else {
            cut.audio.as_str()
        }
    ));
    accepted
}

pub(super) fn take_unplayed_start_ended() -> Option<u64> {
    UNPLAYED_START_ENDED_MS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
}

pub fn note_scene_start_popped(name: &str) {
    UNPLAYED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_popped(name, net_clock_ms());
}

pub fn note_no_cut_reveal() {
    UNPLAYED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_no_cut_reveal(net_clock_ms());
}

pub fn drop_end_of_an_unplayed_start(build: &VerifiedBuild, name: &str) -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let guest = FollowRole::from_slot(view.local_slot) == FollowRole::Guest
        && view.state == NetState::Connected;
    if !guest {
        return false;
    }
    let cut_loaded = local_cut(build).loaded;
    let now = net_clock_ms();
    let start = UNPLAYED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_unplayed_start(name, cut_loaded, now);
    let Some(start) = start else {
        return false;
    };
    *UNPLAYED_START_ENDED_MS
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(now);
    log_line(&format!(
        "scene follow: dropped {name}: your game had no event cut loaded when {start} ran, so the engine ended the scene unplayed (no cover, no zone change); it starts again once the cut is loaded"
    ));
    true
}

fn describe_local(local: &LocalCut) -> String {
    match (&local.event, local.loaded) {
        (Some(event), true) => format!("cut {event}"),
        _ => String::from("no cut"),
    }
}

fn read_cut(build: &VerifiedBuild) -> Option<(bool, CutBuffer, CutBuffer)> {
    let memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let loaded = memory
        .read_u8(base + u64::from(EVENT_CUT_LOADED_BYTE_RVA))
        .ok()?
        != 0;
    let event = memory
        .read_array::<EVENT_CUT_NAME_BUFFER_BYTES>(base + u64::from(EVENT_CUT_EVENT_NAME_RVA))
        .ok()?;
    let audio = memory
        .read_array::<EVENT_CUT_NAME_BUFFER_BYTES>(base + u64::from(EVENT_CUT_AUDIO_NAME_RVA))
        .ok()?;
    Some((loaded, event, audio))
}
