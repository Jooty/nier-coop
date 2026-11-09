use core::cell::Cell;
use core::ffi::{c_char, c_void};
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use rc_game::effects::{
    EffectSource, HitSparkNames, LookAttempt, MAX_EFFECT_EVENTS_PER_SECOND, OriginNames,
    RateWindow, SentEffects, SoundRoute, WorldEffectTrail, classify_effect_source,
    effect_is_finished, hit_spark_log_line, look_attempt, power_actor_class, read_actor_position,
    read_effect, read_effect_look, read_effect_transform, read_sound, read_world_effect,
    spawn_is_sent,
};
use rc_game::marker::read_world_flags;
use rc_game::offsets::{
    ACTOR_PLAY_EFFECT_FUNCTION_RVA, ACTOR_PLAY_SOUND_FUNCTION_RVA,
    EFFECT_CREATE_BY_ID_FUNCTION_RVA, EFFECT_INVALID_HANDLE, EFFECT_SPAWN_BY_NAME_FUNCTION_RVA,
};
use rc_game::puppet::actor_is_live;
use rc_game::{GameMemory, Replicant};
use rc_proto::{
    BulletEffectPlay, EffectStop, Message, SoundPlay, WORLD_EFFECT_ARGUMENT_COUNT, WorldEffectMove,
    WorldEffectPlay,
};

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
static ORIGINAL_SPAWN_BY_NAME: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CREATE_BY_ID: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WORLD_TRAIL: LazyLock<Mutex<WorldEffectTrail>> = LazyLock::new(Mutex::default);
static SENDING: AtomicBool = AtomicBool::new(false);
static SENT_HANDLES: LazyLock<Mutex<SentEffects>> =
    LazyLock::new(|| Mutex::new(SentEffects::new()));
static PROBED_CLASS_AND_NAME_PAIRS: LazyLock<Mutex<HashSet<(u64, String)>>> =
    LazyLock::new(Mutex::default);
static HIT_SPARK_NAMES: LazyLock<Mutex<HitSparkNames>> = LazyLock::new(Mutex::default);
static PENDING_WORLD_EFFECTS: LazyLock<Mutex<Vec<PendingWorldEffect>>> =
    LazyLock::new(Mutex::default);
static ORIGIN_NAMES: LazyLock<Mutex<OriginNames>> = LazyLock::new(Mutex::default);
static LOOKLESS_NAMES: LazyLock<Mutex<HitSparkNames>> = LazyLock::new(Mutex::default);
static SEND_WINDOW: LazyLock<Mutex<RateWindow>> =
    LazyLock::new(|| Mutex::new(RateWindow::new(1000, MAX_EFFECT_EVENTS_PER_SECOND)));

const MAX_PROBED_CLASS_AND_NAME_PAIRS: usize = 2048;
const MAX_PENDING_WORLD_EFFECTS: usize = 64;
const SPAWN_BY_NAME_CALL_NAME: &str = "SpawnByName";
const PLAY_EFFECT_CALL_NAME: &str = "PlayEffect";
const CREATE_BY_ID_CALL_NAME: &str = "CreateById";

struct PendingWorldEffect {
    wire: WorldEffectPlay,
    frames_waited: u32,
}

thread_local! {
    static CREATE_BY_ID_CALLER_RVA: Cell<u32> = const { Cell::new(0) };
    static NESTED_EFFECT_CALLS: Cell<u32> = const { Cell::new(0) };
    static OWN_HIT_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
    static HIT_SPARK_VIA: Cell<&'static str> = const { Cell::new(SPAWN_BY_NAME_CALL_NAME) };
}

type PlayEffect =
    unsafe extern "system" fn(*mut c_void, *mut u32, *const u8, *const u8) -> *mut u32;
type PlaySound = unsafe extern "system" fn(*mut c_void, *const c_char, *mut c_void) -> *mut c_void;
type SpawnByName =
    unsafe extern "system" fn(*mut u32, *const u8, *const f32, f32, u32, u32, u32, i32) -> *mut u32;
