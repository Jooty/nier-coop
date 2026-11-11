use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::effects::local_has_book;
use replicant_game::offsets::SET_HAVE_WHITE_BOOK_NATIVE_RVA;
use replicant_game::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_VALUE_OFFSET_BYTES, NATIVE_ARGS_HEADER_BYTES,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{hook_address, note_native};
use crate::memory::InProcessMemory;
use crate::story_writes::calls::running_call;

static ORIGINAL_SET_HAVE_WHITE_BOOK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BUILD: std::sync::OnceLock<VerifiedBuild> = std::sync::OnceLock::new();

type BookNative = unsafe extern "system" fn(*mut c_void);

pub(crate) fn install(build: VerifiedBuild) {
    let _ = BUILD.set(build);
    #[expect(
        unsafe_code,
        reason = "hooks the set-have-white-book script native by address to log its calls"
    )]
    // SAFETY: the RVA starts _SetHaveWhiteBook in the verified build (docs design/story-jump.md).
    unsafe {
        hook_address(
            "LuaNative::SetHaveWhiteBook",
            (build.module_base() + u64::from(SET_HAVE_WHITE_BOOK_NATIVE_RVA)) as usize,
            set_have_white_book_detour as *const () as *mut c_void,
            &ORIGINAL_SET_HAVE_WHITE_BOOK,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "reads the native's argument block and calls the original through the trampoline"
)]
extern "system" fn set_have_white_book_detour(args: *mut c_void) {
    let args_address = args as u64;
    let argument = panic::catch_unwind(|| {
        note_native("_SetHaveWhiteBook");
        read_first_argument(args_address)
    })
    .ok();
    let trampoline = ORIGINAL_SET_HAVE_WHITE_BOOK.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, BookNative>(trampoline)(args) };
    }
    let _ = panic::catch_unwind(|| log_book_call(argument));
}

#[expect(
    unsafe_code,
    reason = "reads one integer argument from the block the game passes to the native"
)]
fn read_first_argument(args_address: u64) -> i32 {
    let offset =
        NATIVE_ARGS_HEADER_BYTES + NATIVE_ARG_FIRST_OFFSET_BYTES + NATIVE_ARG_VALUE_OFFSET_BYTES;
    // SAFETY: the native itself reads its first integer argument from this block.
    unsafe { ((args_address as usize + offset) as *const i32).read_unaligned() }
}

fn log_book_call(argument: Option<i32>) {
    let Some(build) = BUILD.get() else {
        return;
    };
    let field = match local_has_book(&InProcessMemory::new(build)) {
        Ok(has_book) => has_book.to_string(),
        Err(_) => "unreadable".to_owned(),
    };
    let argument = argument.map_or_else(|| "?".to_owned(), |value| value.to_string());
    let caller = running_call().map_or_else(
        || "none".to_owned(),
        |call| format!("{:#x}", call.name_hash),
    );
    log_line(&format!(
        "book: _SetHaveWhiteBook({argument}), field now {field}, script call {caller}"
    ));
}
