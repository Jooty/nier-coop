use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_game::offsets::{
    ACTOR_PLAY_MOTION_FUNCTION_RVA, ACTOR_SET_TRANSFORM_VTABLE_OFFSET, BODY_MOTION_LAYER,
    CREATION_MATRIX_BYTES, PUPPET_PLAY_MOTION_FLAG,
};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

const SET_TRANSFORM_FLAG: i32 = 0;

type PlayMotion = unsafe extern "system" fn(*mut c_void, i32, i32, f32, u8, i32, f32, f32);
type SetTransform = unsafe extern "system" fn(*mut c_void, *const u8, i32);

#[derive(Debug, Clone, Copy)]
pub(super) struct NpcCalls {
    module_base: u64,
    image_end: u64,
}

impl NpcCalls {
    pub(super) fn new(build: &VerifiedBuild) -> Self {
        let module_base = build.module_base();
        Self {
            module_base,
            image_end: module_base + u64::from(SUPPORTED_PE_IDENTITY.size_of_image),
        }
    }

    pub(super) fn set_transform(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> bool {
        let Some(setter) = memory
            .read_u64(actor)
            .and_then(|vtable| memory.read_u64(vtable + ACTOR_SET_TRANSFORM_VTABLE_OFFSET))
            .ok()
            .filter(|function| (self.module_base..self.image_end).contains(function))
        else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "calls the actor's transform setter virtual function"
        )]
        // SAFETY: the pointer comes from the actor's vtable inside the game image; the slot
        // takes (actor, 64-byte matrix, flag), as for puppets and enemies. The actor passed the
        // liveness check this tick and this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, SetTransform>(setter as usize)(
                actor as usize as *mut c_void,
                matrix.as_ptr(),
                SET_TRANSFORM_FLAG,
            );
        }
        true
    }

    pub(super) fn play_body_motion(
        &self,
        actor: u64,
        index: i32,
        blend: f32,
        looping: bool,
        speed: f32,
        start_frame: f32,
    ) {
        let function = (self.module_base + u64::from(ACTOR_PLAY_MOTION_FUNCTION_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls CActor::PlayMotion at its verified address"
        )]
        // SAFETY: the RVA starts PlayMotion (actor, layer, index, blend, loop, flag, speed, start
        // frame) in the verified build; the actor is live on the game thread and the index came
        // from the host's NPC of the same class, so it is in the same motion pack.
        unsafe {
            core::mem::transmute::<usize, PlayMotion>(function)(
                actor as usize as *mut c_void,
                BODY_MOTION_LAYER.cast_signed(),
                index,
                blend,
                u8::from(looping),
                PUPPET_PLAY_MOTION_FLAG,
                speed,
                start_frame,
            );
        }
    }
}