type CreateById =
    unsafe extern "system" fn(*mut u32, u32, *const f32, u32, u32, u32, u32) -> *mut u32;

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
    #[expect(
        unsafe_code,
        reason = "hooks the spawn-effect-by-name function by address"
    )]
    // SAFETY: the RVA starts the function (out handle, name, 3x4 matrix, scale, four stack
    // words -> out handle) in the verified build, and the replacement matches it.
    unsafe {
        hook_address(
            "Effect::SpawnByName",
            (base + u64::from(EFFECT_SPAWN_BY_NAME_FUNCTION_RVA)) as usize,
            spawn_by_name_detour as *const () as *mut c_void,
            &ORIGINAL_SPAWN_BY_NAME,
        )
    };
    #[expect(
        unsafe_code,
        reason = "hooks the create-effect-by-id function by address"
    )]
    // SAFETY: the RVA starts the function (out handle, effect id, matrix, four words -> out
    // handle) in the verified build, and the replacement matches it.
    unsafe {
        hook_address(
            "Effect::CreateById",
            (base + u64::from(EFFECT_CREATE_BY_ID_FUNCTION_RVA)) as usize,
            create_by_id_detour as *const () as *mut c_void,
            &ORIGINAL_CREATE_BY_ID,
        )
    };
}

/// Runs `call` as a nested effect call: an effect spawned by name inside it belongs to an
/// effect the outer call already sent (or to one we are playing for the partner), so it is not sent.
pub(super) fn nested_effect_call<R>(call: impl FnOnce() -> R) -> R {
    NESTED_EFFECT_CALLS.with(|depth| depth.set(depth.get() + 1));
    let result = call();
    NESTED_EFFECT_CALLS.with(|depth| depth.set(depth.get().saturating_sub(1)));
    result
}

pub(crate) fn begin_own_hit() {
    OWN_HIT_IN_FLIGHT.with(|flag| flag.set(true));
}

pub(crate) fn end_own_hit() {
    OWN_HIT_IN_FLIGHT.with(|flag| flag.set(false));
}

fn own_hit_in_flight() -> bool {
    OWN_HIT_IN_FLIGHT.with(Cell::get)
}

fn with_hit_spark_call<R>(via: &'static str, own_effect: bool, call: impl FnOnce() -> R) -> R {
    let was_in_flight = OWN_HIT_IN_FLIGHT.with(|flag| flag.replace(flag.get() && !own_effect));
    let was_via = HIT_SPARK_VIA.with(|cell| cell.replace(via));
    let result = call();
    HIT_SPARK_VIA.with(|cell| cell.set(was_via));
    OWN_HIT_IN_FLIGHT.with(|flag| flag.set(was_in_flight));
    result
}

fn effect_is_sent_as_actor_effect(source: EffectSource) -> bool {
    matches!(
        source,
        EffectSource::Player
            | EffectSource::Book
            | EffectSource::BulletOfPlayer
            | EffectSource::PowerOfPlayer
    )
}

fn actor_effect_is_sent_directly(actor: u64) -> bool {
    source_in_play(actor).is_some_and(|(_, source)| effect_is_sent_as_actor_effect(source))
}

