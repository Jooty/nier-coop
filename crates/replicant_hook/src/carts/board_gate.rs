use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

use replicant_game::carts::{CART_BOARD_MESSAGE_ID, CART_MESSAGE_FUNCTION_RVA};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::net::net_clock_ms;

static ORIGINAL_CART_MESSAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GUEST_BOARDING_HELD: AtomicBool = AtomicBool::new(false);
static LAST_REFUSED_AT_MS: AtomicU64 = AtomicU64::new(0);

type CartMessage = unsafe extern "system" fn(*mut c_void, u32, u64, i64, i64) -> u64;

pub fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the cart actor's message method by address"
    )]
    // SAFETY: the RVA starts CActorTruck's message method (vtable slot 0x5A0) in the verified
    // build; the replacement takes the same five arguments and passes them on unchanged.
    unsafe {
        hook_address(
            "CActorTruck_Message",
            (build.module_base() + u64::from(CART_MESSAGE_FUNCTION_RVA)) as usize,
            cart_message_detour as *const () as *mut c_void,
            &ORIGINAL_CART_MESSAGE,
        );
    }
}

pub(super) fn hold_guest_boarding(held: bool) {
    GUEST_BOARDING_HELD.store(held, Ordering::Release);
}

pub(super) fn take_refusal() -> Option<u64> {
    match LAST_REFUSED_AT_MS.swap(0, Ordering::AcqRel) {
        0 => None,
        at => Some(at),
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original cart message method through the trampoline MinHook built"
)]
extern "system" fn cart_message_detour(
    cart: *mut c_void,
    message: u32,
    sender: u64,
    argument: i64,
    extra: i64,
) -> u64 {
    let original = ORIGINAL_CART_MESSAGE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    if message == CART_BOARD_MESSAGE_ID
        && GUEST_BOARDING_HELD.load(Ordering::Acquire)
        && panic::catch_unwind(note_refusal).is_ok()
    {
        // Refused before the cart asks the player aboard: no on-cart flag, no ride message.
        return 0;
    }
    // SAFETY: the trampoline continues into the original method, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, CartMessage>(original)(
            cart, message, sender, argument, extra,
        )
    }
}

fn note_refusal() {
    LAST_REFUSED_AT_MS.store(net_clock_ms().max(1), Ordering::Release);
}
