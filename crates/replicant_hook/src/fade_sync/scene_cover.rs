use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::fade_sync::{EVENT_FADE_LAYER_RVA, NO_CUT_REVEAL_FADE_KIND, SceneCovers};
use replicant_game::stay_together::FollowRole;

use crate::diagnostics::log_line;
use crate::fade_sync::hooks::{now_ms, reveal_layer};
use crate::net::{self, NetState};

const COVER_LIFT_FADE_KIND: i32 = 2;
const COVER_LIFT_FADE_SECONDS: f32 = 1.0;
const COVER_LIFT_FADE_DEPTH: u32 = 0x20;

static COVERS: Mutex<SceneCovers> = Mutex::new(SceneCovers::new());
static HOST_SCENE_RUNNING: AtomicBool = AtomicBool::new(false);
static LOCAL_IN_SCENE: AtomicBool = AtomicBool::new(false);

pub fn note_scene_follow_state(host_scene_running: bool, local_in_scene: bool) {
    HOST_SCENE_RUNNING.store(host_scene_running, Ordering::Relaxed);
    LOCAL_IN_SCENE.store(local_in_scene, Ordering::Relaxed);
}

pub fn step_scene_covers(host_in_scene: bool, follow_gave_up: bool, local_in_scene: bool) {
    let lifted = COVERS.lock().unwrap_or_else(PoisonError::into_inner).step(
        host_in_scene,
        follow_gave_up,
        local_in_scene,
    );
    let Some((reason, covers)) = lifted else {
        return;
    };
    let now = now_ms();
    for cover in covers {
        let done = reveal_layer(
            cover.layer,
            COVER_LIFT_FADE_KIND,
            COVER_LIFT_FADE_SECONDS,
            COVER_LIFT_FADE_DEPTH,
        );
        log_line(&format!(
            "fade: lifted layer {:#x} that your game covered {} ms ago (by {}) for a scene it did not play: {} (fade in called {done})",
            cover.layer,
            now.saturating_sub(cover.covered_ms),
            cover.by,
            reason.describe()
        ));
    }
}

pub(super) fn note_game_fade(base: u64, layer: u64, fading_out: bool, kind: i32, seconds: f32) {
    let event_layer = base + u64::from(EVENT_FADE_LAYER_RVA);
    if !fading_out && layer == event_layer && kind == NO_CUT_REVEAL_FADE_KIND && seconds <= 0.0 {
        crate::cutscene_skip::note_no_cut_reveal();
    }
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let guest = FollowRole::from_slot(view.local_slot) == FollowRole::Guest
        && view.state == NetState::Connected;
    if !guest {
        return;
    }
    let mut covers = COVERS.lock().unwrap_or_else(PoisonError::into_inner);
    if !fading_out {
        covers.note_reveal(layer);
        return;
    }
    let by = crate::game_hooks::last_pop().map_or_else(|| String::from("?"), |(name, _)| name);
    let kept = covers.note_cover(
        layer,
        &by,
        now_ms(),
        HOST_SCENE_RUNNING.load(Ordering::Relaxed),
        LOCAL_IN_SCENE.load(Ordering::Relaxed),
    );
    drop(covers);
    if kept {
        log_line(&format!(
            "fade: your game covered layer {layer:#x} (kind {kind}, {seconds:.2} s, by {by}) while the host's scene runs and yours does not; lifted if the follow gives up or the host's scene ends"
        ));
    }
}