pub(super) fn set_sending(sending: bool) {
    let was_sending = SENDING.swap(sending, Ordering::Relaxed);
    if was_sending && !sending {
        let abandoned = SENT_HANDLES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take_all();
        send_stops(&abandoned, "the partner left play");
        PENDING_WORLD_EFFECTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        WORLD_TRAIL
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}

pub(super) fn send_ended_effects(build: &VerifiedBuild) {
    send_pending_world_effects(build);
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
    send_world_moves(&memory);
    send_stops(&ended, "the effect ended");
    send_stops(&orphaned, "its actor vanished or left the player's hands");
}

fn send_world_moves(memory: &InProcessMemory) {
    let moves = WORLD_TRAIL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .due_moves(net_clock_ms(), |handle| {
            read_effect_transform(memory, handle).ok().flatten()
        });
    let Some(net) = net::handle() else {
        return;
    };
    for (handle, transform) in moves {
        let moved = WorldEffectMove { handle, transform };
        if moved.validate().is_ok() {
            net.send_relayed(Message::WorldEffectMove(moved));
        }
    }
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
    let own_effect = own_hit_in_flight() && actor_effect_is_sent_directly(actor as u64);
    // SAFETY: the trampoline continues into the original function, which has this signature.
    let result = with_hit_spark_call(PLAY_EFFECT_CALL_NAME, own_effect, || {
        nested_effect_call(|| unsafe {
            core::mem::transmute::<*mut c_void, PlayEffect>(original)(actor, handle, params, attach)
        })
    });
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

#[expect(
    unsafe_code,
    reason = "calls the original spawn-effect-by-name function through the trampoline MinHook built"
)]
extern "system" fn spawn_by_name_detour(
    handle: *mut u32,
    name: *const u8,
    matrix: *const f32,
    scale: f32,
    fifth: u32,
    sixth: u32,
    seventh: u32,
    eighth: i32,
) -> *mut u32 {
    let original = ORIGINAL_SPAWN_BY_NAME.load(Ordering::Acquire);
    if original.is_null() {
        return handle;
    }
    // SAFETY: the trampoline continues into the original function, which has this signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, SpawnByName>(original)(
            handle, name, matrix, scale, fifth, sixth, seventh, eighth,
        )
    };
    let outermost = NESTED_EFFECT_CALLS.with(|depth| depth.get() == 0);
    if spawn_is_sent(outermost, own_hit_in_flight()) && SENDING.load(Ordering::Relaxed) {
        let (handle, name, matrix) = (handle as usize, name as usize, matrix as usize);
        let arguments = [
            scale.to_bits(),
            fifth,
            sixth,
            seventh,
            eighth.cast_unsigned(),
        ];
        let _ = panic::catch_unwind(|| {
            send_world_effect(handle as u64, name as u64, matrix as u64, arguments)
        });
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original create-effect-by-id function through the trampoline MinHook built"
)]
extern "system" fn create_by_id_detour(
    handle: *mut u32,
    id: u32,
    matrix: *const f32,
    fourth: u32,
    fifth: u32,
    sixth: u32,
    seventh: u32,
) -> *mut u32 {
    let original = ORIGINAL_CREATE_BY_ID.load(Ordering::Acquire);
    if original.is_null() {
        return handle;
    }
    let caller_rva = create_by_id_caller_rva();
    let was_caller = CREATE_BY_ID_CALLER_RVA.with(|cell| cell.replace(caller_rva));
    // Bullet effects by id are sent as bullet flights, so the by-name spawn inside is nested.
    // SAFETY: the trampoline continues into the original function, which has this signature.
    let result = with_hit_spark_call(CREATE_BY_ID_CALL_NAME, false, || {
        nested_effect_call(|| unsafe {
            core::mem::transmute::<*mut c_void, CreateById>(original)(
                handle, id, matrix, fourth, fifth, sixth, seventh,
            )
        })
    });
    CREATE_BY_ID_CALLER_RVA.with(|cell| cell.set(was_caller));
    result
}

#[expect(
    unsafe_code,
    reason = "walks one stack frame to find who called the create-by-id detour"
)]
fn create_by_id_caller_rva() -> u32 {
    let Some(build) = HOOKED_BUILD.get() else {
        return 0;
    };
    let mut frames = [ptr::null_mut::<c_void>(); 1];
    // SAFETY: the buffer holds the one frame asked for; skipping one frame passes the detour
    // itself, so the entry is the return address inside the game function that called it.
    let captured = unsafe { RtlCaptureStackBackTrace(1, 1, frames.as_mut_ptr(), ptr::null_mut()) };
    if captured == 0 {
        return 0;
    }
    let address = frames[0] as usize as u64;
    u32::try_from(address.wrapping_sub(build.module_base())).unwrap_or(0)
}

