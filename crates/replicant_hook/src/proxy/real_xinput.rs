use std::sync::OnceLock;

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

use crate::proxy::XINPUT_SYSTEM_DLL_FILE_NAME;

const SYSTEM_DIRECTORY_BUFFER_LEN: usize = 260;

static REAL_XINPUT: OnceLock<Option<RealXInput>> = OnceLock::new();

pub(super) type GetStateFn = unsafe extern "system" fn(u32, usize) -> u32;
pub(super) type SetStateFn = unsafe extern "system" fn(u32, usize) -> u32;
pub(super) type GetCapabilitiesFn = unsafe extern "system" fn(u32, u32, usize) -> u32;
pub(super) type EnableFn = unsafe extern "system" fn(i32);
pub(super) type GetDSoundGuidsFn = unsafe extern "system" fn(u32, usize, usize) -> u32;

pub(super) struct RealXInput {
    pub(super) get_state: GetStateFn,
    pub(super) set_state: SetStateFn,
    pub(super) get_capabilities: GetCapabilitiesFn,
    pub(super) enable: EnableFn,
    pub(super) get_dsound_guids: GetDSoundGuidsFn,
}

impl RealXInput {
    pub(super) fn get() -> Option<&'static Self> {
        REAL_XINPUT.get_or_init(load_from_system_directory).as_ref()
    }
}

fn load_from_system_directory() -> Option<RealXInput> {
    let path = system_dll_path()?;
    #[expect(
        unsafe_code,
        reason = "loads the real XInput DLL by full path from the system directory"
    )]
    // SAFETY: `path` is a NUL-terminated UTF-16 string that outlives the call.
    let module = unsafe { LoadLibraryW(path.as_ptr()) };
    if module.is_null() {
        return None;
    }
    Some(RealXInput {
        get_state: resolve(module, b"XInputGetState\0")?,
        set_state: resolve(module, b"XInputSetState\0")?,
        get_capabilities: resolve(module, b"XInputGetCapabilities\0")?,
        enable: resolve(module, b"XInputEnable\0")?,
        get_dsound_guids: resolve(module, b"XInputGetDSoundAudioDeviceGuids\0")?,
    })
}

fn system_dll_path() -> Option<Vec<u16>> {
    let mut buffer = [0_u16; SYSTEM_DIRECTORY_BUFFER_LEN];
    #[expect(
        unsafe_code,
        reason = "asks Windows for the system directory into a local buffer"
    )]
    // SAFETY: the buffer is live and its length is passed in UTF-16 units.
    let written = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if written == 0 || written >= buffer.len() {
        return None;
    }
    let mut path = buffer[..written].to_vec();
    path.push(u16::from(b'\\'));
    path.extend(XINPUT_SYSTEM_DLL_FILE_NAME.encode_utf16());
    path.push(0);
    Some(path)
}

fn resolve<F: Copy>(module: HMODULE, name_with_nul: &[u8]) -> Option<F> {
    #[expect(unsafe_code, reason = "looks up an export of the module just loaded")]
    // SAFETY: the module handle is valid and the name is NUL-terminated.
    let address = unsafe { GetProcAddress(module, name_with_nul.as_ptr()) }?;
    assert_eq!(size_of::<F>(), size_of_val(&address));
    #[expect(
        unsafe_code,
        reason = "an export address becomes the function pointer type it is declared as"
    )]
    // SAFETY: `F` is a function pointer type of the same size, chosen by the caller to match the export.
    Some(unsafe { core::mem::transmute_copy::<_, F>(&address) })
}
