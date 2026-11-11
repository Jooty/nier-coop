use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU64, Ordering};

use replicant_game::GameMemory;
use replicant_game::marker::{
    CameraState, FRAME_KEY_BYTES, FrameKey, MATRIX_FLOATS, VIEW_LIST_SET_VIEW_RVA,
    VIEW_LIST_UPLOAD_VIEW_RVA, projection_is_cameras, read_camera_state, read_drawn_view_key,
};
use replicant_game::offsets::{
    PLAYABLE_ACTOR_X_OFFSET, PLAYABLE_ACTOR_Y_OFFSET, PLAYABLE_ACTOR_Z_OFFSET,
};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::puppet::first_actor_address;
use crate::render_camera::frame::{PresentedFrame, mark_rendered, record_tick};

static ORIGINAL_SET_VIEW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_UPLOAD_VIEW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static LAST_VIEW_LIST: AtomicU64 = AtomicU64::new(0);
static LAST_VIEW_SLOT: AtomicI32 = AtomicI32::new(0);

type SetView = unsafe extern "system" fn(*mut c_void, i32, *const c_void, *const c_void, u32, u32);
type UploadView = unsafe extern "system" fn(*mut c_void);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let set_view = set_view_detour as *const () as *mut c_void;
    let upload_view = upload_view_detour as *const () as *mut c_void;
    let at = |rva: u32| (build.module_base() + u64::from(rva)) as usize;
    #[expect(unsafe_code, reason = "hooks the render view list by its addresses")]
    // SAFETY: the RVAs start RenderViewList::SetView (list, slot, view, projection, two u32,
    // void) and RenderViewList::UploadView (entry, void) in the verified build, and each
    // replacement has the same signature.
    unsafe {
        hook_address(
            "RenderViewList::SetView",
            at(VIEW_LIST_SET_VIEW_RVA),
            set_view,
            &ORIGINAL_SET_VIEW,
        );
        hook_address(
            "RenderViewList::UploadView",
            at(VIEW_LIST_UPLOAD_VIEW_RVA),
            upload_view,
            &ORIGINAL_UPLOAD_VIEW,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original SetView through the trampoline MinHook built"
)]
extern "system" fn set_view_detour(
    list: *mut c_void,
    slot: i32,
    view: *const c_void,
    projection: *const c_void,
    flag: u32,
    extra: u32,
) {
    let original = ORIGINAL_SET_VIEW.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original SetView, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, SetView>(original)(
            list, slot, view, projection, flag, extra,
        );
    }
    let view_address = view as usize as u64;
    let list_address = list as usize as u64;
    let projection_address = projection as usize as u64;
    let _ = panic::catch_unwind(|| {
        record_this_tick(list_address, slot, (view_address, projection_address));
    });
}

#[expect(
    unsafe_code,
    reason = "calls the original UploadView through the trampoline MinHook built"
)]
extern "system" fn upload_view_detour(entry: *mut c_void) {
    let original = ORIGINAL_UPLOAD_VIEW.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let entry_address = entry as usize as u64;
    let _ = panic::catch_unwind(|| note_rendered_entry(entry_address));
    // SAFETY: the trampoline continues into the original UploadView, which has this signature.
    unsafe {
        core::mem::transmute::<*mut c_void, UploadView>(original)(entry);
    }
}

fn record_this_tick(list_address: u64, slot: i32, (view_address, projection_address): (u64, u64)) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let Some(key) = read_key(&memory, view_address) else {
        return;
    };
    let Ok(Some(camera)) = read_camera_state(&memory) else {
        return;
    };
    if view_is_cameras(&memory, projection_address, &camera) {
        LAST_VIEW_LIST.store(list_address, Ordering::Relaxed);
        LAST_VIEW_SLOT.store(slot, Ordering::Relaxed);
    }
    record_tick(
        slot,
        key,
        PresentedFrame {
            camera,
            puppet_position: first_actor_address().and_then(|actor| read_position(&memory, actor)),
        },
    );
}

fn view_is_cameras(
    memory: &InProcessMemory,
    projection_address: u64,
    camera: &CameraState,
) -> bool {
    let Ok(bytes) = memory.read_array::<{ MATRIX_FLOATS * 4 }>(projection_address) else {
        return false;
    };
    let projection: [f32; MATRIX_FLOATS] = core::array::from_fn(|index| {
        let at = index * 4;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    });
    projection_is_cameras(&projection, camera.vertical_fov, camera.aspect)
}

fn note_rendered_entry(entry_address: u64) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    if let Some(key) = read_key(&memory, entry_address) {
        mark_rendered(&key);
    }
}

pub(super) fn read_drawn_key() -> Option<FrameKey> {
    let build = HOOKED_BUILD.get()?;
    let list = LAST_VIEW_LIST.load(Ordering::Relaxed);
    if list == 0 {
        return None;
    }
    let memory = InProcessMemory::new(build);
    read_drawn_view_key(&memory, list, LAST_VIEW_SLOT.load(Ordering::Relaxed))
        .ok()
        .flatten()
}

fn read_key(memory: &InProcessMemory, address: u64) -> Option<FrameKey> {
    memory.read_array::<FRAME_KEY_BYTES>(address).ok()
}

fn read_position(memory: &InProcessMemory, actor: u64) -> Option<[f32; 3]> {
    Some([
        memory.read_f32(actor + PLAYABLE_ACTOR_X_OFFSET).ok()?,
        memory.read_f32(actor + PLAYABLE_ACTOR_Y_OFFSET).ok()?,
        memory.read_f32(actor + PLAYABLE_ACTOR_Z_OFFSET).ok()?,
    ])
}
