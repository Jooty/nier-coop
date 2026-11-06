use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_game::offsets::{
    ACTOR_MANAGER_RVA, ACTOR_PARAMETER_OBJECT_OFFSET, ACTOR_SET_TRANSFORM_VTABLE_OFFSET,
    CREATION_MATRIX_BYTES, GET_ACTOR_IN_GROUP_FUNCTION_RVA, PARAMETER_GET_HP_VTABLE_OFFSET,
    PARAMETER_GET_MAX_HP_VTABLE_OFFSET, PARAMETER_SET_HP_VTABLE_OFFSET,
};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

const SET_TRANSFORM_FLAG: i32 = 0;

type GetActorInGroup = unsafe extern "system" fn(*mut c_void, i32, i32) -> *mut c_void;
type ParameterGet = unsafe extern "system" fn(*mut c_void) -> i32;
type ParameterSet = unsafe extern "system" fn(*mut c_void, i32);
type SetTransform = unsafe extern "system" fn(*mut c_void, *const u8, i32);

#[derive(Debug, Clone, Copy)]
pub(super) struct EnemyCalls {
    module_base: u64,
    image_end: u64,
}

impl EnemyCalls {
    pub(super) fn new(build: &VerifiedBuild) -> Self {
        let module_base = build.module_base();
        Self {
            module_base,
            image_end: module_base + u64::from(SUPPORTED_PE_IDENTITY.size_of_image),
        }
    }

    pub(super) fn actor_in_group(&self, group: u16, index: i32) -> u64 {
        let function = (self.module_base + u64::from(GET_ACTOR_IN_GROUP_FUNCTION_RVA)) as usize;
        let manager = (self.module_base + u64::from(ACTOR_MANAGER_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls CActorManager::GetActorInGroup at its verified address"
        )]
        // SAFETY: the build gate passed, so the RVA starts GetActorInGroup (manager, group,
        // index -> actor or null), which bounds-checks both numbers itself. This runs on the
        // game thread inside the script manager tick, where scripts call it too.
        unsafe {
            core::mem::transmute::<usize, GetActorInGroup>(function)(
                manager as *mut c_void,
                i32::from(group),
                index,
            ) as usize as u64
        }
    }

    pub(super) fn hp_of(&self, memory: &InProcessMemory, actor: u64) -> Option<(i32, i32)> {
        let parameters = memory
            .read_u64(actor + ACTOR_PARAMETER_OBJECT_OFFSET)
            .ok()?;
        if parameters == 0 {
            return None;
        }
        let get_hp = self.virtual_function(memory, parameters, PARAMETER_GET_HP_VTABLE_OFFSET)?;
        let get_max_hp =
            self.virtual_function(memory, parameters, PARAMETER_GET_MAX_HP_VTABLE_OFFSET)?;
        #[expect(
            unsafe_code,
            reason = "calls the parameter object's HP getters through its vtable"
        )]
        // SAFETY: both pointers come from the parameter object's vtable and lie inside the game
        // image; the slots take only `this` and return an int (the shape CPlayerParam has). The
        // owning actor passed the liveness check this tick and this runs on the game thread.
        unsafe {
            let this = parameters as usize as *mut c_void;
            Some((
                core::mem::transmute::<usize, ParameterGet>(get_hp)(this),
                core::mem::transmute::<usize, ParameterGet>(get_max_hp)(this),
            ))
        }
    }

    pub(super) fn set_hp(&self, memory: &InProcessMemory, actor: u64, hp: i32) -> bool {
        let Some(parameters) = memory
            .read_u64(actor + ACTOR_PARAMETER_OBJECT_OFFSET)
            .ok()
            .filter(|pointer| *pointer != 0)
        else {
            return false;
        };
        let Some(set_hp) =
            self.virtual_function(memory, parameters, PARAMETER_SET_HP_VTABLE_OFFSET)
        else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "calls the parameter object's HP setter through its vtable"
        )]
        // SAFETY: the pointer comes from the parameter object's vtable inside the game image;
        // the slot takes (this, hp), the call `_SetActorLife` makes. The actor is live this tick
        // and this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, ParameterSet>(set_hp)(
                parameters as usize as *mut c_void,
                hp,
            );
        }
        true
    }

    pub(super) fn set_transform(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> bool {
        let Some(setter) = self.virtual_function(memory, actor, ACTOR_SET_TRANSFORM_VTABLE_OFFSET)
        else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "calls the actor's transform setter virtual function"
        )]
        // SAFETY: the pointer comes from the actor's vtable inside the game image; the slot
        // takes (actor, 64-byte matrix, flag), as for puppets. The actor is live this tick and
        // this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, SetTransform>(setter)(
                actor as usize as *mut c_void,
                matrix.as_ptr(),
                SET_TRANSFORM_FLAG,
            );
        }
        true
    }

    fn virtual_function(&self, memory: &InProcessMemory, object: u64, slot: u64) -> Option<usize> {
        let vtable = memory.read_u64(object).ok()?;
        let function = memory.read_u64(vtable + slot).ok()?;
        (self.module_base..self.image_end)
            .contains(&function)
            .then_some(function as usize)
    }
}
