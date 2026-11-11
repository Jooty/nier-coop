use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use automata_game::address::{GameAddress, HUD_LOG_POST_TEXT_ID_FUNCTION, UI_TEXT_LOOKUP_FUNCTION};
use automata_game::co_op_log::{
    GAME_LOG_TEXT_GROUP, GAME_LOG_TEXT_SLOT_COUNT, HUD_LOG_POST_TEXT_ID_FUNCTION_START,
    UI_TEXT_LOOKUP_FUNCTION_START, game_log_text_id, game_log_text_slot, game_log_text_units,
};
use automata_game::partner::code_starts_with;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

static ORIGINAL_TEXT_LOOKUP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static POST_TEXT_ID: AtomicUsize = AtomicUsize::new(0);
static LINE_TEXTS: [AtomicPtr<u16>; GAME_LOG_TEXT_SLOT_COUNT] =
    [const { AtomicPtr::new(ptr::null_mut()) }; GAME_LOG_TEXT_SLOT_COUNT];
static NEXT_LINE_SLOT: AtomicUsize = AtomicUsize::new(0);
static FIRST_LOOKUP_OF_A_LINE_LOGGED: AtomicBool = AtomicBool::new(false);

type TextLookup = unsafe extern "system" fn(usize, u32, u32) -> *const u16;
type PostTextId = unsafe extern "system" fn(u32);

/// Lets the game's own HUD log show the mod's lines; false, with nothing hooked, when either
/// function start differs from the one read in both exes.
pub(super) fn install_game_log(calls: &GameCalls) -> bool {
    let starts_match = start_is_the_noted_one(
        calls,
        UI_TEXT_LOOKUP_FUNCTION,
        &UI_TEXT_LOOKUP_FUNCTION_START,
    ) && start_is_the_noted_one(
        calls,
        HUD_LOG_POST_TEXT_ID_FUNCTION,
        &HUD_LOG_POST_TEXT_ID_FUNCTION_START,
    );
    let hooked = starts_match
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the game's UI text lookup; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the text lookup of the verified build and its start was just
            // checked; the replacement forwards all three arguments and the result.
            unsafe {
                nc_hook::detour::hook_address(
                    "UI text lookup (the mod's lines in the game's own log)",
                    calls.address_value(UI_TEXT_LOOKUP_FUNCTION) as usize,
                    text_lookup_detour as *const () as *mut c_void,
                    &ORIGINAL_TEXT_LOOKUP,
                )
            }
        };
    if hooked {
        POST_TEXT_ID.store(
            calls.address_value(HUD_LOG_POST_TEXT_ID_FUNCTION) as usize,
            Ordering::Release,
        );
    }
    hooked
}

pub(super) fn game_log_is_hooked() -> bool {
    POST_TEXT_ID.load(Ordering::Acquire) != 0
}

/// Game thread only: hands the line to the game's own log; false when the log is not hooked.
pub(super) fn post_to_game_log(line: &str) -> bool {
    let post = POST_TEXT_ID.load(Ordering::Acquire);
    if post == 0 {
        return false;
    }
    let slot = NEXT_LINE_SLOT.fetch_add(1, Ordering::Relaxed) % GAME_LOG_TEXT_SLOT_COUNT;
    // The game may read the text on another thread and at any later time, so a line's text is
    // never freed or changed once its pointer is published.
    let text: &'static mut [u16] = Box::leak(game_log_text_units(line).into_boxed_slice());
    LINE_TEXTS[slot].store(text.as_mut_ptr(), Ordering::Release);
    #[expect(
        unsafe_code,
        reason = "calls the game's own function that adds a line to its HUD log by text id"
    )]
    // SAFETY: the address is that function of the verified build and its start was checked at
    // install; it takes the one text id, as the game's own callers pass it.
    unsafe {
        core::mem::transmute::<usize, PostTextId>(post)(game_log_text_id(slot));
    }
    true
}

#[expect(
    unsafe_code,
    reason = "calls the original text lookup through the trampoline MinHook built"
)]
extern "system" fn text_lookup_detour(table: usize, text_id: u32, group: u32) -> *const u16 {
    if group == GAME_LOG_TEXT_GROUP
        && let Some(slot) = game_log_text_slot(text_id)
    {
        let text = LINE_TEXTS[slot].load(Ordering::Acquire);
        if !text.is_null() {
            if !FIRST_LOOKUP_OF_A_LINE_LOGGED.swap(true, Ordering::Relaxed) {
                log_line(
                    "co-op log: the game's own log asked for the text of a co-op line and got it",
                );
            }
            return text;
        }
    }
    let original = ORIGINAL_TEXT_LOOKUP.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null();
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe { core::mem::transmute::<*mut c_void, TextLookup>(original)(table, text_id, group) }
}

fn start_is_the_noted_one(calls: &GameCalls, function: GameAddress, noted_start: &[u8]) -> bool {
    calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start))
}
