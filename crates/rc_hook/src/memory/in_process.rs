use core::ffi::c_void;

use rc_game::{GameMemory, MemoryError};
use windows_sys::Win32::System::Diagnostics::Debug::{
    FlushInstructionCache, ReadProcessMemory, WriteProcessMemory,
};
use windows_sys::Win32::System::Memory::{
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::build_gate::VerifiedBuild;

#[derive(Debug)]
pub struct InProcessMemory {
    module_base: u64,
}

impl InProcessMemory {
    pub fn new(build: &VerifiedBuild) -> Self {
        Self {
            module_base: build.module_base(),
        }
    }
}

impl GameMemory for InProcessMemory {
    fn module_base(&self) -> u64 {
        self.module_base
    }

    #[expect(
        unsafe_code,
        reason = "reads this process's own memory through the Win32 API"
    )]
    fn read(&self, address: u64, buffer: &mut [u8]) -> Result<(), MemoryError> {
        let mut bytes_read = 0_usize;
        // SAFETY: the pseudo handle is always valid, the buffer is a live &mut [u8], and the API
        // returns an error instead of faulting when the source range is not mapped.
        let ok = unsafe {
            ReadProcessMemory(
                GetCurrentProcess(),
                address as *const c_void,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut bytes_read,
            )
        };
        if ok == 0 || bytes_read != buffer.len() {
            return Err(MemoryError::Unreadable { address });
        }
        Ok(())
    }

    #[expect(
        unsafe_code,
        reason = "writes this process's own memory through the Win32 API"
    )]
    fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        let target = address as *const c_void;
        let mut previous_protection: PAGE_PROTECTION_FLAGS = 0;
        // SAFETY: the pseudo handle is always valid and `bytes` is a live slice. The page protection
        // is widened for the copy and restored; a bad range makes the API return an error.
        let written = unsafe {
            if VirtualProtect(
                target,
                bytes.len(),
                PAGE_EXECUTE_READWRITE,
                &mut previous_protection,
            ) == 0
            {
                false
            } else {
                let process = GetCurrentProcess();
                let mut bytes_written = 0_usize;
                let copied = WriteProcessMemory(
                    process,
                    target,
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    &mut bytes_written,
                ) != 0
                    && bytes_written == bytes.len();
                let mut widened_protection: PAGE_PROTECTION_FLAGS = 0;
                VirtualProtect(
                    target,
                    bytes.len(),
                    previous_protection,
                    &mut widened_protection,
                );
                FlushInstructionCache(process, target, bytes.len());
                copied
            }
        };
        if written {
            Ok(())
        } else {
            Err(MemoryError::Unwritable { address })
        }
    }
}
