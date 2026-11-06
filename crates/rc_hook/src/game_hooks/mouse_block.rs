use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::core::GUID;

use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;

const DIRECT_INPUT_VERSION_8: u32 = 0x0800;
const IID_DIRECT_INPUT8_W: GUID = GUID {
    data1: 0xBF79_8031,
    data2: 0x483A,
    data3: 0x4DA2,
    data4: [0xAA, 0x99, 0x5D, 0x64, 0xED, 0x36, 0x97, 0x00],
};
const GUID_SYSTEM_MOUSE: GUID = GUID {
    data1: 0x6F1D_2B60,
    data2: 0xD5A0,
    data3: 0x11CF,
    data4: [0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00],
};
const RELEASE_VTABLE_SLOT: usize = 2;
const CREATE_DEVICE_VTABLE_SLOT: usize = 3;
const GET_CAPABILITIES_VTABLE_SLOT: usize = 3;
const GET_DEVICE_STATE_VTABLE_SLOT: usize = 9;
const GET_DEVICE_DATA_VTABLE_SLOT: usize = 10;
const DEVICE_CAPABILITIES_SIZE_BYTES: u32 = 44;
const DEVICE_TYPE_MASK: u32 = 0xFF;
const DEVICE_TYPE_MOUSE: u32 = 2;
const FAILED_HRESULT: i32 = -2_147_467_259;

static CURSOR_MODE: AtomicBool = AtomicBool::new(false);
static ORIGINAL_GET_DEVICE_STATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_DEVICE_DATA: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static KNOWN_DEVICES: Mutex<Vec<(usize, bool)>> = Mutex::new(Vec::new());

type ExportAddress = unsafe extern "system" fn() -> isize;
type DirectInput8Create =
    unsafe extern "system" fn(*mut c_void, u32, *const GUID, *mut *mut c_void, *mut c_void) -> i32;
type CreateDevice =
    unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void, *mut c_void) -> i32;
type Release = unsafe extern "system" fn(*mut c_void) -> u32;
type GetCapabilities = unsafe extern "system" fn(*mut c_void, *mut DeviceCapabilities) -> i32;
type GetDeviceState = unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> i32;
type GetDeviceData = unsafe extern "system" fn(*mut c_void, u32, *mut c_void, *mut u32, u32) -> i32;

#[repr(C)]
struct DeviceCapabilities {
    _size: u32,
    _flags: u32,
    device_type: u32,
    _reserved: [u32; 8],
}

pub fn set_cursor_mode(active: bool) {
    CURSOR_MODE.store(active, Ordering::Relaxed);
}

pub(super) fn install() {
    let (state_address, data_address) = match find_mouse_entry_points() {
        Ok(addresses) => addresses,
        Err(reason) => {
            log_line(&format!("hooks: no mouse block ({reason})"));
            return;
        }
    };
    #[expect(unsafe_code, reason = "hooks the mouse device's GetDeviceState")]
    // SAFETY: the address came from slot 9 of a live IDirectInputDevice8W, whose signature
    // is (this, byte count, buffer) returning an HRESULT, matching the replacement.
    unsafe {
        hook_address(
            "mouse GetDeviceState",
            state_address,
            get_device_state_detour as *const () as *mut c_void,
            &ORIGINAL_GET_DEVICE_STATE,
        )
    };
    #[expect(unsafe_code, reason = "hooks the mouse device's GetDeviceData")]
    // SAFETY: the address came from slot 10 of a live IDirectInputDevice8W, whose signature
    // is (this, element size, buffer, in-out count, flags) returning an HRESULT.
    unsafe {
        hook_address(
            "mouse GetDeviceData",
            data_address,
            get_device_data_detour as *const () as *mut c_void,
            &ORIGINAL_GET_DEVICE_DATA,
        )
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original GetDeviceState through the trampoline MinHook built"
)]
extern "system" fn get_device_state_detour(
    device: *mut c_void,
    size: u32,
    data: *mut c_void,
) -> i32 {
    let original = ORIGINAL_GET_DEVICE_STATE.load(Ordering::Acquire);
    if original.is_null() {
        return FAILED_HRESULT;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, GetDeviceState>(original)(device, size, data)
    };
    if result >= 0 && !data.is_null() && mouse_should_be_blocked(device) {
        // SAFETY: the game passed `size` writable bytes as the state buffer.
        unsafe { ptr::write_bytes(data.cast::<u8>(), 0, size as usize) };
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original GetDeviceData through the trampoline MinHook built"
)]
extern "system" fn get_device_data_detour(
    device: *mut c_void,
    element_size: u32,
    elements: *mut c_void,
    element_count: *mut u32,
    flags: u32,
) -> i32 {
    let original = ORIGINAL_GET_DEVICE_DATA.load(Ordering::Acquire);
    if original.is_null() {
        return FAILED_HRESULT;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, GetDeviceData>(original)(
            device,
            element_size,
            elements,
            element_count,
            flags,
        )
    };
    if result >= 0 && !element_count.is_null() && mouse_should_be_blocked(device) {
        // SAFETY: the game passed a writable in-out count; zero tells it no events arrived.
        unsafe { element_count.write(0) };
    }
    result
}

