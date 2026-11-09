use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::coop_pause::{
    GET_SLOT_ZERO_FUNCTION_RVA, NEAR_CALL_LENGTH, RADIO_SLOT_ZERO_CALL_SITE_RVAS,
    SET_SLOT_ZERO_FUNCTION_RVA, TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
    TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA, TalkWindowTaints, near_call_bytes,
    radio_slot_zero, talk_window_restored_slot_zero,
};
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::menu_no_pause::{
    menu_added_bits_now, no_pause_applies, note_slot_zero_writer, pause_menu_open,
};
use crate::game_hooks::other_threads_frozen::OtherThreadsFrozen;

const STUB_SEARCH_FIRST_OFFSET: u64 = 0x0800_0000;
const STUB_SEARCH_LAST_OFFSET: u64 = 0x6000_0000;
const STUB_SEARCH_STEP: u64 = 0x1_0000;
const STUB_PAGE_BYTES: usize = 0x1000;
const JUMP_STUB_BYTES: usize = 12;
const STUB_SLOT_BYTES: usize = 16;
const JUMP_STUB_SECOND_STUB_OFFSET: usize = 16;
const JUMP_STUB_THIRD_STUB_OFFSET: usize = 32;
const NO_REGISTER_CAPTURE: [u8; 0] = [];
const MOV_R8_FROM_R15: [u8; 3] = [0x4D, 0x89, 0xF8];
const MOV_RDX_FROM_RBX: [u8; 3] = [0x48, 0x89, 0xDA];
const MOV_RAX_IMMEDIATE_OPCODE: [u8; 2] = [0x48, 0xB8];
const JMP_RAX_OPCODE: [u8; 2] = [0xFF, 0xE0];

static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static RADIO_STRETCH_LOGGED: AtomicBool = AtomicBool::new(false);
static TALK_WINDOW_TAINTS: Mutex<TalkWindowTaints> = Mutex::new(TalkWindowTaints::new());
static TALK_RESTORE_STRETCH_LOGGED: AtomicBool = AtomicBool::new(false);

