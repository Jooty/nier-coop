use std::sync::{Mutex, OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::offsets::SIDE_VIEW_FLAG_RVA;
use rc_game::side_view::SideViewWatch;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static WATCH: Mutex<SideViewWatch> = Mutex::new(SideViewWatch::new());
static LAST_CALLBACK: Mutex<String> = Mutex::new(String::new());

pub(super) fn install(build: VerifiedBuild) {
    let _ = HOOKED_BUILD.set(build);
}

pub(super) fn observe() {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let address = build.module_base() + u64::from(SIDE_VIEW_FLAG_RVA);
    let Ok(flag) = InProcessMemory::new(build).read_u8(address) else {
        return;
    };
    let flipped = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(flag != 0);
    let Some(now_on) = flipped else {
        return;
    };
    let by = LAST_CALLBACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let state = if now_on { "on" } else { "off" };
    log_line(&format!("view: side view {state} here (by {by})"));
}

pub(super) fn note_callback(name: &str) {
    name.clone_into(&mut LAST_CALLBACK.lock().unwrap_or_else(PoisonError::into_inner));
}