fn send_world_effect(
    handle_address: u64,
    name: u64,
    matrix: u64,
    arguments: [u32; WORLD_EFFECT_ARGUMENT_COUNT],
) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let in_hit = own_hit_in_flight();
    let Ok(Some((effect, transform))) = read_world_effect(&memory, name, matrix, in_hit) else {
        return;
    };
    if in_hit {
        log_hit_spark_once(&effect);
    }
    let game = Replicant::new(memory);
    let playing = game.playable_actor().is_ok()
        && read_world_flags(game.memory()).is_ok_and(|flags| !flags.activity().world_paused());
    if !playing {
        return;
    }
    let handle = game
        .memory()
        .read_i32(handle_address)
        .map_or(EFFECT_INVALID_HANDLE, i32::cast_unsigned);
    let wire = WorldEffectPlay {
        effect,
        transform,
        arguments,
        handle,
        look: None,
    };
    if wire.validate().is_err() {
        count(&UNREADABLE);
        return;
    }
    let caller_rva = CREATE_BY_ID_CALLER_RVA.with(Cell::get);
    if caller_rva != 0 {
        let first = ORIGIN_NAMES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_first(&wire.effect, caller_rva);
        if first {
            log_line(&format!(
                "effects: effect origin {} via {CREATE_BY_ID_CALL_NAME} from {caller_rva:#x}",
                wire.effect
            ));
        }
    }
    let mut pending = PENDING_WORLD_EFFECTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if pending.len() < MAX_PENDING_WORLD_EFFECTS {
        pending.push(PendingWorldEffect {
            wire,
            frames_waited: 0,
        });
    }
}

fn send_pending_world_effects(build: &VerifiedBuild) {
    let due = core::mem::take(
        &mut *PENDING_WORLD_EFFECTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    if due.is_empty() {
        return;
    }
    let memory = InProcessMemory::new(build);
    let mut waiting = Vec::new();
    for mut pending in due {
        let look = read_effect_look(&memory, pending.wire.handle)
            .ok()
            .flatten();
        let attempt = if pending.wire.handle == EFFECT_INVALID_HANDLE {
            LookAttempt::GiveUp
        } else {
            look_attempt(pending.frames_waited, look.is_some())
        };
        match attempt {
            LookAttempt::Wait => {
                pending.frames_waited += 1;
                waiting.push(pending);
            }
            LookAttempt::Use => {
                pending.wire.look = look;
                send_world_effect_now(pending.wire);
            }
            LookAttempt::GiveUp => {
                let first = LOOKLESS_NAMES
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_first(&pending.wire.effect);
                if first {
                    log_line(&format!(
                        "effects: effect {} sent without a look, its instance was not found",
                        pending.wire.effect
                    ));
                }
                send_world_effect_now(pending.wire);
            }
        }
    }
    PENDING_WORLD_EFFECTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(waiting);
}

fn send_world_effect_now(wire: WorldEffectPlay) {
    if !SENDING.load(Ordering::Relaxed) || !admit_send() {
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let (handle, transform) = (wire.handle, wire.transform);
    log_limited(&format!(
        "effects: sent world effect {} at [{:.1}, {:.1}, {:.1}] (handle {handle:#x})",
        wire.effect, transform[3], transform[7], transform[11]
    ));
    if handle != EFFECT_INVALID_HANDLE {
        let evicted = SENT_HANDLES
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .track_lasting(handle, None);
        if let Some(evicted) = evicted {
            send_stops(&[evicted], "the sent list was full");
        }
        WORLD_TRAIL
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .track(handle, transform, net_clock_ms());
    }
    net.send_relayed(Message::WorldEffect(wire));
    count(&SENT_EFFECTS);
}

fn log_hit_spark_once(name: &str) {
    let first = HIT_SPARK_NAMES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_first(name);
    if first {
        let via = HIT_SPARK_VIA.with(Cell::get);
        log_line(&hit_spark_log_line(name, via));
    }
}

fn send_effect(actor: u64, params: u64, attach: u64, handle_address: u64) {
    let Some((game, source)) = source_in_play(actor) else {
        return;
    };
    if !effect_is_sent_as_actor_effect(source) {
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
    let Some(route) = source.sound_route() else {
        let name = read_sound(game.memory(), name).ok().flatten();
        probe_other_actor(&game, "sound", actor, source, name.as_deref());
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
    log_limited(&format!("effects: sent {source:?} sound {}", wire.sound));
    net.send_relayed(match route {
        SoundRoute::Puppet => Message::Sound(wire),
        SoundRoute::Book => Message::BookSound(wire),
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
