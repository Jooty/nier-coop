use hudhook::Hudhook;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::Sleep;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::overlay::render_loop::DebugOverlay;
use crate::overlay::{
    RENDERER_POLL_ATTEMPTS, RENDERER_POLL_INTERVAL_MS, SETTLE_AFTER_RENDERER_LOADED_MS,
};

pub fn install_when_renderer_ready() {
    if !wait_for_d3d11() {
        log_line("overlay: d3d11.dll never loaded, no overlay");
        return;
    }
    let gate = VerifiedBuild::verify();
    match &gate {
        Ok(build) => log_line(&format!(
            "build gate: pass, base {:#x}",
            build.module_base()
        )),
        Err(error) => log_line(&format!("build gate: FAIL ({error})")),
    }
    let applied = Hudhook::builder()
        .with::<ImguiDx11Hooks>(DebugOverlay::new(gate))
        .build()
        .apply();
    match applied {
        Ok(()) => log_line("overlay: DX11 hooks applied"),
        Err(status) => log_line(&format!("overlay: hooks failed ({status:?})")),
    }
}

fn wait_for_d3d11() -> bool {
    for _ in 0..RENDERER_POLL_ATTEMPTS {
        #[expect(
            unsafe_code,
            reason = "asks the loader whether d3d11.dll is mapped yet"
        )]
        // SAFETY: the name is a static NUL-terminated UTF-16 string.
        let loaded = !unsafe { GetModuleHandleW(windows_sys::w!("d3d11.dll")) }.is_null();
        if loaded {
            #[expect(unsafe_code, reason = "sleeps this thread only")]
            // SAFETY: Sleep has no preconditions.
            unsafe {
                Sleep(SETTLE_AFTER_RENDERER_LOADED_MS)
            };
            return true;
        }
        #[expect(unsafe_code, reason = "sleeps this thread only")]
        // SAFETY: Sleep has no preconditions.
        unsafe {
            Sleep(RENDERER_POLL_INTERVAL_MS)
        };
    }
    false
}
