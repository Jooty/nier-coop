use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use automata_game::address::{
    MINIMAP_AREA_SCALE_VALUE, MINIMAP_CENTRE_VALUE, MINIMAP_RECT_VALUE, MINIMAP_UPDATE_FUNCTION,
    MINIMAP_ZOOM_SCALE_VALUE,
};
use automata_game::partner::code_starts_with;
use automata_game::partner_hud::{
    GameMinimap, MINIMAP_OBJECT_KIND_OFFSET, MINIMAP_OBJECT_OPEN_STATE_OFFSET,
    MINIMAP_OBJECT_ROTATION_OFFSET, MINIMAP_OBJECT_ZOOM_OFFSET, minimap_update_start,
};

use crate::game_calls::GameCalls;
use crate::hud::install::calls;

/// The game's minimap after its last update, with the zoom it eases towards and when it was read.
#[derive(Debug, Clone, Copy)]
pub(super) struct MinimapReading {
    pub map: GameMinimap,
    pub zoom: f32,
    pub read_at: Instant,
}

static ORIGINAL_MINIMAP_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LAST_READING: Mutex<Option<MinimapReading>> = Mutex::new(None);

type MinimapUpdate = unsafe extern "system" fn(usize);

/// Reads the game's own minimap after each of its updates; false, with nothing hooked, when the
/// update's start differs from the one read in this exe.
pub(super) fn install_minimap_reader(calls: &GameCalls) -> bool {
    let noted_start = minimap_update_start(calls.variant());
    let starts_match = calls
        .read_bytes(MINIMAP_UPDATE_FUNCTION, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start));
    starts_match
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the game's minimap update; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the minimap update of the verified build and its start was just
            // checked; the replacement forwards the one object pointer.
            unsafe {
                nt_hook::detour::hook_address(
                    "minimap update (the partner's dot follows the game's minimap)",
                    calls.address_value(MINIMAP_UPDATE_FUNCTION) as usize,
                    minimap_update_detour as *const () as *mut c_void,
                    &ORIGINAL_MINIMAP_UPDATE,
                )
            }
        }
}

pub(super) fn last_minimap_reading() -> Option<MinimapReading> {
    *LAST_READING.lock().unwrap_or_else(PoisonError::into_inner)
}

#[expect(
    unsafe_code,
    reason = "calls the original minimap update through the trampoline MinHook built"
)]
extern "system" fn minimap_update_detour(minimap: usize) {
    let original = ORIGINAL_MINIMAP_UPDATE.load(Ordering::Acquire);
    if !original.is_null() {
        // SAFETY: the trampoline continues into the original function with the pointer received.
        unsafe { core::mem::transmute::<*mut c_void, MinimapUpdate>(original)(minimap) }
    }
    if let Some(reading) = calls().and_then(|calls| read_minimap(calls, minimap as u64)) {
        *LAST_READING.lock().unwrap_or_else(PoisonError::into_inner) = Some(reading);
    }
}

fn read_minimap(calls: &GameCalls, minimap: u64) -> Option<MinimapReading> {
    let centre = calls
        .read_f32s_at::<2>(calls.address_value(MINIMAP_CENTRE_VALUE))
        .ok()?;
    let [left, top, bottom, right] = calls
        .read_f32s_at::<4>(calls.address_value(MINIMAP_RECT_VALUE))
        .ok()?;
    let area_scale = calls
        .read_f32_at(calls.address_value(MINIMAP_AREA_SCALE_VALUE))
        .ok()?;
    let zoom_scale = calls
        .read_f32_at(calls.address_value(MINIMAP_ZOOM_SCALE_VALUE))
        .ok()?;
    Some(MinimapReading {
        map: GameMinimap {
            open_state: calls
                .read_i32_at(minimap + MINIMAP_OBJECT_OPEN_STATE_OFFSET)
                .ok()?,
            kind: calls
                .read_i32_at(minimap + MINIMAP_OBJECT_KIND_OFFSET)
                .ok()?,
            centre,
            left,
            top,
            bottom,
            right,
            ui_units_per_world_unit: area_scale * zoom_scale,
            rotation_radians: calls
                .read_f32_at(minimap + MINIMAP_OBJECT_ROTATION_OFFSET)
                .ok()?,
        },
        zoom: calls
            .read_f32_at(minimap + MINIMAP_OBJECT_ZOOM_OFFSET)
            .ok()?,
        read_at: Instant::now(),
    })
}
