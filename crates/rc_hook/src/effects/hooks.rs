use core::ffi::{c_char, c_void};
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use rc_game::effects::{
    MAX_EFFECT_EVENTS_PER_SECOND, RateWindow, SentEffects, effect_is_finished, read_effect,
    read_sound,
};
use rc_game::marker::read_world_flags;
use rc_game::offsets::{
    ACTOR_PLAY_EFFECT_FUNCTION_RVA, ACTOR_PLAY_SOUND_FUNCTION_RVA, EFFECT_INVALID_HANDLE,
};
use rc_game::{GameMemory, Replicant};
use rc_proto::{EffectStop, Message, SoundPlay};

use crate::build_gate::VerifiedBuild;
use crate::effects::report::{
    REFUSED_BY_RATE_LIMIT, SENT_EFFECTS, SENT_SOUNDS, SENT_STOPS, UNREADABLE, count, log_limited,
};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_PLAY_EFFECT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PLAY_SOUND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SENDING: AtomicBool = AtomicBool::new(false);
static SENT_HANDLES: LazyLock<Mutex<SentEffects>> =
    LazyLock::new(|| Mutex::new(SentEffects::new()));
static SEND_WINDOW: LazyLock<Mutex<RateWindow>> =
    LazyLock::new(|| Mutex::new(RateWindow::new(1000, MAX_EFFECT_EVENTS_PER_SECOND)));

type PlayEffect =
    unsafe extern "system" fn(*mut c_void, *mut u32, *const u8, *const u8) -> *mut u32;
type PlaySound = unsafe extern "system" fn(*mut c_void, *const c_char, *mut c_void) -> *mut c_void;

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the actor play-effect function by address"
    )]
    // SAFETY: the RVA starts the virtual function (actor, out handle, parameters, attach
    // descriptor -> handle pointer) in the verified build, and the replacement matches it.
    unsafe {
        hook_address(
            "CActor::PlayEffect",
            (base + u64::from(ACTOR_PLAY_EFFECT_FUNCTION_RVA)) as usize,
            play_effect_detour as *const () as *mut c_void,
            &ORIGINAL_PLAY_EFFECT,
        )
    };
    #[expect(unsafe_code, reason = "hooks the actor play-sound function by address")]
    // SAFETY: the RVA starts the function (actor, sound name, emitter -> emitter handle) in the
    // verified build, and the replacement matches it.
    unsafe {
        hook_address(
            "CActor::PlaySound",
            (base + u64::from(ACTOR_PLAY_SOUND_FUNCTION_RVA)) as usize,
            play_sound_detour as *const () as *mut c_void,
            &ORIGINAL_PLAY_SOUND,
        )
    };
}

pub(super) fn set_sending(sending: bool) {
    SENDING.store(sending, Ordering::Relaxed);
    if !sending {
        SENT_HANDLES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

pub(super) fn send_ended_effects(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let ended = SENT_HANDLES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_ended(|handle| effect_is_finished(&memory, handle).unwrap_or(true));
    for handle in ended {
        net.send_relayed(Message::EffectStop(EffectStop { handle }));
        count(&SENT_STOPS);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original play-effect function through the trampoline MinHook built"
)]
extern "system" fn play_effect_detour(
    actor: *mut c_void,
    handle: *mut u32,
    params: *const u8,
    attach: *const u8,
) -> *mut u32 {
    let original = ORIGINAL_PLAY_EFFECT.load(Ordering::Acquire);
    if original.is_null() {
        return handle;
    }
    // SAFETY: the trampoline continues into the original function, which has this signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, PlayEffect>(original)(actor, handle, params, attach)
    };
    if SENDING.load(Ordering::Relaxed) {
        let (actor, params, attach) = (actor as usize, params as usize, attach as usize);
        let handle = handle as usize;
        let _ = panic::catch_unwind(|| {
            send_effect(actor as u64, params as u64, attach as u64, handle as u64)
        });
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original play-sound function through the trampoline MinHook built"
)]
extern "system" fn play_sound_detour(
    actor: *mut c_void,
    name: *const c_char,
    emitter: *mut c_void,
) -> *mut c_void {
    let original = ORIGINAL_PLAY_SOUND.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null_mut();
    }
    if SENDING.load(Ordering::Relaxed) {
        let (actor, name) = (actor as usize, name as usize);
        let _ = panic::catch_unwind(|| send_sound(actor as u64, name as u64));
    }
    // SAFETY: the trampoline continues into the original function, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, PlaySound>(original)(actor, name, emitter) }
}

fn send_effect(actor: u64, params: u64, attach: u64, handle_address: u64) {
    let Some(game) = local_player_in_play(actor) else {
        return;
    };
    if !admit_send() {
        return;
    }
    let captured = match read_effect(game.memory(), params, attach) {
        Ok(Some(captured)) => captured,
        Ok(None) => return,
        Err(_) => {
            count(&UNREADABLE);
            return;
        }
    };
    let handle = if handle_address == 0 {
        EFFECT_INVALID_HANDLE
    } else {
        game.memory()
            .read_i32(handle_address)
            .map_or(EFFECT_INVALID_HANDLE, i32::cast_unsigned)
    };
    let modes = captured.attach_modes();
    let wire = captured.to_wire(handle);
    if wire.validate().is_err() {
        count(&UNREADABLE);
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    log_limited(&format!(
        "effects: sent effect {} (bone {:?}, {:?}, attach modes {}/{}, handle {handle:#x})",
        wire.effect, wire.bone, wire.second_bone, modes.0, modes.1,
    ));
    if handle != EFFECT_INVALID_HANDLE {
        SENT_HANDLES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .track(handle);
    }
    net.send_relayed(Message::Effect(wire));
    count(&SENT_EFFECTS);
}

fn send_sound(actor: u64, name: u64) {
    let Some(game) = local_player_in_play(actor) else {
        return;
    };
    if !admit_send() {
        return;
    }
    let sound = match read_sound(game.memory(), name) {
        Ok(Some(sound)) => sound,
        Ok(None) => return,
        Err(_) => {
            count(&UNREADABLE);
            return;
        }
    };
    let wire = SoundPlay { sound };
    if wire.validate().is_err() {
        count(&UNREADABLE);
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    log_limited(&format!("effects: sent sound {}", wire.sound));
    net.send_relayed(Message::Sound(wire));
    count(&SENT_SOUNDS);
}

fn local_player_in_play(actor: u64) -> Option<Replicant<InProcessMemory>> {
    let build = HOOKED_BUILD.get()?;
    let game = Replicant::new(InProcessMemory::new(build));
    if game.playable_actor().ok()? != actor {
        return None;
    }
    let playing =
        read_world_flags(game.memory()).is_ok_and(|flags| !flags.activity().world_paused());
    playing.then_some(game)
}

fn admit_send() -> bool {
    let admitted = SEND_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(net_clock_ms());
    if !admitted {
        count(&REFUSED_BY_RATE_LIMIT);
    }
    admitted
}
