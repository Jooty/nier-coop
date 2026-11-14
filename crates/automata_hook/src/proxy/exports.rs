use std::panic;

use automata_game::save_redirect::game_may_start;
use nt_hook::diagnostics::log_line;
use windows_sys::Win32::Foundation::E_FAIL;

use crate::boot::boot_once;
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
    // The game makes this call once, before it builds its first save path, and leaves when it
    // fails; so this is where a game whose saves are not redirected is stopped.
    let may_start = panic::catch_unwind(|| game_may_start(boot_once().save_boot)).unwrap_or(false);
    if !may_start {
        log_line("saves: NOT redirected, the game is stopped to protect the normal saves");
        return E_FAIL;
    }
    let Some(real) = RealDirectInput::get() else {
        log_line(
            "proxy: cannot load the system dinput8.dll or find DirectInput8Create in it, the game will not start",
        );
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
