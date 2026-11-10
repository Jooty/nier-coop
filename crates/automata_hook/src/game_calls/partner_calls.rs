use automata_game::address::{
    CURRENT_BUDDY_FUNCTION, CURRENT_HERO_FUNCTION, DESTROY_BUDDY_FUNCTION,
    ENTITY_AS_PLAYER_BODY_FUNCTION, ENTITY_FROM_HANDLE_FUNCTION, ENTITY_MANAGER_OBJECT,
    ENTITY_TERMINATE_FUNCTION, PLAYER_BODY_VTABLE, SET_BUDDY_HANDLE_FUNCTION,
    SPAWN_BY_NAME_FUNCTION, TELEPORT_RESET_HAP_FUNCTION,
};
use automata_game::partner::{RemoteStateIds, STATE_START_VTABLE_OFFSET};

use crate::game_calls::GameCalls;

const TELEPORT_WITHOUT_CAMERA_RESET: i32 = 0;
const TELEPORT_WITHOUT_INVULNERABLE_TIME: f32 = 0.0;
const SPAWN_WITHOUT_POSITIONAL_DATA: usize = 0;

type BodyGetter = unsafe extern "system" fn() -> usize;
type EntityFromHandle = unsafe extern "system" fn(*const u32) -> usize;
type EntityAsPlayerBody = unsafe extern "system" fn(usize) -> usize;
type SpawnByName = unsafe extern "system" fn(usize, *const u8, u32, usize) -> usize;
type SetBuddyHandle = unsafe extern "system" fn(usize, *const u32);
type OnePointerCall = unsafe extern "system" fn(usize);
type TeleportResetHap = unsafe extern "system" fn(usize, *const Vec4, *const Vec4, i32, f32);
type StartState = unsafe extern "system" fn(usize, i32, u32, u32, u32);

/// A vector the game may read with aligned loads.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct Vec4(pub [f32; 4]);

impl GameCalls {
    #[expect(unsafe_code, reason = "calls the game's current hero getter")]
    pub fn current_hero(&self) -> usize {
        // SAFETY: the address is Hero::getCurrent of the verified build; it takes no arguments and
        // returns 0 when the possessed handle is empty or dead.
        unsafe {
            core::mem::transmute::<usize, BodyGetter>(self.address_of(CURRENT_HERO_FUNCTION))()
        }
    }

    #[expect(unsafe_code, reason = "calls the game's getter of the hero's partner")]
    pub fn current_buddy(&self) -> usize {
        // SAFETY: the address is Hero::getCurrentBuddy of the verified build; it takes no
        // arguments and returns 0 when there is no hero or no live partner.
        unsafe {
            core::mem::transmute::<usize, BodyGetter>(self.address_of(CURRENT_BUDDY_FUNCTION))()
        }
    }

    #[expect(unsafe_code, reason = "calls the game's handle lookup and body cast")]
    pub fn player_body_of_handle(&self, handle: u32) -> usize {
        // SAFETY: the addresses are Entity::fromHandle and Entity::asPl0000 of the verified build;
        // the lookup reads the handle through a pointer to this stack frame and returns 0 for a
        // dead handle, and the cast returns 0 for 0 and for another class.
        unsafe {
            let entity = core::mem::transmute::<usize, EntityFromHandle>(
                self.address_of(ENTITY_FROM_HANDLE_FUNCTION),
            )(&handle);
            self.as_player_body(entity)
        }
    }

    /// # Safety
    /// `entity` must be 0 or a live entity of the game.
    #[expect(unsafe_code, reason = "calls the game's entity to player body cast")]
    pub unsafe fn as_player_body(&self, entity: usize) -> usize {
        // SAFETY: the address is Entity::asPl0000 of the verified build; the caller passes 0 or a
        // live entity.
        unsafe {
            core::mem::transmute::<usize, EntityAsPlayerBody>(
                self.address_of(ENTITY_AS_PLAYER_BODY_FUNCTION),
            )(entity)
        }
    }

