use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::containers::{
    ACTION_RECORD_REMOVE_FUNCTION_RVA, ACTION_RECORD_TABLE_RVA, MAP_EFFECT_REMOVE_FUNCTION_RVA,
    TRIGGER_REMOVE_BY_ID_FUNCTION_RVA, TRIGGER_TABLE_RVA, lua_argument_offset,
    lua_integer_argument,
};
use rc_game::offsets::MAP_MANAGER_RVA;

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type RemoveById = unsafe extern "system" fn(*mut c_void, i32);
type RemoveActionRecord = unsafe extern "system" fn(*mut c_void, i32, u8);
type RemoveMapEffect = unsafe extern "system" fn(*mut c_void, i32, u32);

pub(super) fn lua_integer(
    build: &VerifiedBuild,
    arguments: *mut c_void,
    index: u64,
) -> Option<i32> {
    let memory = InProcessMemory::new(build);
    let bytes = memory
        .read_array::<16>(arguments as u64 + lua_argument_offset(index))
        .ok()?;
    let tag = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let mut payload = [0_u8; 8];
    payload.copy_from_slice(&bytes[8..16]);
    Some(lua_integer_argument(tag, payload))
}

#[expect(
    unsafe_code,
    reason = "calls the game's own trigger and action record removers by address"
)]
pub(super) fn remove_use_point(build: &VerifiedBuild, action_id: i32) {
    let base = build.module_base();
    let triggers = (base + u64::from(TRIGGER_TABLE_RVA)) as *mut c_void;
    let records = (base + u64::from(ACTION_RECORD_TABLE_RVA)) as *mut c_void;
    let remove_trigger = (base + u64::from(TRIGGER_REMOVE_BY_ID_FUNCTION_RVA)) as *const ();
    let remove_record = (base + u64::from(ACTION_RECORD_REMOVE_FUNCTION_RVA)) as *const ();
    // SAFETY: both RVAs start the functions the game's _RemoveCallbackFromId and
    // _RemoveActionTargetFromId use in the verified build, with these tables as their first
    // argument; an id that is not in a table is a no-op; this runs on the game thread.
    unsafe {
        core::mem::transmute::<*const (), RemoveById>(remove_trigger)(triggers, action_id);
        core::mem::transmute::<*const (), RemoveActionRecord>(remove_record)(records, action_id, 0);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the game's own unique effect remover by address"
)]
pub(super) fn remove_glint(build: &VerifiedBuild, handle: i32) {
    let base = build.module_base();
    let manager = (base + u64::from(MAP_MANAGER_RVA)) as *mut c_void;
    let remove = (base + u64::from(MAP_EFFECT_REMOVE_FUNCTION_RVA)) as *const ();
    // SAFETY: the RVA starts the map manager function _RemoveUniqueEffect calls in the verified
    // build (manager, handle, 0); this runs on the game thread.
    unsafe {
        core::mem::transmute::<*const (), RemoveMapEffect>(remove)(manager, handle, 0);
    }
}
