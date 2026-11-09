use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::script_rolls::{
    GET_RAND_F_RVA, GET_RAND_I_RVA, LUA_CALL_ARGUMENTS_OFFSET, LUA_CALL_RESULTS_OFFSET, RollFacts,
    RollNumber, number_from_slot, roll_is_shared, value_fits_arguments,
};
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::StoryCallKind;
use rc_game::story_relay::{
    ARG_STRIDE_BYTES, ARG_TAG_INT, ARG_TAG_NUMBER, ARG_VALUE_OFFSET, ARGS_HEADER_BYTES,
    pack_hash_of_phase, relay_kind,
};
use rc_game::{GameMemory, Replicant};
use rc_proto::{Message, ScriptRoll, ScriptRollValue};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, net_clock_ms};
use crate::script_rolls::receive::{current_phase_index, drain_arrived, forget_other_phases};
use crate::script_rolls::state::{
    HOOKED_BUILD, ORDINALS, ROLLS_MISSED, ROLLS_SENT, ROLLS_USED, STORE,
};
use crate::story_relay::shared_story_events;
use crate::story_writes::calls::running_call;

static ORIGINAL_GET_RAND_I: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_RAND_F: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type RollNative = unsafe extern "system" fn(*mut c_void);

#[derive(Clone, Copy, PartialEq, Eq)]
enum RollKind {
    Int,
    Float,
}

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the two script random number natives by address"
    )]
    // SAFETY: each RVA starts the native the replacement mirrors in the verified build
    // (_GetRandI, _GetRandF; see docs off-hand/story-choices.md).
    unsafe {
        hook_address(
            "LuaNative::GetRandI",
            (base + u64::from(GET_RAND_I_RVA)) as usize,
            get_rand_i_detour as *const () as *mut c_void,
            &ORIGINAL_GET_RAND_I,
        );
        hook_address(
            "LuaNative::GetRandF",
            (base + u64::from(GET_RAND_F_RVA)) as usize,
            get_rand_f_detour as *const () as *mut c_void,
            &ORIGINAL_GET_RAND_F,
        );
    }
}

extern "system" fn get_rand_i_detour(args: *mut c_void) {
    roll_through(args, RollKind::Int, &ORIGINAL_GET_RAND_I);
}

extern "system" fn get_rand_f_detour(args: *mut c_void) {
    roll_through(args, RollKind::Float, &ORIGINAL_GET_RAND_F);
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
fn roll_through(args: *mut c_void, kind: RollKind, original: &AtomicPtr<c_void>) {
    let trampoline = original.load(Ordering::Acquire);
    if trampoline.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, RollNative>(trampoline)(args) };
    let args_address = args as u64;
    // Inside a fight window's streamed or replayed handler the fight tape owns the read.
    if crate::fight_sync::tape_native_result(args_address, native_name(kind)) {
        return;
    }
    let _ = panic::catch_unwind(|| share_roll(args_address, kind));
}

