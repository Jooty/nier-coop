use hudhook::Hudhook;
use hudhook::hooks::dx11::ImguiDx11Hooks;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::Sleep;

use crate::build_gate::{GateError, PeIdentity, VerifiedBuild};
use crate::diagnostics::log_line;
use crate::overlay::render_loop::MenuOverlay;
use crate::overlay::{
    OverlayGame, RENDERER_POLL_ATTEMPTS, RENDERER_POLL_INTERVAL_MS, SETTLE_AFTER_RENDERER_LOADED_MS,
};

pub struct OverlayConfig<G> {
    pub supported_builds: &'static [PeIdentity],
    pub on_build_verified: fn(VerifiedBuild),
    pub make_game: fn(&Result<VerifiedBuild, GateError>) -> G,
}

pub fn install_when_renderer_ready<G: OverlayGame>(config: &OverlayConfig<G>) {
    if !wait_for_d3d11() {
        log_line("overlay: d3d11.dll never loaded, no overlay");
        return;
    }
    let gate = VerifiedBuild::verify(config.supported_builds);
    match &gate {
        Ok(build) => log_line(&format!(
            "build gate: pass, base {:#x}, heap sample {:#x}",
            build.module_base(),
            Box::into_raw(Box::new(0_u8)) as usize
        )),
        Err(error) => log_line(&format!("build gate: FAIL ({error})")),
    }
    if let Ok(build) = gate {
        (config.on_build_verified)(build);
    }
    let game = (config.make_game)(&gate);
    let applied = Hudhook::builder()
        .with::<ImguiDx11Hooks>(MenuOverlay::new(game, gate))
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
