use core::ffi::{CStr, c_void};
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::{PLAYABLE_SEED_NAME_FUNCTION_RVA, SEED_NAME_TABLE_RVA};
use rc_game::puppet::{SEED_NAME_TABLE_STRIDE, body_info, own_seed_swap};
use rc_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;

static ORIGINAL_GETTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED: OnceLock<(VerifiedBuild, PartnerBody)> = OnceLock::new();

type GetSeedName = unsafe extern "system" fn(*mut c_void) -> *const u8;

pub fn install(build: VerifiedBuild, body: PartnerBody) {
    if body == PartnerBody::Nier || HOOKED.set((build, body)).is_err() {
        return;
    }
    let info = body_info(body);
    log_line(&format!(
        "player body: seed {} becomes {} at phase load ({})",
        body_info(PartnerBody::Nier).own_seed_name,
        info.own_seed_name,
        info.own_body_changes
    ));
    let address = (build.module_base() + u64::from(PLAYABLE_SEED_NAME_FUNCTION_RVA)) as usize;
    #[expect(unsafe_code, reason = "hooks the playable seed name getter by address")]
    // SAFETY: the RVA starts the seed name getter (this, returns a NUL-terminated name; never
    // writes the save) in the verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "PlayableSeedName",
            address,
            seed_name_detour as *const () as *mut c_void,
            &ORIGINAL_GETTER,
        )
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original seed name getter through the trampoline MinHook built"
)]
extern "system" fn seed_name_detour(this: *mut c_void) -> *const u8 {
    let original = ORIGINAL_GETTER.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null();
    }
    // SAFETY: the trampoline continues into the original getter, which has this exact signature.
    let returned = unsafe { core::mem::transmute::<*mut c_void, GetSeedName>(original)(this) };
    panic::catch_unwind(|| own_body_seed(returned))
        .ok()
        .flatten()
        .unwrap_or(returned)
}

#[expect(unsafe_code, reason = "reads the game's NUL-terminated seed name strings")]
fn own_body_seed(returned: *const u8) -> Option<*const u8> {
    let (build, body) = HOOKED.get()?;
    if returned.is_null() {
        return None;
    }
    // SAFETY: the getter returns a pointer into the game's static seed name table.
    let returned_name = unsafe { CStr::from_ptr(returned.cast()) }.to_str().ok()?;
    let swap = own_seed_swap(returned_name, *body)?;
    let table = build.module_base() + u64::from(SEED_NAME_TABLE_RVA);
    let replacement = (table + u64::from(swap.table_index) * SEED_NAME_TABLE_STRIDE) as *const u8;
    // SAFETY: the table entry is a 16-byte slot of the same static table the getter returns from.
    let replacement_name = unsafe { CStr::from_ptr(replacement.cast()) }.to_str().ok()?;
    if replacement_name != swap.to {
        log_line(&format!(
            "player body: table entry {} reads {replacement_name}, not {}; seed unchanged",
            swap.table_index, swap.to
        ));
        return None;
    }
    log_line(&format!("player body: seed {}", swap.to));
    Some(replacement)
}
