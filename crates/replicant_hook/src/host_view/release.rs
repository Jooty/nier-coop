use std::sync::PoisonError;

use replicant_game::fight_sync::CHANGE_CAMERA_TO_FREE_CAMERA_KIND;
use replicant_game::scene_follow::{
    GUEST_TAKES_HOST_VIEW_WITHIN_UNITS, HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS, HostViewKind,
};
use replicant_game::side_view::CHANGE_CAMERA_NATIVE_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::host_view::gate::{HELD_IN_PHASE, HOLD, facts_now, units_text};
use crate::net::{self, net_clock_ms};
use crate::story_jump::{NativeArg, call_native_with};

const HOST_CAMERA_RELEASE_BLEND_SECONDS: f64 = 1.0;
const HOST_CAMERA_RELEASE_BLEND_KIND: i32 = 1;

pub(crate) fn tick(build: &VerifiedBuild) {
    if !HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .holds_any()
    {
        return;
    }
    let facts = facts_now(build, false);
    let phase = net::handle().and_then(|net| net.view().local.as_ref().map(|local| local.phase));
    let mut hold = HOLD.lock().unwrap_or_else(PoisonError::into_inner);
    let held_in_phase = *HELD_IN_PHASE.lock().unwrap_or_else(PoisonError::into_inner);
    if !facts.is_guest || held_in_phase != phase {
        hold.reset();
        return;
    }
    if facts.in_fight_window || facts.in_event_scene {
        return;
    }
    let releases = hold.step(facts.guest_to_host_units, net_clock_ms());
    drop(hold);
    for release in releases {
        match release.kind {
            HostViewKind::Camera => {
                log_line(&format!(
                    "host camera: you are {} (more than {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS} for {HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS} ms), the host's camera from {} is released, your free camera is back",
                    units_text(facts.guest_to_host_units),
                    release.run
                ));
                call_native_with(
                    build,
                    CHANGE_CAMERA_NATIVE_RVA,
                    &[
                        NativeArg::Integer(CHANGE_CAMERA_TO_FREE_CAMERA_KIND),
                        NativeArg::Real(HOST_CAMERA_RELEASE_BLEND_SECONDS),
                        NativeArg::Integer(HOST_CAMERA_RELEASE_BLEND_KIND),
                    ],
                );
            }
            HostViewKind::Fade => {
                let faded_in = crate::fade_sync::fade_guest_back_in();
                log_line(&format!(
                    "host fade: you are {} (more than {GUEST_TAKES_HOST_VIEW_WITHIN_UNITS} for {HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS} ms), the host's fade out from {} is released (faded back in {faded_in})",
                    units_text(facts.guest_to_host_units),
                    release.run
                ));
            }
        }
    }
}
