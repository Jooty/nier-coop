use core::ffi::c_void;

use replicant_game::GameMemory;
use replicant_game::build::SUPPORTED_PE_IDENTITY;
use replicant_game::enemies::spec_slot_address;
use replicant_game::offsets::{
    ACTOR_DESTROY_FUNCTION_RVA, ACTOR_MANAGER_RVA, ACTOR_MESSAGE_VTABLE_OFFSET,
    ACTOR_ON_RECEIVE_HIT_VTABLE_OFFSET, ACTOR_PARAMETER_OBJECT_OFFSET,
    ACTOR_PLAY_MOTION_FUNCTION_RVA, ACTOR_SELF_HANDLE_OFFSET, ACTOR_SET_TRANSFORM_VTABLE_OFFSET,
    ADD_AGGRO_BY_HANDLE_FUNCTION_RVA, ATTACK_QUERY_MANAGER_RVA, BODY_MOTION_LAYER,
    CREATION_MATRIX_BYTES, DIRECT_HIT_FUNCTION_RVA, GET_ACTOR_IN_GROUP_FUNCTION_RVA,
    HUMAN_ENEMY_ON_RECEIVE_HIT_FUNCTION_RVA, MAP_MANAGER_RVA, PARAMETER_DAMAGE_TAKEN_SCALE_OFFSET,
    PARAMETER_FLAGS_OFFSET, PARAMETER_GET_HP_VTABLE_OFFSET, PARAMETER_GET_MAX_HP_VTABLE_OFFSET,
    PARAMETER_SET_HP_VTABLE_OFFSET, PARAMETER_SURVIVE_AT_ONE_HP_FLAG, PUPPET_PLAY_MOTION_FLAG,
    PUT_ENEMY_ACTOR_FUNCTION_RVA,
};
use replicant_proto::{ENEMY_SPEC_BYTES, EnemyCreation, MAX_ENEMY_ACTOR_NAME_BYTES};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

const SET_TRANSFORM_FLAG: i32 = 0;

type GetActorInGroup = unsafe extern "system" fn(*mut c_void, i32, i32) -> *mut c_void;
type ParameterGet = unsafe extern "system" fn(*mut c_void) -> i32;
type ParameterSet = unsafe extern "system" fn(*mut c_void, i32);
type SetTransform = unsafe extern "system" fn(*mut c_void, *const u8, i32);
type DirectHit = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut c_void,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
) -> u64;
type ActorMessage = unsafe extern "system" fn(*mut c_void, i32, *mut c_void, u64, u64) -> u64;
type AddAggroByHandle = unsafe extern "system" fn(*mut c_void, *const u32, f32, u8) -> u8;
type PutEnemyActor = unsafe extern "system" fn(
    *mut c_void,
    *const u8,
    i32,
    *const u8,
    *const u8,
    *const c_void,
    u8,
    f32,
    u8,
) -> *mut c_void;
type Destroy = unsafe extern "system" fn(*mut c_void, u8);
type PlayMotion = unsafe extern "system" fn(*mut c_void, i32, i32, f32, u8, i32, f32, f32);

