use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::callback_gate::ParkedKind;
use rc_game::dummy_hits::{
    DummyHitArrival, DummyHitArrivalFacts, DummyHitCallbacks, DummyHitFire, DummyHitFireFacts,
    DummyHitOrigin, PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA, dummy_callback_name, dummy_hit_arrival,
    dummy_hit_fire, dummy_hit_log_line, dummy_native_args_len,
};
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::name_hash;
use rc_game::story_relay::{CallKey, args_hash};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, net_clock_ms};
use crate::story_relay::inject::queue_call;
use crate::story_relay::parked::deliver;
use crate::story_relay::settings::shared_story_events;
use crate::story_relay::state::{HOOKED_BUILD, LAST_RUN_STARTED_BY_INJECTION_AT_MS, LEDGER};

static CALLBACKS: Mutex<DummyHitCallbacks> = Mutex::new(DummyHitCallbacks::new());
static ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void);

pub(super) struct FireContext<'a> {
    pub phase: u16,
    pub function_name: &'a str,
    pub arguments: u64,
    pub call: Option<StoryCall>,
}

pub(super) fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the _PutActorDamageDummy script native by address to learn its callback names"
    )]
    // SAFETY: the RVA starts _PutActorDamageDummy in the verified build (binding table 0x1306710,
    // thunk 0x6E10C0), which takes the Lua argument block as its only argument.
    unsafe {
        hook_address(
            "LuaNative::PutActorDamageDummy",
            (build.module_base() + u64::from(PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA)) as usize,
            put_actor_damage_dummy_detour as *const () as *mut c_void,
            &ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY,
        );
    }
}

pub(super) fn is_registered(phase: u16, callback: &str) -> bool {
    CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_registered(phase, callback)
}

pub(super) fn on_fire(context: &FireContext) -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let injected = LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_injected_by_pointer(context.arguments, net_clock_ms());
    *LAST_RUN_STARTED_BY_INJECTION_AT_MS
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = injected.then(net_clock_ms);
    let facts = DummyHitFireFacts {
        registered: true,
        injected,
        enabled: shared_story_events(),
        connected: view.state == NetState::Connected,
        partner_here: view.partner_place().is_same(),
        args_relayable: context.call.is_some(),
    };
    let role = FollowRole::from_slot(view.local_slot);
    let origin = match dummy_hit_fire(&facts) {
        DummyHitFire::NotADummyHit => return false,
        DummyHitFire::RunForPartner => DummyHitOrigin::partner_of(role),
        DummyHitFire::RunAndSend => {
            if let Some(call) = &context.call {
                net.send_relayed(Message::DummyHit(call.clone()));
            }
            DummyHitOrigin::Here
        }
        DummyHitFire::RunHereOnly => DummyHitOrigin::Here,
    };
    let runs_here = CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .count_run(context.phase, context.function_name);
    log_line(&dummy_hit_log_line(
        context.function_name,
        origin,
        runs_here,
    ));
    false
}

pub(super) fn on_arrival(
    build: &VerifiedBuild,
    call: &StoryCall,
    local_in_play: bool,
    local_phase: Option<u16>,
) {
    let facts = DummyHitArrivalFacts {
        enabled: shared_story_events(),
        local_in_play,
        local_phase,
        call_phase: call.phase,
        registered_here: is_registered(call.phase, &call.function),
    };
    match dummy_hit_arrival(&facts) {
        DummyHitArrival::Drop(reason) => log_line(&format!(
            "dummy hit: {} from the partner not run here ({reason:?})",
            call.function
        )),
        DummyHitArrival::Inject => deliver(build, ParkedKind::DummyHit, call),
    }
}

pub(super) fn inject_arrival(build: &VerifiedBuild, call: &StoryCall) {
    let key = CallKey {
        function: name_hash(&call.function),
        args: args_hash(&call.args),
    };
    let Some(arguments_pointer) = queue_call(build, call) else {
        log_line(&format!(
            "dummy hit: {} from the partner not queued (queue full)",
            call.function
        ));
        return;
    };
    LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_injected(arguments_pointer, key, net_clock_ms());
}

#[expect(
    unsafe_code,
    reason = "copies the native's argument block and calls the original through the trampoline"
)]
extern "system" fn put_actor_damage_dummy_detour(args: *mut c_void) {
    let args_address = args as u64;
    let name = panic::catch_unwind(|| read_callback_name(args_address)).ok();
    let trampoline = ORIGINAL_PUT_ACTOR_DAMAGE_DUMMY.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, LuaNative>(trampoline)(args) };
    }
    if let Some(name) = name {
        let _ = panic::catch_unwind(|| note_registered(name));
    }
}

#[expect(
    unsafe_code,
    reason = "reads the native's argument block, which the native reads itself"
)]
fn read_callback_name(args_address: u64) -> Result<String, String> {
    let mut block = vec![0_u8; dummy_native_args_len()];
    // SAFETY: the native reads the same bytes (its first three arguments) from this block.
    unsafe {
        ptr::copy_nonoverlapping(args_address as *const u8, block.as_mut_ptr(), block.len());
    }
    dummy_callback_name(&block).map_err(|refusal| format!("{refusal:?}"))
}

fn note_registered(name: Result<String, String>) {
    let name = match name {
        Ok(name) => name,
        Err(refusal) => {
            log_line(&format!(
                "dummy hit: _PutActorDamageDummy callback name unreadable ({refusal})"
            ));
            return;
        }
    };
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    let Some(phase) = phase else {
        log_line(&format!(
            "dummy hit: {name} not recorded, the phase is unreadable"
        ));
        return;
    };
    let recorded = CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .register(phase, &name);
    log_line(&format!(
        "dummy hit: _PutActorDamageDummy callback {name} recorded for phase {phase} ({})",
        if recorded { "ok" } else { "table full" }
    ));
}
