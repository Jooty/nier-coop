use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::offsets::ADD_ITEM_NUM_NATIVE_RVA;
use rc_game::pickups::{TurnInSplit, is_shared_item, split_turn_in};

use crate::build_gate::VerifiedBuild;
use crate::containers::{lua_integer, set_lua_integer};
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::pickups::item_calls::{hooked_build, local_count};
use crate::pickups::shared_items::{
    lower_partner_count, partner_count, partner_shares_phase, send_take_to_partner,
};

const ITEM_COUNT_ARGUMENT_MASK: u32 = 0xFF;

static ORIGINAL_ADD_ITEM_NUM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void);

struct TurnIn {
    item_id: u16,
    wanted: u8,
    split: TurnInSplit,
}

pub fn install(build: VerifiedBuild) {
    #[expect(unsafe_code, reason = "hooks the _AddItemNum native by its address")]
    // SAFETY: the RVA starts the _AddItemNum native (lua args -> nothing) in the verified build
    // and the replacement has the same signature.
    unsafe {
        hook_address(
            "_AddItemNum native",
            (build.module_base() + u64::from(ADD_ITEM_NUM_NATIVE_RVA)) as usize,
            add_item_num_detour as *const () as *mut c_void,
            &ORIGINAL_ADD_ITEM_NUM,
        )
    };
}

extern "system" fn add_item_num_detour(args: *mut c_void) {
    let original = ORIGINAL_ADD_ITEM_NUM.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let args_address = args as usize;
    let plan = panic::catch_unwind(|| plan_turn_in(args_address as *mut c_void))
        .ok()
        .flatten();
    let Some(plan) = plan else {
        call_original(original, args);
        return;
    };
    log_line(&format!(
        "shared items: turn-in of {} {}: you took {}, partner asked for {}",
        plan.item_id, plan.wanted, plan.split.local, plan.split.partner
    ));
    if plan.split.local > 0 {
        let rewritten = hooked_build()
            .is_some_and(|build| set_lua_integer(build, args, 1, -i32::from(plan.split.local)));
        if rewritten {
            call_original(original, args);
        }
    }
    lower_partner_count(plan.item_id, plan.split.partner);
    send_take_to_partner(plan.item_id, plan.split.partner);
}

#[expect(unsafe_code, reason = "calls the original _AddItemNum native")]
fn call_original(original: *mut c_void, args: *mut c_void) {
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaNative>(original)(args) };
}

fn plan_turn_in(args: *mut c_void) -> Option<TurnIn> {
    if !partner_shares_phase() {
        return None;
    }
    let build = hooked_build()?;
    let item_id = u16::try_from(lua_integer(build, args, 0)?).ok()?;
    let count = lua_integer(build, args, 1)?;
    if count >= 0 || !is_shared_item(item_id) {
        return None;
    }
    let wanted = (count.unsigned_abs() & ITEM_COUNT_ARGUMENT_MASK) as u8;
    if wanted == 0 {
        return None;
    }
    let split = split_turn_in(wanted, local_count(item_id)?, partner_count(item_id));
    (split.partner > 0).then_some(TurnIn {
        item_id,
        wanted,
        split,
    })
}
