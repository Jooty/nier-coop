use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::offsets::ADD_ITEM_NUM_NATIVE_RVA;
use rc_game::pickups::{TurnInSplit, is_shared_item, split_turn_in};
use rc_game::story_progress::StoryArrays;
use rc_game::story_writes::talker_write_batches;

use crate::build_gate::VerifiedBuild;
use crate::containers::{lua_integer, set_lua_integer};
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::net::net_clock_ms;
use crate::pickups::item_calls::{hooked_build, local_count};
use crate::pickups::shared_items::{
    injected_end_is_running, lower_partner_count, note_take, partner_count, partner_shares_phase,
    send_take_to_partner,
};
use crate::story_progress::{describe_changes, read_live_arrays};

const ITEM_COUNT_ARGUMENT_MASK: u32 = 0xFF;
const TURN_IN_FLAG_REPORT_DELAY_MS: u64 = 3_000;

static ORIGINAL_ADD_ITEM_NUM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static PENDING_FLAG_REPORT: Mutex<Option<PendingFlagReport>> = Mutex::new(None);

struct PendingFlagReport {
    item_id: u16,
    story_before: Option<StoryArrays>,
    report_at_ms: u64,
}

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
    if let Some((item_id, count)) =
        panic::catch_unwind(|| injected_end_take(args_address as *mut c_void))
            .ok()
            .flatten()
    {
        log_line(&format!(
            "shared items: the partner's turn-in already took {count} of {item_id}"
        ));
        return;
    }
    let plan = panic::catch_unwind(|| plan_turn_in(args_address as *mut c_void))
        .ok()
        .flatten();
    let Some(plan) = plan else {
        call_original(original, args);
        return;
    };
    note_take(plan.item_id);
    lower_partner_count(plan.item_id, plan.split.partner);
    note_flag_report(plan.item_id);
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

pub fn report_finished_turn_in(build: &VerifiedBuild) {
    let mut pending = PENDING_FLAG_REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if pending
        .as_ref()
        .is_none_or(|report| net_clock_ms() < report.report_at_ms)
    {
        return;
    }
    let Some(report) = pending.take() else {
        return;
    };
    let flags = match (report.story_before, read_live_arrays(build)) {
        (Some(before), Some(now)) => {
            let changes: Vec<_> = talker_write_batches(&before, &now)
                .into_iter()
                .flatten()
                .collect();
            describe_changes(&changes)
        }
        _ => "unreadable".to_owned(),
    };
    log_line(&format!(
        "shared items: turn-in of {} done, flags {flags}",
        report.item_id
    ));
}

fn note_flag_report(item_id: u16) {
    let story_before = hooked_build().and_then(read_live_arrays);
    *PENDING_FLAG_REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(PendingFlagReport {
        item_id,
        story_before,
        report_at_ms: net_clock_ms() + TURN_IN_FLAG_REPORT_DELAY_MS,
    });
}

fn injected_end_take(args: *mut c_void) -> Option<(u16, u8)> {
    if !injected_end_is_running() {
        return None;
    }
    let build = hooked_build()?;
    let item_id = u16::try_from(lua_integer(build, args, 0)?).ok()?;
    let count = lua_integer(build, args, 1)?;
    if count >= 0 || !is_shared_item(item_id) {
        return None;
    }
    Some((
        item_id,
        (count.unsigned_abs() & ITEM_COUNT_ARGUMENT_MASK) as u8,
    ))
}