type SlotZeroGetter = unsafe extern "system" fn(*mut c_void) -> u32;
type SlotZeroSetter = unsafe extern "system" fn(*mut c_void, u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatchGroup {
    Radio,
    TalkRestore,
    TalkSave,
}

#[derive(Debug, Clone, Copy)]
struct PendingPatch {
    group: PatchGroup,
    site_rva: u32,
    site: u64,
    expected: [u8; NEAR_CALL_LENGTH],
    replacement: [u8; NEAR_CALL_LENGTH],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatchOutcome {
    Patched,
    BytesDiffer([u8; NEAR_CALL_LENGTH]),
    WriteRefused,
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    let Some(stubs) = allocate_stub_page_near(base) else {
        log_line("hooks: radio pause sites not patched (no executable page within 2 GB of the game)");
        return;
    };
    let radio_stub = write_jump_stub(stubs, &NO_REGISTER_CAPTURE, radio_slot_zero_detour as *const () as u64);
    let restore_stub = write_jump_stub(
        stubs.wrapping_add(JUMP_STUB_SECOND_STUB_OFFSET),
        &MOV_R8_FROM_R15,
        talk_restore_detour as *const () as u64,
    );
    let save_stub = write_jump_stub(
        stubs.wrapping_add(JUMP_STUB_THIRD_STUB_OFFSET),
        &MOV_RDX_FROM_RBX,
        talk_save_detour as *const () as u64,
    );
    let mut plans = Vec::with_capacity(RADIO_SLOT_ZERO_CALL_SITE_RVAS.len() + 2);
    for site in RADIO_SLOT_ZERO_CALL_SITE_RVAS {
        plans.extend(plan_patch(base, PatchGroup::Radio, site, GET_SLOT_ZERO_FUNCTION_RVA, radio_stub));
    }
    plans.extend(plan_patch(
        base,
        PatchGroup::TalkRestore,
        TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
        SET_SLOT_ZERO_FUNCTION_RVA,
        restore_stub,
    ));
    plans.extend(plan_patch(
        base,
        PatchGroup::TalkSave,
        TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA,
        GET_SLOT_ZERO_FUNCTION_RVA,
        save_stub,
    ));
    let outcomes = apply_with_game_threads_frozen(&plans);
    let patched = |group: PatchGroup| {
        plans
            .iter()
            .zip(&outcomes)
            .filter(|(plan, outcome)| plan.group == group && **outcome == PatchOutcome::Patched)
            .count()
    };
    let (radio_patched, restore_patched, save_patched) = (
        patched(PatchGroup::Radio),
        patched(PatchGroup::TalkRestore),
        patched(PatchGroup::TalkSave),
    );
    for (plan, outcome) in plans.iter().zip(&outcomes) {
        match outcome {
            PatchOutcome::Patched => {}
            PatchOutcome::BytesDiffer(found) => log_line(&format!(
                "hooks: radio pause site {:#x} not patched (bytes {found:02x?}, expected {:02x?})",
                plan.site_rva, plan.expected
            )),
            PatchOutcome::WriteRefused => log_line(&format!(
                "hooks: radio pause site {:#x} not patched (write refused)",
                plan.site_rva
            )),
        }
    }
    log_line(&format!(
        "hooks: radio pause sites patched {radio_patched} of {}, talk window restore site patched {restore_patched}, save site patched {save_patched}",
        RADIO_SLOT_ZERO_CALL_SITE_RVAS.len()
    ));
}

pub(super) fn end_stretch() {
    RADIO_STRETCH_LOGGED.store(false, Ordering::Relaxed);
    TALK_RESTORE_STRETCH_LOGGED.store(false, Ordering::Relaxed);
}

#[expect(unsafe_code, reason = "reserves one executable page near the game image")]
fn allocate_stub_page_near(base: u64) -> Option<*mut u8> {
    let mut offset = STUB_SEARCH_FIRST_OFFSET;
    while offset <= STUB_SEARCH_LAST_OFFSET {
        // SAFETY: VirtualAlloc with a specific address either returns that fresh page or null; it
        // never touches an existing mapping.
        let page = unsafe {
            VirtualAlloc(
                (base + offset) as *const c_void,
                STUB_PAGE_BYTES,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_EXECUTE_READWRITE,
            )
        };
        if !page.is_null() {
            return Some(page.cast());
        }
        offset += STUB_SEARCH_STEP;
    }
    None
}

#[expect(unsafe_code, reason = "writes the jump stub into the page just allocated")]
fn write_jump_stub(at: *mut u8, register_capture: &[u8], target: u64) -> u64 {
    let mut bytes = [0_u8; STUB_SLOT_BYTES];
    let jump_start = register_capture.len();
    bytes[..jump_start].copy_from_slice(register_capture);
    bytes[jump_start..jump_start + 2].copy_from_slice(&MOV_RAX_IMMEDIATE_OPCODE);
    bytes[jump_start + 2..jump_start + 10].copy_from_slice(&target.to_le_bytes());
    bytes[jump_start + 10..jump_start + JUMP_STUB_BYTES].copy_from_slice(&JMP_RAX_OPCODE);
    // SAFETY: `at` points into the fresh read-write-execute page, with room for the stub slot.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), at, STUB_SLOT_BYTES) };
    at as u64
}

fn plan_patch(
    base: u64,
    group: PatchGroup,
    site_rva: u32,
    original_target_rva: u32,
    stub_address: u64,
) -> Option<PendingPatch> {
    let site = base + u64::from(site_rva);
    let expected = near_call_bytes(site, base + u64::from(original_target_rva))?;
    let Some(replacement) = near_call_bytes(site, stub_address) else {
        log_line(&format!(
            "hooks: radio pause site {site_rva:#x} not patched (stub out of reach)"
        ));
        return None;
    };
    Some(PendingPatch {
        group,
        site_rva,
        site,
        expected,
        replacement,
    })
}

fn apply_with_game_threads_frozen(plans: &[PendingPatch]) -> Vec<PatchOutcome> {
    let mut previous_protections = Vec::with_capacity(plans.len());
    for plan in plans {
        previous_protections.push(make_site_writable(plan));
    }
    let mut outcomes = Vec::with_capacity(plans.len());
    let frozen = OtherThreadsFrozen::freeze();
    for (plan, previous) in plans.iter().zip(&previous_protections) {
        outcomes.push(if previous.is_some() {
            write_patch(plan)
        } else {
            PatchOutcome::WriteRefused
        });
    }
    drop(frozen);
    for (plan, previous) in plans.iter().zip(&previous_protections) {
        if let Some(previous) = previous {
            finish_site(plan, *previous);
        }
    }
    outcomes
}

#[expect(unsafe_code, reason = "widens one call site's page protection before any thread is frozen")]
fn make_site_writable(plan: &PendingPatch) -> Option<u32> {
    let mut previous = 0;
    // SAFETY: the site is inside the verified build's committed code image.
    let widened = unsafe {
        VirtualProtect(
            (plan.site as *const u8).cast::<c_void>(),
            NEAR_CALL_LENGTH,
            PAGE_EXECUTE_READWRITE,
            &mut previous,
        )
    };
    (widened != 0).then_some(previous)
}

#[expect(
    unsafe_code,
    reason = "rewrites one call instruction in the game's code while its other threads are frozen"
)]
fn write_patch(plan: &PendingPatch) -> PatchOutcome {
    let target = plan.site as *mut u8;
    let mut found = [0_u8; NEAR_CALL_LENGTH];
    // SAFETY: the site is committed, already read-write-execute, 5 bytes long, and all other game
    // threads are frozen, so nothing executes or writes it during the read and write.
    unsafe {
        ptr::copy_nonoverlapping(target, found.as_mut_ptr(), NEAR_CALL_LENGTH);
        if found != plan.expected {
            return PatchOutcome::BytesDiffer(found);
        }
        ptr::copy_nonoverlapping(plan.replacement.as_ptr(), target, NEAR_CALL_LENGTH);
    }
    PatchOutcome::Patched
}

