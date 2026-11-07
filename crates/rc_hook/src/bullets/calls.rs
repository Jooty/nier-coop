use rc_game::GameMemory;
use rc_game::bullet_flights::style_is_set;
use rc_game::effects::{effect_instance_address, finish_effect};
use rc_game::offsets::{
    EFFECT_CREATE_BY_ID_FUNCTION_RVA, EFFECT_CREATE_FADE_ARGUMENT_RVA,
    EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, EFFECT_INSTANCE_MATRIX_OFFSET,
    EFFECT_INSTANCE_STYLE_OFFSET, EFFECT_INVALID_HANDLE,
};
use rc_proto::{BULLET_STYLE_FLOATS, BULLET_TRANSFORM_FLOATS};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type CreateEffectById =
    unsafe extern "system" fn(*mut u32, u32, *const f32, u32, u32, u32, u32) -> *mut u32;

#[repr(align(16))]
struct AlignedMatrix([f32; 16]);

pub(super) fn create_effect(
    build: &VerifiedBuild,
    effect_id: i32,
    transform: &[f32; BULLET_TRANSFORM_FLOATS],
) -> Option<u32> {
    let id = u32::try_from(effect_id).ok().filter(|id| *id > 0)?;
    let memory = InProcessMemory::new(build);
    let fade = memory
        .read_i32(build.module_base() + u64::from(EFFECT_CREATE_FADE_ARGUMENT_RVA))
        .ok()?
        .cast_unsigned();
    let matrix = AlignedMatrix(creation_matrix(transform));
    let function = (build.module_base() + u64::from(EFFECT_CREATE_BY_ID_FUNCTION_RVA)) as usize;
    let mut handle = EFFECT_INVALID_HANDLE;
    #[expect(unsafe_code, reason = "calls the game's create-effect-by-id function")]
    // SAFETY: the RVA starts the function (out handle, effect id, transform matrix, 0, fade,
    // fade, 0 -> out handle) in the verified build, as the bullet's own start code calls it; the
    // matrix is 16-byte aligned and outlives the call, and this runs on the game thread.
    unsafe {
        core::mem::transmute::<usize, CreateEffectById>(function)(
            &mut handle,
            id,
            matrix.0.as_ptr(),
            0,
            fade,
            fade,
            0,
        );
    }
    let created = effect_instance_address(&memory, handle)
        .ok()
        .flatten()
        .is_some();
    created.then_some(handle)
}

pub(super) fn place_effect(
    build: &VerifiedBuild,
    handle: u32,
    transform: &[f32; BULLET_TRANSFORM_FLOATS],
) {
    let mut memory = InProcessMemory::new(build);
    let Ok(Some(instance)) = effect_instance_address(&memory, handle) else {
        return;
    };
    let bytes: Vec<u8> = transform
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    if memory
        .write(instance + EFFECT_INSTANCE_MATRIX_OFFSET, &bytes)
        .is_ok()
    {
        let _ = memory.write_u8(instance + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, 1);
    }
}

pub(super) fn style_effect(build: &VerifiedBuild, handle: u32, style: &[f32; BULLET_STYLE_FLOATS]) {
    if !style_is_set(style) {
        return;
    }
    let mut memory = InProcessMemory::new(build);
    let Ok(Some(instance)) = effect_instance_address(&memory, handle) else {
        return;
    };
    let bytes: Vec<u8> = style.iter().flat_map(|value| value.to_le_bytes()).collect();
    if memory
        .write(instance + EFFECT_INSTANCE_STYLE_OFFSET, &bytes)
        .is_ok()
    {
        let _ = memory.write_u8(instance + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, 1);
    }
}

pub(super) fn end_effect(build: &VerifiedBuild, handle: u32) {
    let mut memory = InProcessMemory::new(build);
    let _ = finish_effect(&mut memory, handle);
}

fn creation_matrix(transform: &[f32; BULLET_TRANSFORM_FLOATS]) -> [f32; 16] {
    let translation = [transform[3], transform[7], transform[11]];
    [
        transform[0],
        transform[1],
        transform[2],
        translation[0],
        transform[4],
        transform[5],
        transform[6],
        translation[1],
        transform[8],
        transform[9],
        transform[10],
        translation[2],
        translation[0],
        translation[1],
        translation[2],
        1.0,
    ]
}
