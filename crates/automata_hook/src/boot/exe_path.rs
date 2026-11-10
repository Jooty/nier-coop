use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

const EXE_PATH_BUFFER_LEN: usize = 4096;

pub(super) fn running_exe_path() -> Option<PathBuf> {
    let mut buffer = [0_u16; EXE_PATH_BUFFER_LEN];
    #[expect(
        unsafe_code,
        reason = "asks Windows for the running exe's path into a local buffer"
    )]
    // SAFETY: a null module is the calling process's exe and the length is in UTF-16 units.
    let written =
        unsafe { GetModuleFileNameW(ptr::null_mut(), buffer.as_mut_ptr(), buffer.len() as u32) }
            as usize;
    if written == 0 || written >= buffer.len() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&buffer[..written])))
}
