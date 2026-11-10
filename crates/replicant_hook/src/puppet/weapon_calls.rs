use core::ffi::{CStr, c_char, c_void};
use std::ffi::CString;

use replicant_game::offsets::{
    ACTOR_BONE_MATRIX_BY_NAME_VTABLE_OFFSET, ACTOR_DETACH_MODEL_VTABLE_OFFSET,
    CREATION_MATRIX_BYTES, MODEL_PACK_FIND_BY_NAME_FUNCTION_RVA, MODEL_PACK_MANAGER_RVA,
    MODEL_PACK_RELEASE_FUNCTION_RVA, MODEL_PACK_REQUEST_FUNCTION_RVA, MODEL_PACK_WEAPON_KIND,
    MODEL_PACK_WEAPON_SLOT_HINT, WEAPON_PACK_NAME_BYTES, WEAPON_PACK_NAME_FUNCTION_RVA,
};
use replicant_game::{GameMemory, MemoryError};

use crate::memory::InProcessMemory;
use crate::puppet::error::PuppetError;
use crate::puppet::game_calls::GameCalls;

type WeaponPackName = unsafe extern "system" fn(*mut c_char, i32, u8) -> u64;
type FindModelPack = unsafe extern "system" fn(*mut c_void, *const c_char) -> *mut c_void;
type RequestModelPack =
    unsafe extern "system" fn(*mut c_void, *const c_char, i32, i32, u8, u8) -> *mut c_void;
type ReleaseModelPack = unsafe extern "system" fn(*mut c_void, *mut c_void, u8);
type BoneMatrixByName =
    unsafe extern "system" fn(*mut c_void, *mut u8, *const c_char) -> *mut c_void;
type DetachModel = unsafe extern "system" fn(*mut c_void);

impl GameCalls {
    pub(super) fn weapon_pack_name(&self, id: u8, level: u8) -> Option<CString> {
        let mut name = [0_u8; WEAPON_PACK_NAME_BYTES];
        #[expect(
            unsafe_code,
            reason = "calls the weapon pack name builder at its verified address"
        )]
        // SAFETY: the RVA starts the builder (64-byte out buffer, weapon id, level -> found);
        // explicit non-negative arguments keep it off the local player's save fields, and the
        // buffer is the 64 bytes it writes with strcpy_s.
        let found = unsafe {
            core::mem::transmute::<usize, WeaponPackName>(
                self.address(WEAPON_PACK_NAME_FUNCTION_RVA),
            )(name.as_mut_ptr().cast(), i32::from(id), level)
        };
        if found & 0xFF == 0 {
            return None;
        }
        let end = name.iter().position(|&byte| byte == 0)?;
        CString::new(&name[..end]).ok()
    }

    pub(super) fn find_model_pack(&self, name: &CStr) -> u64 {
        #[expect(
            unsafe_code,
            reason = "calls ModelPackManager::FindByName at its verified address"
        )]
        // SAFETY: the RVA starts FindByName (manager, name -> pack or null); the manager is the
        // static at its RVA and the name is NUL terminated. It only reads.
        unsafe {
            core::mem::transmute::<usize, FindModelPack>(
                self.address(MODEL_PACK_FIND_BY_NAME_FUNCTION_RVA),
            )(self.model_pack_manager(), name.as_ptr()) as usize as u64
        }
    }

    pub(super) fn request_model_pack(&self, name: &CStr) -> u64 {
        self.request_model_pack_of_kind(name, MODEL_PACK_WEAPON_KIND, MODEL_PACK_WEAPON_SLOT_HINT)
    }

    pub(super) fn release_model_pack(&self, pack: u64) {
        #[expect(
            unsafe_code,
            reason = "calls ModelPackManager::Release at its verified address"
        )]
        // SAFETY: the RVA starts Release (manager, pack, force); force 0 as the game's phase
        // loader passes. The pack guard detour sits on this address and removes any actor
        // still wearing the pack before the original runs. Game thread.
        unsafe {
            core::mem::transmute::<usize, ReleaseModelPack>(
                self.address(MODEL_PACK_RELEASE_FUNCTION_RVA),
            )(self.model_pack_manager(), pack as usize as *mut c_void, 0);
        }
    }

    pub(super) fn request_model_pack_of_kind(&self, name: &CStr, kind: i32, slot_hint: i32) -> u64 {
        #[expect(
            unsafe_code,
            reason = "calls ModelPackManager::Request at its verified address"
        )]
        // SAFETY: the RVA starts Request (manager, name, kind, slot hint, two flags -> slot);
        // the arguments are the ones the game's own weapon equip passes for a weapon pack, and
        // this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, RequestModelPack>(
                self.address(MODEL_PACK_REQUEST_FUNCTION_RVA),
            )(
                self.model_pack_manager(),
                name.as_ptr(),
                kind,
                slot_hint,
                0,
                0,
            ) as usize as u64
        }
    }

    pub(super) fn bone_matrix(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        bone: &CStr,
    ) -> Result<[u8; CREATION_MATRIX_BYTES], PuppetError> {
        let function =
            self.virtual_function(memory, actor, ACTOR_BONE_MATRIX_BY_NAME_VTABLE_OFFSET)?;
        let mut matrix = [0_u8; CREATION_MATRIX_BYTES];
        #[expect(unsafe_code, reason = "calls the actor's bone-matrix-by-name virtual")]
        // SAFETY: the pointer was read from the actor's vtable inside the game image; the slot
        // takes (actor, 64-byte out matrix, bone name) and falls back to the actor's own matrix
        // for an unknown name. The actor is live on the game thread.
        unsafe {
            core::mem::transmute::<usize, BoneMatrixByName>(function)(
                actor as usize as *mut c_void,
                matrix.as_mut_ptr(),
                bone.as_ptr(),
            );
        }
        Ok(matrix)
    }

    pub(super) fn detach_model(
        &self,
        memory: &InProcessMemory,
        actor: u64,
    ) -> Result<(), PuppetError> {
        let function = self.virtual_function(memory, actor, ACTOR_DETACH_MODEL_VTABLE_OFFSET)?;
        #[expect(unsafe_code, reason = "calls the actor's model-detach virtual")]
        // SAFETY: the pointer was read from the actor's vtable inside the game image; the game
        // calls this slot (actor) on its own weapon actors before it frees their pack.
        unsafe {
            core::mem::transmute::<usize, DetachModel>(function)(actor as usize as *mut c_void);
        }
        Ok(())
    }

    fn model_pack_manager(&self) -> *mut c_void {
        self.address(MODEL_PACK_MANAGER_RVA) as *mut c_void
    }

    fn virtual_function(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        offset: u64,
    ) -> Result<usize, PuppetError> {
        let missing = |_: MemoryError| PuppetError::VirtualFunctionMissing(offset);
        let vtable = memory.read_u64(actor).map_err(missing)?;
        let function = memory.read_u64(vtable + offset).map_err(missing)?;
        if function < self.module_base() {
            return Err(PuppetError::VirtualFunctionMissing(offset));
        }
        Ok(function as usize)
    }
}