#[repr(align(16))]
struct AlignedBytes<const N: usize>([u8; N]);

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

    pub(super) fn put_enemy_actor(
        &self,
        creation: &EnemyCreation,
        group: u16,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> Option<u64> {
        let slot = spec_slot_address(self.module_base, creation.spec_slot)?;
        if creation.spec.len() != ENEMY_SPEC_BYTES {
            return None;
        }
        let mut name = [0_u8; MAX_ENEMY_ACTOR_NAME_BYTES + 1];
        name[..creation.actor_name.len()].copy_from_slice(creation.actor_name.as_bytes());
        let mut spec = AlignedBytes([0_u8; ENEMY_SPEC_BYTES]);
        spec.0.copy_from_slice(&creation.spec);
        let mut matrix_copy = AlignedBytes([0_u8; CREATION_MATRIX_BYTES]);
        matrix_copy.0.copy_from_slice(matrix);
        let function = (self.module_base + u64::from(PUT_ENEMY_ACTOR_FUNCTION_RVA)) as usize;
        let map_manager = (self.module_base + u64::from(MAP_MANAGER_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls MapManager::PutEnemyActor at its verified address"
        )]
        // SAFETY: the RVA starts PutEnemyActor (nine arguments, as SpawnGroupAtPoint calls it);
        // the name is NUL terminated, the spec and matrix are aligned copies that outlive the
        // call, the slot is in the game's own array and this runs on the game thread.
        let actor = unsafe {
            core::mem::transmute::<usize, PutEnemyActor>(function)(
                map_manager as *mut c_void,
                name.as_ptr(),
                i32::from(group),
                matrix_copy.0.as_ptr(),
                spec.0.as_ptr(),
                slot as usize as *const c_void,
                creation.spawn_flag,
                0.0,
                creation.attach_flag,
            )
        };
        let actor = actor as usize as u64;
        (actor != 0).then_some(actor)
    }

    pub(super) fn direct_hit(&self, attacker: u64, victim: u64, damage: [u32; 3], flags: u32) {
        let function = (self.module_base + u64::from(DIRECT_HIT_FUNCTION_RVA)) as usize;
        let manager = (self.module_base + u64::from(ATTACK_QUERY_MANAGER_RVA)) as usize;
        let [hp, second, third] = damage;
        #[expect(
            unsafe_code,
            reason = "calls the game's direct hit (FUN_1403711A0) at its verified address"
        )]
        // SAFETY: the RVA starts the direct hit (manager, attacker, victim, three damages, flags,
        // three record words) in the verified build, as the player's message 0x192 calls it; both
        // actors are live this tick and this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, DirectHit>(function)(
                manager as *mut c_void,
                attacker as usize as *mut c_void,
                victim as usize as *mut c_void,
                hp,
                second,
                third,
                flags,
                0,
                0,
                0,
            );
        }
    }

    pub(super) fn send_actor_message(
        &self,
        memory: &InProcessMemory,
        actor: u64,
        message: i32,
        sender: u64,
        argument: u64,
    ) -> bool {
        let Some(handler) = self.virtual_function(memory, actor, ACTOR_MESSAGE_VTABLE_OFFSET)
        else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "calls the actor's message handler virtual function"
        )]
        // SAFETY: the pointer comes from the actor's vtable inside the game image; the slot takes
        // (actor, message, sender actor, argument, result record or null), as the hit funnel and
        // the player's own code call it. Both actors are live and this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, ActorMessage>(handler)(
                actor as usize as *mut c_void,
                message,
                sender as usize as *mut c_void,
                argument,
                0,
            );
        }
        true
    }

    pub(super) fn has_human_aggro_table(&self, memory: &InProcessMemory, enemy: u64) -> bool {
        self.virtual_function(memory, enemy, ACTOR_ON_RECEIVE_HIT_VTABLE_OFFSET)
            .is_some_and(|function| {
                function as u64
                    == self.module_base + u64::from(HUMAN_ENEMY_ON_RECEIVE_HIT_FUNCTION_RVA)
            })
    }

    pub(super) fn add_aggro(
        &self,
        memory: &InProcessMemory,
        enemy: u64,
        target: u64,
        weight: f32,
    ) -> bool {
        if !self.has_human_aggro_table(memory, enemy) {
            return false;
        }
        let Ok(handle) = memory.read_array::<8>(target + ACTOR_SELF_HANDLE_OFFSET) else {
            return false;
        };
        let handle = [
            u32::from_le_bytes([handle[0], handle[1], handle[2], handle[3]]),
            u32::from_le_bytes([handle[4], handle[5], handle[6], handle[7]]),
        ];
        let function = (self.module_base + u64::from(ADD_AGGRO_BY_HANDLE_FUNCTION_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls CActorBodyBase::AddAggroByHandle at its verified address"
        )]
        // SAFETY: the RVA starts AddAggroByHandle (enemy, handle pointer, weight in xmm2, keep
        // others flag); the enemy has the human hit callback, so it owns the aggro table written
        // here. The function checks the handle's generation itself; this runs on the game thread.
        unsafe {
            core::mem::transmute::<usize, AddAggroByHandle>(function)(
                enemy as usize as *mut c_void,
                handle.as_ptr(),
                weight,
                0,
            ) != 0
        }
    }

    pub(super) fn destroy(&self, actor: u64) {
        let function = (self.module_base + u64::from(ACTOR_DESTROY_FUNCTION_RVA)) as usize;
        #[expect(unsafe_code, reason = "calls CActor::Destroy at its verified address")]
        // SAFETY: the RVA starts Destroy (actor, force); the caller checked the actor is still
        // live, so the manager frees it later instead of now (force = 0).
        unsafe {
            core::mem::transmute::<usize, Destroy>(function)(actor as usize as *mut c_void, 0);
        }
    }

    pub(super) fn set_survive_at_one_hp(
        &self,
        memory: &mut InProcessMemory,
        actor: u64,
        survive: bool,
    ) -> bool {
        let Some(parameters) = memory
            .read_u64(actor + ACTOR_PARAMETER_OBJECT_OFFSET)
            .ok()
            .filter(|pointer| *pointer != 0)
        else {
            return false;
        };
        let address = parameters + PARAMETER_FLAGS_OFFSET;
        let Ok(flags) = memory.read_u8(address) else {
            return false;
        };
        let wanted = if survive {
            flags | PARAMETER_SURVIVE_AT_ONE_HP_FLAG
        } else {
            flags & !PARAMETER_SURVIVE_AT_ONE_HP_FLAG
        };
        wanted == flags || memory.write_u8(address, wanted).is_ok()
    }

    pub(super) fn damage_taken_scale(&self, memory: &InProcessMemory, actor: u64) -> Option<f32> {
        let parameters = memory
            .read_u64(actor + ACTOR_PARAMETER_OBJECT_OFFSET)
            .ok()
            .filter(|pointer| *pointer != 0)?;
        memory
            .read_f32(parameters + PARAMETER_DAMAGE_TAKEN_SCALE_OFFSET)
            .ok()
    }

    pub(super) fn set_damage_taken_scale(
        &self,
        memory: &mut InProcessMemory,
        actor: u64,
        scale: f32,
    ) -> bool {
        let Some(parameters) = memory
            .read_u64(actor + ACTOR_PARAMETER_OBJECT_OFFSET)
            .ok()
            .filter(|pointer| *pointer != 0)
        else {
            return false;
        };
        memory
            .write_f32(parameters + PARAMETER_DAMAGE_TAKEN_SCALE_OFFSET, scale)
            .is_ok()
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
        // from the host's enemy of the same id, so it is in the same motion pack.
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

    fn virtual_function(&self, memory: &InProcessMemory, object: u64, slot: u64) -> Option<usize> {
        let vtable = memory.read_u64(object).ok()?;
        let function = memory.read_u64(vtable + slot).ok()?;
        (self.module_base..self.image_end)
            .contains(&function)
            .then_some(function as usize)
    }
}
