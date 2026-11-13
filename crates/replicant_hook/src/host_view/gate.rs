use std::sync::{Mutex, PoisonError};

use replicant_game::Replicant;
use replicant_game::scene_follow::{
    GUEST_TAKES_HOST_VIEW_WITHIN_UNITS, HostViewFacts, HostViewHold, HostViewVerdict,
    guest_to_host_units,
};
use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::Whereabouts;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::event_scene_running;
use crate::diagnostics::log_line;
use crate::game_hooks::host_led_run;
use crate::memory::InProcessMemory;
use crate::net::{self, NetSharedExt, NetState};
use crate::story_jump::mod_native_call_running;

pub(super) static HOLD: Mutex<HostViewHold> = Mutex::new(HostViewHold::new());
pub(super) static HELD_IN_PHASE: Mutex<Option<u16>> = Mutex::new(None);

/// Whether a scripted camera native may run here. `leaves_free_camera` is true for a switch
/// back to the free camera. The host's calls reach a guest only while it stands near the host.
pub(crate) fn camera_call_runs(
    build: &VerifiedBuild,
    native: &str,
    leaves_free_camera: bool,
    sets_a_scripted_camera: bool,
) -> bool {
    let Some((run, _)) = host_led_run() else {
        if !mod_native_call_running() {
            HOLD.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .own_camera_call();
        }
        return true;
    };
    let verdict = HostViewVerdict::judge(&HostViewFacts {
        gives_the_view_back: leaves_free_camera,
        ..facts_now(build, true)
    });
    if leaves_free_camera {
        HOLD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .host_camera_applied(&run, true);
    }
    match verdict {
        HostViewVerdict::NotGated => {}
        HostViewVerdict::Apply { units } => {
            if sets_a_scripted_camera {
                HOLD.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .host_camera_applied(&run, false);
            }
            note_held_phase();
            log_line(&format!(
                "host camera: the host's {native} from {run} applied here, you are {units:.1} units from the host (within {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS})"
            ));
        }
        HostViewVerdict::Leave { units } => log_line(&format!(
            "host camera: the host's {native} from {run} left out here, you are {} (more than {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS}), your own camera stays",
            units_text(units)
        )),
    }
    verdict.runs()
}

/// Whether a screen fade on the game's main fade layer may run here; the same rule as the camera.
pub(crate) fn fade_call_runs(build: &VerifiedBuild, fading_out: bool) -> bool {
    let Some((run, _)) = host_led_run() else {
        if !mod_native_call_running() {
            HOLD.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .own_fade_call();
        }
        return true;
    };
    let verdict = HostViewVerdict::judge(&HostViewFacts {
        gives_the_view_back: !fading_out,
        ..facts_now(build, true)
    });
    if !fading_out {
        HOLD.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .host_fade_applied(&run, false);
    }
    match verdict {
        HostViewVerdict::NotGated => {}
        HostViewVerdict::Apply { units } => {
            HOLD.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .host_fade_applied(&run, true);
            note_held_phase();
            log_line(&format!(
                "host fade: the host's fade out from {run} applied here, you are {units:.1} units from the host (within {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS})"
            ));
        }
        HostViewVerdict::Leave { units } => log_line(&format!(
            "host fade: the host's fade out from {run} left out here, you are {} (more than {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS}), your screen stays",
            units_text(units)
        )),
    }
    verdict.runs()
}

pub(super) fn facts_now(build: &VerifiedBuild, host_led_run: bool) -> HostViewFacts {
    let unknown = HostViewFacts {
        is_guest: false,
        called_by_the_mod: mod_native_call_running(),
        in_fight_window: false,
        in_event_scene: false,
        host_led_run,
        gives_the_view_back: false,
        guest_to_host_units: None,
    };
    let Some(net) = net::handle() else {
        return unknown;
    };
    let view = net.view();
    let is_guest = view.state == NetState::Connected
        && FollowRole::from_slot(view.local_slot) == FollowRole::Guest;
    let Some(local) = view.local.as_ref() else {
        return HostViewFacts {
            is_guest,
            ..unknown
        };
    };
    // The live body, read now: a move the mod or the scene made this tick counts already.
    let position = Replicant::new(InProcessMemory::new(build))
        .position()
        .map_or(local.position, |here| [here.x, here.y, here.z]);
    let host = view
        .partner_in_play()
        .then(|| view.partner_state())
        .flatten()
        .map(|remote| Whereabouts {
            phase: remote.state.phase,
            position: remote.state.position,
        });
    HostViewFacts {
        is_guest,
        in_fight_window: crate::fight_sync::covers_phase(local.phase),
        in_event_scene: event_scene_running(build) == Some(true),
        host_led_run,
        guest_to_host_units: guest_to_host_units(
            Some(Whereabouts {
                phase: local.phase,
                position,
            }),
            host,
        ),
        ..unknown
    }
}

fn note_held_phase() {
    let phase = net::handle().and_then(|net| net.view().local.as_ref().map(|local| local.phase));
    *HELD_IN_PHASE.lock().unwrap_or_else(PoisonError::into_inner) = phase;
}

pub(super) fn units_text(units: Option<f32>) -> String {
    units.map_or_else(
        || "in another phase than the host or your places are not known".to_owned(),
        |units| format!("{units:.1} units from the host"),
    )
}
