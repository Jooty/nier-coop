use windows_sys::Win32::Foundation::E_FAIL;

use crate::proxy::real_direct_input::RealDirectInput;

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn DirectInput8Create(
    instance_address: usize,
    version: u32,
    interface_id_address: usize,
    out_address: usize,
    outer_address: usize,
) -> i32 {
    let Some(real) = RealDirectInput::get() else {
        return E_FAIL;
    };
    // SAFETY: the caller's arguments are passed through unchanged to the function they were meant for.
    unsafe {
        (real.create)(
            instance_address,
            version,
            interface_id_address,
            out_address,
            outer_address,
        )
    }
}