fn mouse_should_be_blocked(device: *mut c_void) -> bool {
    if !CURSOR_MODE.load(Ordering::Relaxed) {
        return false;
    }
    let address = device as usize;
    panic::catch_unwind(|| device_is_mouse(address)).unwrap_or(false)
}

fn device_is_mouse(address: usize) -> bool {
    let known = KNOWN_DEVICES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, is_mouse)) = known
        .iter()
        .find(|(known_address, _)| *known_address == address)
    {
        return *is_mouse;
    }
    drop(known);
    let is_mouse = read_device_is_mouse(address as *mut c_void);
    KNOWN_DEVICES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((address, is_mouse));
    is_mouse
}

fn read_device_is_mouse(device: *mut c_void) -> bool {
    let mut capabilities = DeviceCapabilities {
        _size: DEVICE_CAPABILITIES_SIZE_BYTES,
        _flags: 0,
        device_type: 0,
        _reserved: [0; 8],
    };
    #[expect(unsafe_code, reason = "asks the device what kind of device it is")]
    // SAFETY: `device` is the `this` the game passed to a device method, and GetCapabilities
    // (slot 3) takes a sized DIDEVCAPS that `capabilities` matches.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, GetCapabilities>(vtable_slot(
            device,
            GET_CAPABILITIES_VTABLE_SLOT,
        ))(device, &raw mut capabilities)
    };
    result >= 0 && capabilities.device_type & DEVICE_TYPE_MASK == DEVICE_TYPE_MOUSE
}

fn find_mouse_entry_points() -> Result<(usize, usize), &'static str> {
    #[expect(unsafe_code, reason = "looks up an export of an already loaded DLL")]
    // SAFETY: the module name and export name are static NUL-terminated strings.
    let create = unsafe {
        let module = GetModuleHandleW(windows_sys::w!("dinput8.dll"));
        if module.is_null() {
            return Err("dinput8.dll is not loaded");
        }
        GetProcAddress(module, windows_sys::s!("DirectInput8Create"))
    }
    .ok_or("dinput8.dll has no DirectInput8Create")?;
    #[expect(
        unsafe_code,
        reason = "the export is called with its documented signature"
    )]
    // SAFETY: the export named DirectInput8Create has the DirectInput8Create signature.
    let create = unsafe { core::mem::transmute::<ExportAddress, DirectInput8Create>(create) };
    probe_mouse_device(create)
}

#[expect(
    unsafe_code,
    reason = "creates a probe mouse device to learn the method addresses"
)]
fn probe_mouse_device(create: DirectInput8Create) -> Result<(usize, usize), &'static str> {
    let mut direct_input: *mut c_void = ptr::null_mut();
    // SAFETY: every pointer passed is valid for the call.
    let created = unsafe {
        create(
            GetModuleHandleW(ptr::null()),
            DIRECT_INPUT_VERSION_8,
            &IID_DIRECT_INPUT8_W,
            &raw mut direct_input,
            ptr::null_mut(),
        )
    };
    if created < 0 || direct_input.is_null() {
        return Err("DirectInput8Create failed");
    }
    // SAFETY: slot 3 of IDirectInput8W is CreateDevice, and both probe objects are released below.
    let (made, device) = unsafe {
        let create_device = core::mem::transmute::<*mut c_void, CreateDevice>(vtable_slot(
            direct_input,
            CREATE_DEVICE_VTABLE_SLOT,
        ));
        let mut device: *mut c_void = ptr::null_mut();
        let made = create_device(
            direct_input,
            &GUID_SYSTEM_MOUSE,
            &raw mut device,
            ptr::null_mut(),
        );
        (made, device)
    };
    let entry_points = (made >= 0 && !device.is_null()).then(|| {
        let addresses = (
            vtable_slot(device, GET_DEVICE_STATE_VTABLE_SLOT) as usize,
            vtable_slot(device, GET_DEVICE_DATA_VTABLE_SLOT) as usize,
        );
        release(device);
        addresses
    });
    release(direct_input);
    entry_points.ok_or("cannot create a probe mouse device")
}

#[expect(unsafe_code, reason = "reads a COM object's virtual table")]
fn vtable_slot(object: *mut c_void, slot: usize) -> *mut c_void {
    // SAFETY: every COM object starts with a pointer to its table of method pointers, and the
    // slots used here exist on the interfaces in this file.
    unsafe {
        let table = *object.cast::<*const *mut c_void>();
        *table.add(slot)
    }
}

#[expect(unsafe_code, reason = "calls IUnknown::Release on a probe object")]
fn release(object: *mut c_void) {
    // SAFETY: `object` is a live COM object this file created and releases exactly once.
    unsafe {
        core::mem::transmute::<*mut c_void, Release>(vtable_slot(object, RELEASE_VTABLE_SLOT))(
            object,
        );
    }
}
