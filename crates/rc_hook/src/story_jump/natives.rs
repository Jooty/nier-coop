use rc_game::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_STRIDE_BYTES,
    NATIVE_ARG_VALUE_OFFSET_BYTES, NATIVE_ARGS_HEADER_BYTES,
};

use crate::build_gate::VerifiedBuild;

type LuaNative = unsafe extern "system" fn(*mut u8);

const BYTES_PER_WORD: usize = 8;

#[expect(
    unsafe_code,
    reason = "calls a game script native at its verified address with a hand-built argument block"
)]
pub(super) fn call_native(build: &VerifiedBuild, rva: u32, integers: &[i32]) {
    let bytes = NATIVE_ARGS_HEADER_BYTES
        + NATIVE_ARG_FIRST_OFFSET_BYTES
        + integers.len() * NATIVE_ARG_STRIDE_BYTES
        + NATIVE_ARG_VALUE_OFFSET_BYTES
        + BYTES_PER_WORD;
    let mut block = vec![0_u64; bytes.div_ceil(BYTES_PER_WORD)];
    let base = block.as_mut_ptr().cast::<u8>();
    for (index, value) in integers.iter().enumerate() {
        let argument = NATIVE_ARGS_HEADER_BYTES
            + NATIVE_ARG_FIRST_OFFSET_BYTES
            + index * NATIVE_ARG_STRIDE_BYTES;
        // SAFETY: `block` holds the header plus every argument slot, so both writes stay inside it,
        // and the offsets are 4-byte aligned from the 8-byte aligned start.
        unsafe {
            base.add(argument)
                .cast::<u32>()
                .write(NATIVE_ARG_INTEGER_TYPE_TAG);
            base.add(argument + NATIVE_ARG_VALUE_OFFSET_BYTES)
                .cast::<i32>()
                .write(*value);
        }
    }
    let function = (build.module_base() + u64::from(rva)) as usize;
    // SAFETY: the build gate passed, so the RVA starts the native that Ghidra named there; it reads
    // its integer arguments from the block built above and this runs on the game thread inside the
    // script manager tick, where the scripts call natives.
    unsafe {
        core::mem::transmute::<usize, LuaNative>(function)(base);
    }
}
