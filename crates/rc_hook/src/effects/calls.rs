use core::ffi::{c_char, c_void};
use std::ffi::CString;

use rc_game::effects::{build_effect_call, match_anchor_offset};
use rc_game::offsets::{
    ACTOR_PLAY_EFFECT_VTABLE_OFFSET, ACTOR_SOUND_EMITTER_OFFSET, ACTOR_TRANSFORM_BLOCK_OFFSET,
    ACTOR_TRANSFORM_ROW_STRIDE, EFFECT_INVALID_HANDLE, SOUND_EMITTER_PLAY_FUNCTION_RVA,
};
use rc_game::{GameMemory, Replicant};
use rc_proto::EffectPlay;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type PlayEffect =
    unsafe extern "system" fn(*mut c_void, *mut u32, *const u8, *const u8) -> *mut u32;
type PlaySound =
    unsafe extern "system" fn(*mut c_void, *const c_char, f32, i32, i32) -> *mut c_void;

pub(super) struct PlayedEffect {
    pub handle: Option<u32>,
    pub block: Vec<u64>,
}

pub(super) fn play_effect(
    build: &VerifiedBuild,
    actor: u64,
    effect: &EffectPlay,
) -> Result<PlayedEffect, String> {
    let memory = InProcessMemory::new(build);
    let vtable = memory.read_u64(actor).map_err(|error| error.to_string())?;
    let function = memory
        .read_u64(vtable + ACTOR_PLAY_EFFECT_VTABLE_OFFSET)
        .map_err(|error| error.to_string())?;
    if function < build.module_base() {
        return Err(format!("the play-effect slot holds {function:#x}"));
    }
    let call = build_effect_call(effect);
    let block = aligned_words(&call.bytes);
    let params = block.as_ptr().cast::<u8>();
    let attach = params.wrapping_add(call.attach_offset);
    let mut handle = EFFECT_INVALID_HANDLE;
    #[expect(unsafe_code, reason = "calls the actor's play-effect virtual function")]
    // SAFETY: the pointer was read from the actor's vtable and lies inside the game image; the
    // slot takes (actor, out handle, effect parameters, attach descriptor) as the game's own
    // callers pass them. The blocks live in `block`, which the caller keeps for the effect's life.
    unsafe {
        core::mem::transmute::<usize, PlayEffect>(function as usize)(
            actor as usize as *mut c_void,
            &mut handle,
            params,
            attach,
        );
    }
    Ok(PlayedEffect {
        handle: (handle != EFFECT_INVALID_HANDLE).then_some(handle),
        block,
    })
}

pub(super) fn align_anchor_offset(build: &VerifiedBuild, puppet: u64) -> Option<[f32; 3]> {
    let player = Replicant::new(InProcessMemory::new(build))
        .playable_actor()
        .ok()?;
    match_anchor_offset(&mut InProcessMemory::new(build), player, puppet).ok()
}

pub(super) fn play_sound(build: &VerifiedBuild, actor: u64, sound: &str, volume: f32) -> bool {
    let Ok(name) = CString::new(sound) else {
        return false;
    };
    let function = (build.module_base() + u64::from(SOUND_EMITTER_PLAY_FUNCTION_RVA)) as usize;
    let emitter = actor + ACTOR_SOUND_EMITTER_OFFSET;
    #[expect(
        unsafe_code,
        reason = "calls the sound emitter's play function at its verified address"
    )]
    // SAFETY: the RVA starts the function (emitter, sound name, volume, 0, 0 -> voice or null)
    // in the verified build; the emitter is the actor's own, the name is NUL terminated and
    // outlives the call, and this runs on the game thread inside the script manager tick.
    let result = unsafe {
        core::mem::transmute::<usize, PlaySound>(function)(
            emitter as usize as *mut c_void,
            name.as_ptr(),
            volume,
            0,
            0,
        )
    };
    !result.is_null()
}

pub(super) fn distance_to_listener(build: &VerifiedBuild, actor: u64) -> Option<f32> {
    let game = Replicant::new(InProcessMemory::new(build));
    let listener = game.position().ok()?;
    let memory = game.memory();
    let base = actor + ACTOR_TRANSFORM_BLOCK_OFFSET;
    let row = ACTOR_TRANSFORM_ROW_STRIDE as u64;
    let lane = 12;
    let (x, y, z) = (
        memory.read_f32(base + lane).ok()?,
        memory.read_f32(base + row + lane).ok()?,
        memory.read_f32(base + 2 * row + lane).ok()?,
    );
    Some(((x - listener.x).powi(2) + (y - listener.y).powi(2) + (z - listener.z).powi(2)).sqrt())
}

fn aligned_words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks(8)
        .map(|chunk| {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            u64::from_le_bytes(word)
        })
        .collect()
}