#[expect(unsafe_code, reason = "flushes the instruction cache and restores the page protection after resume")]
fn finish_site(plan: &PendingPatch, previous: u32) {
    let target = (plan.site as *const u8).cast::<c_void>();
    let mut ignored = 0;
    // SAFETY: plain Win32 calls on the site widened by make_site_writable.
    unsafe {
        FlushInstructionCache(GetCurrentProcess(), target, NEAR_CALL_LENGTH);
        VirtualProtect(target, NEAR_CALL_LENGTH, previous, &mut ignored);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the game's slot 0 getter, which no hook replaces"
)]
extern "system" fn radio_slot_zero_detour(slot_owner: *mut c_void) -> u32 {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    let getter = (base + u64::from(GET_SLOT_ZERO_FUNCTION_RVA)) as usize;
    // SAFETY: the getter RVA starts `mov eax,[rcx+0x12974]; ret` in the verified build and the
    // radio call sites pass the same slot owner they passed to it.
    let real = unsafe { core::mem::transmute::<usize, SlotZeroGetter>(getter)(slot_owner) };
    let applies = no_pause_applies();
    let seen = radio_slot_zero(real, applies);
    if seen != real && !RADIO_STRETCH_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "co-op no pause: radio sees slot 0 {seen:#x} instead of {real:#x}, so it keeps advancing under the menu"
        ));
    }
    seen
}

#[expect(
    unsafe_code,
    reason = "calls the game's slot 0 getter, which no hook replaces"
)]
extern "system" fn talk_save_detour(slot_owner: *mut c_void, window: u64) -> u32 {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    let getter = (base + u64::from(GET_SLOT_ZERO_FUNCTION_RVA)) as usize;
    // SAFETY: the getter RVA starts `mov eax,[rcx+0x12974]; ret` in the verified build and the
    // talk window's save site passes the same slot owner it passed to it.
    let real = unsafe { core::mem::transmute::<usize, SlotZeroGetter>(getter)(slot_owner) };
    let menu_bits = if pause_menu_open() {
        menu_added_bits_now()
    } else {
        0
    };
    TALK_WINDOW_TAINTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .record(window, menu_bits);
    note_slot_zero_writer();
    real
}

#[expect(
    unsafe_code,
    reason = "calls the game's slot 0 setter, which no hook replaces"
)]
extern "system" fn talk_restore_detour(slot_owner: *mut c_void, saved: u32, window: u64) {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    let menu_bits = TALK_WINDOW_TAINTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take(window);
    let restored = talk_window_restored_slot_zero(saved, menu_bits, pause_menu_open());
    note_slot_zero_writer();
    if restored != saved && !TALK_RESTORE_STRETCH_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "co-op no pause: the talk window restores slot 0 {restored:#x} instead of {saved:#x}, the menu it was opened under is closed"
        ));
    }
    let setter = (base + u64::from(SET_SLOT_ZERO_FUNCTION_RVA)) as usize;
    // SAFETY: the setter RVA starts `mov [rcx+0x12974],edx; ret` in the verified build and the
    // restore call site passes it the same slot owner.
    unsafe { core::mem::transmute::<usize, SlotZeroSetter>(setter)(slot_owner, restored) };
}
