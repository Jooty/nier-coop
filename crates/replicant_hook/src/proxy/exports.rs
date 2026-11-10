use windows_sys::Win32::Foundation::ERROR_DEVICE_NOT_CONNECTED;

use crate::proxy::real_xinput::RealXInput;

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn XInputGetState(user_index: u32, state_address: usize) -> u32 {
    let Some(real) = RealXInput::get() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    // SAFETY: the caller's arguments are passed through unchanged to the function they were meant for.
    unsafe { (real.get_state)(user_index, state_address) }
}

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn XInputSetState(user_index: u32, vibration_address: usize) -> u32 {
    let Some(real) = RealXInput::get() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    // SAFETY: the caller's arguments are passed through unchanged to the function they were meant for.
    unsafe { (real.set_state)(user_index, vibration_address) }
}

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn XInputGetCapabilities(
    user_index: u32,
    flags: u32,
    capabilities_address: usize,
) -> u32 {
    let Some(real) = RealXInput::get() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    // SAFETY: the caller's arguments are passed through unchanged to the function they were meant for.
    unsafe { (real.get_capabilities)(user_index, flags, capabilities_address) }
}

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn XInputEnable(enable: i32) {
    if let Some(real) = RealXInput::get() {
        // SAFETY: the caller's argument is passed through unchanged.
        unsafe {
            (real.enable)(enable);
        }
    }
}

#[expect(
    unsafe_code,
    reason = "exported by name so the game's import resolves here"
)]
#[unsafe(no_mangle)]
pub extern "system" fn XInputGetDSoundAudioDeviceGuids(
    user_index: u32,
    render_guid_address: usize,
    capture_guid_address: usize,
) -> u32 {
    let Some(real) = RealXInput::get() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    // SAFETY: the caller's arguments are passed through unchanged to the function they were meant for.
    unsafe { (real.get_dsound_guids)(user_index, render_guid_address, capture_guid_address) }
}
