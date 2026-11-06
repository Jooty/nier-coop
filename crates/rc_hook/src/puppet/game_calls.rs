use core::ffi::{CStr, c_char, c_void};

use rc_game::offsets::{
    ACTOR_DESTROY_FUNCTION_RVA, ACTOR_MOTION_INDEX_BY_NAME_FUNCTION_RVA,
    ACTOR_PLAY_MOTION_FUNCTION_RVA, ACTOR_SET_GROUP_FUNCTION_RVA,
    ACTOR_SET_TRANSFORM_VTABLE_OFFSET, CREATE_MODEL_ACTOR_FUNCTION_RVA, CREATION_MATRIX_BYTES,
    PUPPET_PLAY_MOTION_FLAG,
};
use rc_game::{GameMemory, MemoryError};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;
use crate::puppet::error::PuppetError;

const SET_TRANSFORM_FLAG: i32 = 0;

type CreateModelActor = unsafe extern "system" fn(*const c_char, *const u8, i32) -> *mut c_void;
type SetGroup = unsafe extern "system" fn(*mut c_void, i32);
type MotionIndexByName = unsafe extern "system" fn(*mut c_void, *const c_char) -> i32;
type PlayMotion = unsafe extern "system" fn(*mut c_void, i32, i32, f32, u8, i32, f32, f32);
type Destroy = unsafe extern "system" fn(*mut c_void, u8);
type SetTransform = unsafe extern "system" fn(*mut c_void, *const u8, i32);

#[derive(Debug, Clone, Copy)]
pub(super) struct GameCalls {
    module_base: u64,
}

impl GameCalls {
    pub(super) fn new(build: &VerifiedBuild) -> Self {
        Self {
            module_base: build.module_base(),
        }
    }

    pub(super) fn module_base(&self) -> u64 {
        self.module_base
    }

    pub(super) fn create_model_actor(
        &self,
        model: &CStr,
        matrix: &[u8; CREATION_MATRIX_BYTES],
        class_id: i32,
    ) -> u64 {
        #[expect(
            unsafe_code,
            reason = "calls CActorManager::CreateModelActorByName at its verified address"
        )]
        // SAFETY: the build gate passed so the RVA starts that function (name, 64-byte matrix,
        // default class id -> actor or null); both pointers outlive the call, and this runs on
        // the game thread inside the script manager tick, where scripts create model actors.
        unsafe {
            core::mem::transmute::<usize, CreateModelActor>(
                self.address(CREATE_MODEL_ACTOR_FUNCTION_RVA),
            )(model.as_ptr(), matrix.as_ptr(), class_id) as usize as u64
        }
    }

    pub(super) fn set_group(&self, actor: u64, group: i32) {
        #[expect(unsafe_code, reason = "calls CActor::SetGroup at its verified address")]
        // SAFETY: the RVA starts the group setter (actor, group); the actor is live, checked by
        // the caller on the game thread in the same tick.
        unsafe {
            core::mem::transmute::<usize, SetGroup>(self.address(ACTOR_SET_GROUP_FUNCTION_RVA))(
                actor as usize as *mut c_void,
                group,
            );
        }
    }

    pub(super) fn motion_index_by_name(&self, actor: u64, name: &CStr) -> i32 {
        #[expect(
            unsafe_code,
            reason = "calls CActor::MotionIndexByName at its verified address"
        )]
        // SAFETY: the RVA starts the lookup (actor, name -> index or -1); the actor is live on
        // the game thread and the name is NUL terminated.
        unsafe {
            core::mem::transmute::<usize, MotionIndexByName>(
                self.address(ACTOR_MOTION_INDEX_BY_NAME_FUNCTION_RVA),
            )(actor as usize as *mut c_void, name.as_ptr())
        }
    }

    pub(super) fn play_motion(
        &self,
        actor: u64,
        layer: u32,
        index: i32,
        blend: f32,
        looping: bool,
        speed: f32,
        start_frame: f32,
    ) {
        #[expect(
            unsafe_code,
            reason = "calls CActor::PlayMotion at its verified address"
        )]
        // SAFETY: the RVA starts PlayMotion (actor, layer, index, blend, loop, flag, speed,
        // start frame); the actor is live on the game thread.
        unsafe {
            core::mem::transmute::<usize, PlayMotion>(self.address(ACTOR_PLAY_MOTION_FUNCTION_RVA))(
                actor as usize as *mut c_void,
                layer.cast_signed(),
                index,
                blend,
                u8::from(looping),
                PUPPET_PLAY_MOTION_FLAG,
                speed,
                start_frame,
            );
        }
    }

    pub(super) fn destroy(&self, actor: u64) {
        #[expect(unsafe_code, reason = "calls CActor::Destroy at its verified address")]
        // SAFETY: the RVA starts Destroy (actor, force); the caller checked the actor is still
        // live, so the manager frees it later instead of now (force = 0).
        unsafe {
            core::mem::transmute::<usize, Destroy>(self.address(ACTOR_DESTROY_FUNCTION_RVA))(
                actor as usize as *mut c_void,
                0,
            );
        }
    }

    pub(super) fn set_transform(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> Result<(), PuppetError> {
        let setter = self.transform_setter(memory, actor)?;
        #[expect(
            unsafe_code,
            reason = "calls the actor's transform setter virtual function"
        )]
        // SAFETY: the pointer was read from the actor's vtable and lies inside the game image;
        // by the puppet notes the slot takes (actor, 64-byte matrix, flag). The actor is live
        // and this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, SetTransform>(setter)(
                actor as usize as *mut c_void,
                matrix.as_ptr(),
                SET_TRANSFORM_FLAG,
            );
        }
        Ok(())
    }

    fn transform_setter(&self, memory: &InProcessMemory, actor: u64) -> Result<usize, PuppetError> {
        let vtable = memory.read_u64(actor).map_err(missing_setter)?;
        let setter = memory
            .read_u64(vtable + ACTOR_SET_TRANSFORM_VTABLE_OFFSET)
            .map_err(missing_setter)?;
        if setter < self.module_base {
            return Err(PuppetError::TransformSetterMissing);
        }
        Ok(setter as usize)
    }

    pub(super) fn address(&self, rva: u32) -> usize {
        (self.module_base + u64::from(rva)) as usize
    }
}

fn missing_setter(_: MemoryError) -> PuppetError {
    PuppetError::TransformSetterMissing
}
