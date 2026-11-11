use core::ptr;

use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

use crate::build_gate::{GateError, PE_HEADER_READ_LEN, PeIdentity};

#[derive(Debug, Clone, Copy)]
pub struct VerifiedBuild {
    module_base: u64,
    size_of_image: u32,
}

impl VerifiedBuild {
    pub fn verify(supported: &[PeIdentity]) -> Result<Self, GateError> {
        let module_base = main_module_base().ok_or(GateError::NoMainModule)?;
        let identity = PeIdentity::check_supported(&module_header(module_base), supported)?;
        Ok(Self {
            module_base,
            size_of_image: identity.size_of_image,
        })
    }

    pub fn size_of_image(&self) -> u32 {
        self.size_of_image
    }

    pub fn module_base(&self) -> u64 {
        self.module_base
    }
}

#[expect(
    unsafe_code,
    reason = "GetModuleHandleW(NULL) asks the loader for the running exe"
)]
fn main_module_base() -> Option<u64> {
    // SAFETY: a null module name is documented to return the handle of the calling process's exe.
    let handle = unsafe { GetModuleHandleW(ptr::null()) };
    match handle as usize {
        0 => None,
        base => Some(base as u64),
    }
}

#[expect(
    unsafe_code,
    reason = "the PE headers are read straight out of the loaded image"
)]
fn module_header(module_base: u64) -> Vec<u8> {
    // SAFETY: the loader maps the headers (SizeOfHeaders is 0x400, inside the first page) read-only
    // at the module base for the lifetime of the process.
    unsafe { core::slice::from_raw_parts(module_base as *const u8, PE_HEADER_READ_LEN) }.to_vec()
}
