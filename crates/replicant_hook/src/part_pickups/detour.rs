use core::ffi::c_void;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use replicant_game::containers::{ACTION_RECORD_REMOVE_FUNCTION_RVA, ACTION_RECORD_TABLE_RVA};
use replicant_game::part_pickups::{
    PART_PICKUP_ERRANDS, PART_VISIBILITY_FUNCTION_RVA, PartPickupErrand, local_pickup_to_send,
};
use replicant_game::{GameMemory, PhaseEntry, Replicant};
use replicant_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState};
use crate::part_pickups::state::{
    HOOKED_BUILD, ORIGINAL_ACTOR_HANDLER, ORIGINAL_STATE_STEP, PENDING,
};
use crate::story_relay::shared_story_events;

type ActorHandler = unsafe extern "system" fn(*mut c_void, u32, u64, u64, u64);
type StateStep = unsafe extern "system" fn(*mut c_void, *mut c_void, f32) -> u64;
type SetPartVisible = unsafe extern "system" fn(*mut c_void, u64, u8, u64) -> u8;
type RemoveActionRecord = unsafe extern "system" fn(*mut c_void, i32, u8);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let [errand] = &PART_PICKUP_ERRANDS;
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the errand actor's event handler and its counting state step by address"
    )]
    // SAFETY: both RVAs start the functions named in the part pickup table for the verified
    // build (docs off-hand/pickups.md); each replacement takes the same arguments and passes
    // them on unchanged.
    unsafe {
        hook_address(
            "part pickups: actor event handler",
            (base + u64::from(errand.actor_handler_rva)) as usize,
            actor_handler_detour as *const () as *mut c_void,
            &ORIGINAL_ACTOR_HANDLER,
        );
        hook_address(
            "part pickups: counting state step",
            (base + u64::from(errand.state_step_rva)) as usize,
            state_step_detour as *const () as *mut c_void,
            &ORIGINAL_STATE_STEP,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original handler through the trampoline MinHook built"
)]
extern "system" fn actor_handler_detour(
    actor: *mut c_void,
    event: u32,
    other: u64,
    part_group: u64,
    extra: u64,
) {
    let original = ORIGINAL_ACTOR_HANDLER.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original body, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ActorHandler>(original)(
            actor, event, other, part_group, extra,
        );
    };
    let _ = panic::catch_unwind(|| send_local_pickup(event, part_group));
}

#[expect(
    unsafe_code,
    reason = "calls the original state step through the trampoline MinHook built"
)]
extern "system" fn state_step_detour(
    state: *mut c_void,
    actor: *mut c_void,
    step_seconds: f32,
) -> u64 {
    let original = ORIGINAL_STATE_STEP.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let _ = panic::catch_unwind(|| apply_partner_pickups(state, actor));
    // SAFETY: the trampoline continues into the original body, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, StateStep>(original)(state, actor, step_seconds) }
}

fn send_local_pickup(event: u32, part_group: u64) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let [errand] = &PART_PICKUP_ERRANDS;
    if event != errand.pickup_event {
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let connected = net.view().state == NetState::Connected && shared_story_events();
    let phase = current_phase_index(build);
    let Some(pickup) = local_pickup_to_send(
        errand.actor_handler_rva,
        event,
        part_group,
        connected,
        phase,
    ) else {
        return;
    };
    net.send_relayed(Message::PartPickup(pickup));
    log_line(&format!(
        "part pickups: you picked part {} in zone {}, sent to the partner",
        pickup.part_group, pickup.phase
    ));
}

fn apply_partner_pickups(state: *mut c_void, actor: *mut c_void) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    if state.is_null() || actor.is_null() {
        return;
    }
    let [errand] = &PART_PICKUP_ERRANDS;
    let memory = InProcessMemory::new(build);
    let Ok(stage) = memory.read_i32(state as u64 + errand.state_stage_offset) else {
        return;
    };
    let phase = current_phase_index(build);
    let parts = PENDING
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_for_counting_step(errand, stage, phase);
    for part_group in parts {
        hide_picked_part(build, errand, actor, part_group);
        log_line(&format!(
            "part pickups: the partner picked part {part_group}, hidden here and its pickup point removed"
        ));
    }
}

#[expect(
    unsafe_code,
    reason = "calls the game's own part visibility setter and action record remover by address"
)]
fn hide_picked_part(
    build: &VerifiedBuild,
    errand: &PartPickupErrand,
    actor: *mut c_void,
    part_group: u16,
) {
    let base = build.module_base();
    let set_visible = (base + u64::from(PART_VISIBILITY_FUNCTION_RVA)) as *const ();
    let records = (base + u64::from(ACTION_RECORD_TABLE_RVA)) as *mut c_void;
    let remove_record = (base + u64::from(ACTION_RECORD_REMOVE_FUNCTION_RVA)) as *const ();
    debug_assert!(errand.has_part_group(part_group));
    // SAFETY: the RVAs start the functions the actor's own pickup event calls in the verified
    // build: (actor, part, shown, 0) and (record table, id, clear prompt); `actor` is the one
    // the game passed to its own counting step this frame; this runs on the game thread.
    unsafe {
        core::mem::transmute::<*const (), SetPartVisible>(set_visible)(
            actor,
            u64::from(part_group),
            0,
            0,
        );
        core::mem::transmute::<*const (), RemoveActionRecord>(remove_record)(
            records,
            i32::from(part_group),
            1,
        );
    }
}

pub(super) fn current_phase_index(build: &VerifiedBuild) -> Option<u16> {
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game.current_phase().ok()?;
    PhaseEntry::index_of(&phase).and_then(|index| u16::try_from(index).ok())
}
