use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use replicant_game::GameMemory;
use replicant_game::offsets::{
    BULLET_MANAGER_DRAW_FUNCTION_RVA, BULLET_SPRITE_BATCH_ADD_FUNCTION_RVA,
    BULLET_SPRITE_BATCH_ENTRIES_THIS_FRAME_OFFSET, BULLET_SPRITE_BATCH_FREE_ENTRY_POINTER_OFFSET,
    BULLET_SPRITE_BATCH_RVA,
};
use replicant_proto::BulletSprite;

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

const SPRITE_POSITION_W: f32 = 1.0;

static ORIGINAL_DRAW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static SPRITES_TO_DRAW: Mutex<Vec<SpriteToDraw>> = Mutex::new(Vec::new());
static DRAWS: AtomicU32 = AtomicU32::new(0);
static DRAWS_WITHOUT_A_BATCH: AtomicU32 = AtomicU32::new(0);
static SPRITES_HANDED_TO_THE_BATCH: AtomicU32 = AtomicU32::new(0);
static SPRITES_THE_BATCH_TOOK: AtomicU32 = AtomicU32::new(0);

type DrawBullets = unsafe extern "system" fn(*mut c_void);
type AddSprite =
    unsafe extern "system" fn(*mut c_void, i32, i32, *const f32, f32, f32, f32, *const c_void);

#[repr(align(16))]
struct AlignedPosition([f32; 4]);

#[derive(Debug, Clone, Copy)]
pub(super) struct SpriteToDraw {
    pub position: [f32; 3],
    pub sprite: BulletSprite,
    pub frame: u16,
}

/// What the bullet draw hook did since the last call, for the guest's log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SpriteDrawCounts {
    pub draws: u32,
    pub draws_without_a_batch: u32,
    pub handed: u32,
    pub taken: u32,
}

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let address = (build.module_base() + u64::from(BULLET_MANAGER_DRAW_FUNCTION_RVA)) as usize;
    #[expect(unsafe_code, reason = "hooks the bullet manager's draw by its address")]
    // SAFETY: the RVA starts the bullet manager's draw (manager, void) in the verified build,
    // and the replacement has the same signature.
    unsafe {
        hook_address(
            "BulletMan::Draw",
            address,
            draw_detour as *const () as *mut c_void,
            &ORIGINAL_DRAW,
        );
    }
}

pub(super) fn set_sprites_to_draw(sprites: Vec<SpriteToDraw>) {
    *SPRITES_TO_DRAW
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = sprites;
}

pub(super) fn take_sprite_draw_counts() -> SpriteDrawCounts {
    SpriteDrawCounts {
        draws: DRAWS.swap(0, Ordering::Relaxed),
        draws_without_a_batch: DRAWS_WITHOUT_A_BATCH.swap(0, Ordering::Relaxed),
        handed: SPRITES_HANDED_TO_THE_BATCH.swap(0, Ordering::Relaxed),
        taken: SPRITES_THE_BATCH_TOOK.swap(0, Ordering::Relaxed),
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original bullet draw through the trampoline MinHook built"
)]
extern "system" fn draw_detour(manager: *mut c_void) {
    // The game's own bullets add their sprites inside the original; the batch is flushed and
    // emptied at its end, so the host's sprites go in first.
    let _ = panic::catch_unwind(add_host_sprites);
    let original = ORIGINAL_DRAW.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original draw, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, DrawBullets>(original)(manager);
    }
}

fn add_host_sprites() {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let sprites = SPRITES_TO_DRAW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    if sprites.is_empty() {
        return;
    }
    DRAWS.fetch_add(1, Ordering::Relaxed);
    let memory = InProcessMemory::new(build);
    let batch = build.module_base() + u64::from(BULLET_SPRITE_BATCH_RVA);
    let entries = |memory: &InProcessMemory| {
        memory
            .read_i32(batch + BULLET_SPRITE_BATCH_ENTRIES_THIS_FRAME_OFFSET)
            .unwrap_or(0)
    };
    // The batch has entries to hand out only between a phase's set-up and its tear-down.
    let set_up = memory
        .read_u64(batch + BULLET_SPRITE_BATCH_FREE_ENTRY_POINTER_OFFSET)
        .is_ok_and(|free_entry| free_entry != 0);
    if !set_up {
        DRAWS_WITHOUT_A_BATCH.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let before = entries(&memory);
    let function = (build.module_base() + u64::from(BULLET_SPRITE_BATCH_ADD_FUNCTION_RVA)) as usize;
    for drawn in &sprites {
        let position = AlignedPosition([
            drawn.position[0],
            drawn.position[1],
            drawn.position[2],
            SPRITE_POSITION_W,
        ]);
        #[expect(unsafe_code, reason = "calls the game's add-to-sprite-batch function")]
        // SAFETY: the RVA starts the function (batch, sprite kind, frame, position of four
        // floats, alpha, size, angle, light or null -> void) in the verified build, as the
        // sprite object's own draw calls it from inside this hooked function; the position is
        // 16-byte aligned and outlives the call, and a null light is the game's own no-light case.
        unsafe {
            core::mem::transmute::<usize, AddSprite>(function)(
                batch as usize as *mut c_void,
                i32::from(drawn.sprite.kind),
                i32::from(drawn.frame),
                position.0.as_ptr(),
                drawn.sprite.alpha,
                drawn.sprite.size,
                drawn.sprite.angle,
                ptr::null(),
            );
        }
    }
    let taken = entries(&memory).saturating_sub(before).max(0);
    SPRITES_HANDED_TO_THE_BATCH.fetch_add(sprites.len() as u32, Ordering::Relaxed);
    SPRITES_THE_BATCH_TOOK.fetch_add(taken.cast_unsigned(), Ordering::Relaxed);
}