    #[expect(unsafe_code, reason = "calls the game's spawn by name")]
    pub fn spawn_by_name(&self, name_with_nul: &[u8], object_id: u32) -> usize {
        // SAFETY: the address is spawnByName of the verified build and the manager its global; the
        // name is a NUL-terminated buffer the game copies, and the game passes a null positional
        // block for its own hero. That the frame-end hook may call it is a guess until run.
        unsafe {
            core::mem::transmute::<usize, SpawnByName>(self.address_of(SPAWN_BY_NAME_FUNCTION))(
                self.address_of(ENTITY_MANAGER_OBJECT),
                name_with_nul.as_ptr(),
                object_id,
                SPAWN_WITHOUT_POSITIONAL_DATA,
            )
        }
    }

    /// # Safety
    /// `hero` must be a live player body and `handle_address` the handle field of a live entity.
    #[expect(unsafe_code, reason = "calls the game's partner link function")]
    pub unsafe fn set_buddy_handle(&self, hero: usize, handle_address: usize) {
        // SAFETY: the address is setBuddyHandle of the verified build; the caller upholds the
        // pointer contract above.
        unsafe {
            core::mem::transmute::<usize, SetBuddyHandle>(
                self.address_of(SET_BUDDY_HANDLE_FUNCTION),
            )(hero, handle_address as *const u32);
        }
    }

    /// # Safety
    /// `hero` must be a live player body, and the partner's player flag must be 0: the game walks
    /// from the hero to its partner until it finds a body without that flag.
    #[expect(unsafe_code, reason = "calls the game's partner removal")]
    pub unsafe fn destroy_buddy(&self, hero: usize) {
        // SAFETY: the address is destroyBuddy of the verified build; the caller upholds the
        // contract above.
        unsafe {
            core::mem::transmute::<usize, OnePointerCall>(self.address_of(DESTROY_BUDDY_FUNCTION))(
                hero,
            );
        }
    }

    /// # Safety
    /// `entity` must be a live entity that this mod spawned.
    #[expect(unsafe_code, reason = "calls the game's entity terminate")]
    pub unsafe fn terminate_entity(&self, entity: usize) {
        // SAFETY: the address is Entity::terminate of the verified build; the caller upholds the
        // contract above.
        unsafe {
            core::mem::transmute::<usize, OnePointerCall>(
                self.address_of(ENTITY_TERMINATE_FUNCTION),
            )(entity);
        }
    }

    /// # Safety
    /// `body` must be a live player body.
    #[expect(unsafe_code, reason = "calls the game's teleport")]
    pub unsafe fn teleport_reset_hap(&self, body: usize, position: &Vec4, rotation_euler: &Vec4) {
        // SAFETY: the address is teleportResetHap of the verified build; both vectors are aligned
        // and live for the call, the camera argument stays 0 so the camera is not re-targeted.
        unsafe {
            core::mem::transmute::<usize, TeleportResetHap>(
                self.address_of(TELEPORT_RESET_HAP_FUNCTION),
            )(
                body,
                position,
                rotation_euler,
                TELEPORT_WITHOUT_CAMERA_RESET,
                TELEPORT_WITHOUT_INVULNERABLE_TIME,
            );
        }
    }

    pub fn has_player_body_vtable(&self, body: usize) -> bool {
        self.read_u64_at(body as u64)
            .is_ok_and(|vtable| vtable == self.address_of(PLAYER_BODY_VTABLE) as u64)
    }

    /// # Safety
    /// `body` must be a live player body and `ids.state` not one of the never-started states.
    /// Returns false, without a call, when the body has another vtable.
    #[expect(
        unsafe_code,
        reason = "calls the body's state start through its vtable"
    )]
    pub unsafe fn start_state(&self, body: usize, ids: RemoteStateIds) -> bool {
        if !self.has_player_body_vtable(body) {
            return false;
        }
        let slot =
            self.address_of(PLAYER_BODY_VTABLE) as u64 + u64::from(STATE_START_VTABLE_OFFSET);
        let Ok(function) = self.read_u64_at(slot) else {
            return false;
        };
        // SAFETY: the function comes from slot 18 of Pl0000::vftable of the verified build, which
        // the body was just checked to use; the caller upholds the contract above.
        unsafe {
            core::mem::transmute::<usize, StartState>(function as usize)(
                body,
                i32::from(ids.state),
                ids.variant,
                ids.b,
                ids.c,
            );
        }
        true
    }
}
