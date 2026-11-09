use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use rc_game::GameMemory;
use rc_game::coop_pause::{
    GET_SLOT_ZERO_FUNCTION_RVA, NEAR_CALL_LENGTH, RADIO_SLOT_ZERO_CALL_SITE_RVAS,
    SET_SLOT_ZERO_FUNCTION_RVA, TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
    TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA, near_call_bytes, radio_slot_zero, talk_window_restored_slot_zero,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::menu_no_pause::{menu_added_bits_now, no_pause_applies, pause_menu_open};
use crate::memory::InProcessMemory;

const STUB_SEARCH_FIRST_OFFSET: u64 = 0x0800_0000;
const STUB_SEARCH_LAST_OFFSET: u64 = 0x6000_0000;
const STUB_SEARCH_STEP: u64 = 0x1_0000;
const STUB_PAGE_BYTES: usize = 0x1000;
const JUMP_STUB_BYTES: usize = 12;
const JUMP_STUB_SECOND_STUB_OFFSET: usize = 16;
const JUMP_STUB_THIRD_STUB_OFFSET: usize = 32;
const MOV_RAX_IMMEDIATE_OPCODE: [u8; 2] = [0x48, 0xB8];
const JMP_RAX_OPCODE: [u8; 2] = [0xFF, 0xE0];

static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static RADIO_STRETCH_LOGGED: AtomicBool = AtomicBool::new(false);
static TALK_SAVE_MENU_BITS: AtomicU32 = AtomicU32::new(0);
static TALK_RESTORE_STRETCH_LOGGED: AtomicBool = AtomicBool::new(false);

type SlotZeroGetter = unsafe extern "system" fn(*mut c_void) -> u32;
type SlotZeroSetter = unsafe extern "system" fn(*mut c_void, u32);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    let Some(stubs) = allocate_stub_page_near(base) else {
        log_line("hooks: radio pause sites not patched (no executable page within 2 GB of the game)");
        return;
    };
    let radio_stub = write_jump_stub(stubs, radio_slot_zero_detour as *const () as u64);
    let restore_stub = write_jump_stub(
        stubs.wrapping_add(JUMP_STUB_SECOND_STUB_OFFSET),
        talk_restore_detour as *const () as u64,
    );
    let save_stub = write_jump_stub(
        stubs.wrapping_add(JUMP_STUB_THIRD_STUB_OFFSET),
        talk_save_detour as *const () as u64,
    );
    let mut memory = InProcessMemory::new(&build);
    let mut patched = 0_usize;
    for site in RADIO_SLOT_ZERO_CALL_SITE_RVAS {
        patched += usize::from(patch_call_site(
            &mut memory,
            base,
            site,
            GET_SLOT_ZERO_FUNCTION_RVA,
            radio_stub,
        ));
    }
    let restore_patched = patch_call_site(
        &mut memory,
        base,
        TALK_WINDOW_CLOSE_SLOT_ZERO_RESTORE_CALL_SITE_RVA,
        SET_SLOT_ZERO_FUNCTION_RVA,
        restore_stub,
    );
    let save_patched = patch_call_site(
        &mut memory,
        base,
        TALK_WINDOW_OPEN_SLOT_ZERO_SAVE_CALL_SITE_RVA,
        GET_SLOT_ZERO_FUNCTION_RVA,
        save_stub,
    );
    log_line(&format!(
        "hooks: radio pause sites patched {patched} of {}, talk window restore site patched {restore_patched}, save site patched {save_patched}",
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
fn write_jump_stub(at: *mut u8, target: u64) -> u64 {
    let mut bytes = [0_u8; JUMP_STUB_BYTES];
    bytes[..2].copy_from_slice(&MOV_RAX_IMMEDIATE_OPCODE);
    bytes[2..10].copy_from_slice(&target.to_le_bytes());
    bytes[10..].copy_from_slice(&JMP_RAX_OPCODE);
    // SAFETY: `at` points into the fresh read-write-execute page, with room for the 12 bytes.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), at, JUMP_STUB_BYTES) };
    at as u64
}

fn patch_call_site(
    memory: &mut InProcessMemory,
    base: u64,
    site_rva: u32,
    original_target_rva: u32,
    stub_address: u64,
) -> bool {
    let site = base + u64::from(site_rva);
    let Some(expected) = near_call_bytes(site, base + u64::from(original_target_rva)) else {
        return false;
    };
    let mut found = [0_u8; NEAR_CALL_LENGTH];
    if memory.read(site, &mut found).is_err() || found != expected {
        log_line(&format!(
            "hooks: radio pause site {site_rva:#x} not patched (bytes {found:02x?}, expected {expected:02x?})"
        ));
        return false;
    }
    let Some(replacement) = near_call_bytes(site, stub_address) else {
        log_line(&format!(
            "hooks: radio pause site {site_rva:#x} not patched (stub out of reach)"
        ));
        return false;
    };
    if memory.write(site, &replacement).is_err() {
        log_line(&format!(
            "hooks: radio pause site {site_rva:#x} not patched (write refused)"
        ));
        return false;
    }
    true
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
extern "system" fn talk_save_detour(slot_owner: *mut c_void) -> u32 {
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
    TALK_SAVE_MENU_BITS.store(menu_bits, Ordering::Relaxed);
    real
}

#[expect(
    unsafe_code,
    reason = "calls the game's slot 0 setter, which no hook replaces"
)]
extern "system" fn talk_restore_detour(slot_owner: *mut c_void, saved: u32) {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    let menu_bits = TALK_SAVE_MENU_BITS.load(Ordering::Relaxed);
    let restored = talk_window_restored_slot_zero(saved, menu_bits, pause_menu_open());
    if !pause_menu_open() {
        TALK_SAVE_MENU_BITS.store(0, Ordering::Relaxed);
    }
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