fn share_roll(args: u64, kind: RollKind) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let role = FollowRole::from_slot(view.local_slot);
    if view.state != NetState::Connected || !shared_story_events() || role == FollowRole::Unknown {
        return;
    }
    let Some(call) = running_call() else {
        return;
    };
    let mut game = Replicant::new(InProcessMemory::new(build));
    let phase_name = game.current_phase().ok();
    let phase_hash = phase_name.as_deref().map(pack_hash_of_phase);
    let relayed_story_call = phase_hash
        .and_then(|pack| relay_kind(pack, call.name_hash))
        .is_some_and(StoryCallKind::is_relayed);
    let facts = RollFacts {
        function_hash: call.name_hash,
        phase_hash,
        relayed_story_call,
        from_talk_tag: call.from_talk_tag,
    };
    if !roll_is_shared(&facts) {
        return;
    }
    let Some(phase) = current_phase_index(&game) else {
        return;
    };
    let result_slot = args + LUA_CALL_RESULTS_OFFSET + ARGS_HEADER_BYTES as u64;
    let Some(rolled) = read_rolled(game.memory(), result_slot, kind) else {
        return;
    };
    let ordinal = {
        let mut ordinals = ORDINALS.lock().unwrap_or_else(PoisonError::into_inner);
        let ordinal = ordinals.peek(call.name_hash, call.started_ms);
        ordinals.commit(call.name_hash, call.started_ms);
        ordinal
    };
    let ids = format!(
        "fn={:016x} ordinal={ordinal} kind={}",
        call.name_hash,
        kind_name(kind)
    );
    if role == FollowRole::Host {
        let roll = ScriptRoll {
            phase,
            call: call.name_hash,
            ordinal,
            value: rolled,
        };
        net.send_relayed(Message::ScriptRoll(roll));
        ROLLS_SENT.fetch_add(1, Ordering::Relaxed);
        log_line(&format!(
            "script roll: {ids} value={} sent",
            describe(rolled)
        ));
        return;
    }
    drain_arrived(net);
    forget_other_phases(phase);
    let shared = STORE.lock().unwrap_or_else(PoisonError::into_inner).take(
        phase,
        call.name_hash,
        ordinal,
        net_clock_ms(),
    );
    let Some(shared) = shared else {
        ROLLS_MISSED.fetch_add(1, Ordering::Relaxed);
        log_line(&format!(
            "script roll: {ids} miss, kept own value={}",
            describe(rolled)
        ));
        return;
    };
    if !within_own_bounds(game.memory(), args, shared) {
        ROLLS_MISSED.fetch_add(1, Ordering::Relaxed);
        log_line(&format!(
            "script roll: {ids} miss, host value={} is outside this call's bounds",
            describe(shared)
        ));
        return;
    }
    if write_rolled(game.memory_mut(), result_slot, shared) {
        ROLLS_USED.fetch_add(1, Ordering::Relaxed);
        log_line(&format!(
            "script roll: {ids} value={} used (own was {})",
            describe(shared),
            describe(rolled)
        ));
    }
}

fn read_rolled(
    memory: &InProcessMemory,
    result_slot: u64,
    kind: RollKind,
) -> Option<ScriptRollValue> {
    let tag = memory.read_i32(result_slot).ok()?;
    let payload = memory
        .read_array::<8>(result_slot + ARG_VALUE_OFFSET as u64)
        .ok()?;
    let RollNumber(number) = number_from_slot(tag, payload)?;
    match (kind, tag) {
        (RollKind::Int, ARG_TAG_INT) => Some(ScriptRollValue::Int(number as i32)),
        (RollKind::Float, ARG_TAG_NUMBER) => Some(ScriptRollValue::Float(number as f32)),
        _ => None,
    }
}

fn write_rolled(memory: &mut InProcessMemory, result_slot: u64, value: ScriptRollValue) -> bool {
    let payload_address = result_slot + ARG_VALUE_OFFSET as u64;
    let written = match value {
        ScriptRollValue::Int(int) => memory.write(payload_address, &int.to_le_bytes()),
        ScriptRollValue::Float(float) => {
            memory.write(payload_address, &f64::from(float).to_le_bytes())
        }
    };
    written.is_ok()
}

fn within_own_bounds(memory: &InProcessMemory, args: u64, value: ScriptRollValue) -> bool {
    let argument_count = memory
        .read_i32(args + LUA_CALL_ARGUMENTS_OFFSET)
        .unwrap_or(0);
    let argument_slot = |index: u64| {
        args + LUA_CALL_ARGUMENTS_OFFSET
            + ARGS_HEADER_BYTES as u64
            + index * ARG_STRIDE_BYTES as u64
    };
    let read_bound = |index: u64| {
        let slot = argument_slot(index);
        let tag = memory.read_i32(slot).ok()?;
        let payload = memory
            .read_array::<8>(slot + ARG_VALUE_OFFSET as u64)
            .ok()?;
        number_from_slot(tag, payload)
    };
    if argument_count < 2 {
        return value_fits_arguments(value, argument_count, None, None);
    }
    value_fits_arguments(value, argument_count, read_bound(0), read_bound(1))
}

fn native_name(kind: RollKind) -> &'static str {
    match kind {
        RollKind::Int => "_GetRandI",
        RollKind::Float => "_GetRandF",
    }
}

fn kind_name(kind: RollKind) -> &'static str {
    match kind {
        RollKind::Int => "int",
        RollKind::Float => "float",
    }
}

fn describe(value: ScriptRollValue) -> String {
    match value {
        ScriptRollValue::Int(int) => int.to_string(),
        ScriptRollValue::Float(float) => format!("{float:.4}"),
    }
}
