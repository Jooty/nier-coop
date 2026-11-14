use std::sync::{Mutex, PoisonError};

use replicant_game::coop_pause::MaskView;
use replicant_game::fight_sync::{SlotState, script_slot_state};
use replicant_game::own_ride::{
    DroppedRideStep, DroppedStepView, END_UNIQUE_ACTOR_SCRIPT_MODE_NATIVE_RVA,
    SET_PLAYER_DEFAULT_STATUS_NATIVE_RVA,
};
use replicant_game::script_runner::PAUSE_OFF_NATIVE_RVA;
use replicant_game::story_jump::PLAYER_UNIQUE_ACTOR_ID;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::script_manager_state;
use crate::net::net_clock_ms;
use crate::story_jump::{NativeArg, call_native_with};
use crate::story_relay::relay_calls_are_clear;

static DROPPED: Mutex<DroppedRideStep> = Mutex::new(DroppedRideStep::new());

/// The partner's copy of a boarding step was dropped here because the partner rides.
pub(crate) fn note_dropped_ride_step(step: &str) {
    DROPPED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_dropped(step, net_clock_ms());
}

/// A partner step arrived, or a step ran here.
pub(crate) fn note_step() {
    DROPPED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_step(net_clock_ms());
}

pub(super) fn forget_dropped_ride_step() {
    DROPPED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
}

/// Once per script tick: lets this game go when a dropped boarding step left it paused, as the
/// partner's ride end would have on the rider's game.
pub(crate) fn release_a_dropped_steps_pause(build: &VerifiedBuild, view: MaskView) {
    let mut dropped = DROPPED.lock().unwrap_or_else(PoisonError::into_inner);
    if !dropped.is_watching() {
        return;
    }
    let step_view = DroppedStepView {
        slot_zero: view.slots[0],
        script_slot_idle: script_slot_state(script_manager_state(build)) == SlotState::Idle,
        partner_calls_clear: relay_calls_are_clear(),
        something_open: view.extra_ui_scene_open
            || view.ui_dialog_state != 0
            || view.event_scene
            || view.talk_window_open,
        in_play: view.in_play,
    };
    let release = dropped.observe(step_view, net_clock_ms());
    drop(dropped);
    let Some(release) = release else {
        return;
    };
    call_native_with(
        build,
        END_UNIQUE_ACTOR_SCRIPT_MODE_NATIVE_RVA,
        &[NativeArg::Integer(PLAYER_UNIQUE_ACTOR_ID)],
    );
    call_native_with(build, SET_PLAYER_DEFAULT_STATUS_NATIVE_RVA, &[]);
    call_native_with(build, PAUSE_OFF_NATIVE_RVA, &[]);
    log_line(&format!(
        "ride: the partner's {} was dropped here {} ms ago (the partner rides) and left your game paused (mask {:#x}) with nothing running; the ride's end never runs here, so your body is let go and the pause taken off",
        release.step, release.after_ms, release.mask
    ));
}
