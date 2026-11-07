use core::ffi::{c_char, c_void};
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use rc_game::effects::{
    EffectSource, MAX_EFFECT_EVENTS_PER_SECOND, RateWindow, SentEffects, classify_effect_source,
    effect_is_finished, power_actor_class, read_actor_position, read_effect, read_sound,
};
use rc_game::marker::read_world_flags;
use rc_game::offsets::{
    ACTOR_PLAY_EFFECT_FUNCTION_RVA, ACTOR_PLAY_SOUND_FUNCTION_RVA, EFFECT_INVALID_HANDLE,
};
use rc_game::puppet::actor_is_live;
use rc_game::{GameMemory, Replicant};
use rc_proto::{BulletEffectPlay, EffectStop, Message, SoundPlay};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::effects::report::{
    REFUSED_BY_RATE_LIMIT, SENT_EFFECTS, SENT_SOUNDS, SENT_STOPS, UNREADABLE, count, log_limited,
};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::puppet::is_puppet_actor;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_PLAY_EFFECT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_PLAY_SOUND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SENDING: AtomicBool = AtomicBool::new(false);
static SENT_HANDLES: LazyLock<Mutex<SentEffects>> =
    LazyLock::new(|| Mutex::new(SentEffects::new()));
static PROBED_CLASS_AND_NAME_PAIRS: LazyLock<Mutex<HashSet<(u64, String)>>> =
    LazyLock::new(Mutex::default);
static SEND_WINDOW: LazyLock<Mutex<RateWindow>> =
    LazyLock::new(|| Mutex::new(RateWindow::new(1000, MAX_EFFECT_EVENTS_PER_SECOND)));

const MAX_PROBED_CLASS_AND_NAME_PAIRS: usize = 2048;

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
    let was_sending = SENDING.swap(sending, Ordering::Relaxed);
    if was_sending && !sending {
        let abandoned = SENT_HANDLES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take_all();
        send_stops(&abandoned, "the partner left play");
    }
}

pub(super) fn send_ended_effects(build: &VerifiedBuild) {
    let memory = InProcessMemory::new(build);
    let mut sent = SENT_HANDLES.lock().unwrap_or_else(PoisonError::into_inner);
    let ended = sent.take_ended(|handle| effect_is_finished(&memory, handle).unwrap_or(true));
    let orphaned = match Replicant::new(InProcessMemory::new(build)).playable_actor() {
        Ok(player) => {
            sent.take_anchor_gone(|anchor| anchor_left_the_players_hands(&memory, anchor, player))
        }
        Err(_) => Vec::new(),
    };
    drop(sent);
    send_stops(&ended, "the effect ended");
    send_stops(&orphaned, "its actor vanished or left the player's hands");
}

fn anchor_left_the_players_hands<M: GameMemory>(memory: &M, anchor: u64, player: u64) -> bool {
    !actor_is_live(memory, anchor).unwrap_or(false)
        || !matches!(
            classify_effect_source(memory, anchor, player),
            Ok(EffectSource::PowerOfPlayer | EffectSource::BulletOfPlayer)
        )
}

fn send_stops(handles: &[u32], reason: &str) {
    let Some(net) = net::handle() else {
        return;
    };
    for handle in handles {
        net.send_relayed(Message::EffectStop(EffectStop { handle: *handle }));
        count(&SENT_STOPS);
        log_line(&format!(
            "effects: sent stop for handle {handle:#x} ({reason})"
        ));
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
    let Some((game, source)) = source_in_play(actor) else {
        return;
    };
    if !matches!(
        source,
        EffectSource::Player
            | EffectSource::Book
            | EffectSource::BulletOfPlayer
            | EffectSource::PowerOfPlayer
    ) {
        let name = read_effect(game.memory(), params, attach)
            .ok()
            .flatten()
            .map(|captured| captured.to_wire(EFFECT_INVALID_HANDLE).effect);
        probe_other_actor(&game, "effect", actor, source, name.as_deref());
        return;
    }
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
        "effects: sent {source:?} effect {} (bone {:?}, {:?}, attach modes {}/{}, handle {handle:#x})",
        wire.effect, wire.bone, wire.second_bone, modes.0, modes.1,
    ));
    if handle != EFFECT_INVALID_HANDLE {
        let evicted = {
            let mut sent = SENT_HANDLES.lock().unwrap_or_else(PoisonError::into_inner);
            match source {
                EffectSource::PowerOfPlayer | EffectSource::BulletOfPlayer => {
                    sent.track_lasting(handle, Some(actor))
                }
                EffectSource::Book => sent.track_lasting(handle, None),
                _ => sent.track(handle),
            }
        };
        if let Some(evicted) = evicted {
            send_stops(&[evicted], "the sent list was full");
        }
    }
    let message = match source {
        EffectSource::Book => Message::BookEffect(wire),
        EffectSource::BulletOfPlayer | EffectSource::PowerOfPlayer => {
            let Ok(position) = read_actor_position(game.memory(), actor) else {
                count(&UNREADABLE);
                return;
            };
            let bullet = BulletEffectPlay {
                position,
                effect: wire,
            };
            if bullet.validate().is_err() {
                count(&UNREADABLE);
                return;
            }
            let class = power_actor_class(game.memory(), actor)
                .ok()
                .flatten()
                .map_or("bullet", |class| class.name);
            log_limited(&format!(
                "effects: {class} {actor:#x} of the local player at {position:?}"
            ));
            Message::BulletEffect(bullet)
        }
        _ => Message::Effect(wire),
    };
    net.send_relayed(message);
    count(&SENT_EFFECTS);
}

fn send_sound(actor: u64, name: u64) {
    let Some((game, source)) = source_in_play(actor) else {
        return;
    };
    if !matches!(source, EffectSource::Player | EffectSource::Book) {
        let name = read_sound(game.memory(), name).ok().flatten();
        probe_other_actor(&game, "sound", actor, source, name.as_deref());
        return;
    }
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
    log_limited(&format!("effects: sent {source:?} sound {}", wire.sound));
    net.send_relayed(if source == EffectSource::Book {
        Message::BookSound(wire)
    } else {
        Message::Sound(wire)
    });
    count(&SENT_SOUNDS);
}

fn source_in_play(actor: u64) -> Option<(Replicant<InProcessMemory>, EffectSource)> {
    let build = HOOKED_BUILD.get()?;
    let game = Replicant::new(InProcessMemory::new(build));
    let player = game.playable_actor().ok()?;
    let playing =
        read_world_flags(game.memory()).is_ok_and(|flags| !flags.activity().world_paused());
    if !playing {
        return None;
    }
    let source = classify_effect_source(game.memory(), actor, player).ok()?;
    Some((game, source))
}

fn probe_other_actor(
    game: &Replicant<InProcessMemory>,
    what: &str,
    actor: u64,
    source: EffectSource,
    name: Option<&str>,
) {
    if is_puppet_actor(actor) {
        return;
    }
    let memory = game.memory();
    let Ok(vtable) = memory.read_u64(actor) else {
        return;
    };
    let class_rva = vtable.wrapping_sub(memory.module_base());
    let name = name.unwrap_or("?");
    let mut probed = PROBED_CLASS_AND_NAME_PAIRS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if probed.len() >= MAX_PROBED_CLASS_AND_NAME_PAIRS
        || !probed.insert((class_rva, name.to_owned()))
    {
        return;
    }
    drop(probed);
    log_line(&format!(
        "effects: probe: a {what} {name} played on actor {actor:#x} of class rva {class_rva:#x} ({source:?}, not sent)"
    ));
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
