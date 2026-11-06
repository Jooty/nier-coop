use core::ffi::c_void;
use core::ptr;

use rc_game::{GameMemory, MemoryError, WriteAccess, is_executable, write_access};
use windows_sys::Win32::System::Diagnostics::Debug::{FlushInstructionCache, ReadProcessMemory};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_PROTECTION_FLAGS, VirtualProtect, VirtualQuery,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::build_gate::VerifiedBuild;

#[derive(Debug)]
pub struct InProcessMemory {
    module_base: u64,
}

struct CommittedRegion {
    end: u64,
    protection: PAGE_PROTECTION_FLAGS,
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

    fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        let unwritable = MemoryError::Unwritable { address };
        if bytes.is_empty() {
            return Ok(());
        }
        let end = address.checked_add(bytes.len() as u64).ok_or(unwritable)?;
        let mut widened = Vec::new();
        let mut executable = false;
        let mut cursor = address;
        while cursor < end {
            let Some(region) = committed_region(cursor) else {
                restore_protection(&widened);
                return Err(unwritable);
            };
            let chunk_end = end.min(region.end);
            executable |= is_executable(region.protection);
            match write_access(region.protection) {
                WriteAccess::AlreadyWritable => {}
                WriteAccess::WidenTo(protection) => {
                    let Some(previous) = change_protection(cursor, chunk_end - cursor, protection)
                    else {
                        restore_protection(&widened);
                        return Err(unwritable);
                    };
                    widened.push((cursor, chunk_end - cursor, previous));
                }
                WriteAccess::Refused => {
                    restore_protection(&widened);
                    return Err(unwritable);
                }
            }
            cursor = chunk_end;
        }
        copy_into_own_memory(address, bytes);
        restore_protection(&widened);
        if executable {
            flush_instruction_cache(address, bytes.len());
        }
        Ok(())
    }
}

#[expect(unsafe_code, reason = "queries this process's own address space")]
fn committed_region(address: u64) -> Option<CommittedRegion> {
    let mut info: MEMORY_BASIC_INFORMATION = MEMORY_BASIC_INFORMATION::default();
    // SAFETY: VirtualQuery only fills the local struct and fails cleanly on any address.
    let filled = unsafe {
        VirtualQuery(
            address as *const c_void,
            &mut info,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if filled == 0 || info.State != MEM_COMMIT {
        return None;
    }
    Some(CommittedRegion {
        end: (info.BaseAddress as u64).checked_add(info.RegionSize as u64)?,
        protection: info.Protect,
    })
}

#[expect(
    unsafe_code,
    reason = "changes the protection of this process's own pages"
)]
fn change_protection(
    address: u64,
    len: u64,
    protection: PAGE_PROTECTION_FLAGS,
) -> Option<PAGE_PROTECTION_FLAGS> {
    let mut previous: PAGE_PROTECTION_FLAGS = 0;
    // SAFETY: the range lies inside one committed region that VirtualQuery just reported, and a
    // bad range makes the API return 0 instead of faulting.
    let changed = unsafe {
        VirtualProtect(
            address as *const c_void,
            len as usize,
            protection,
            &mut previous,
        )
    };
    (changed != 0).then_some(previous)
}

fn restore_protection(widened: &[(u64, u64, PAGE_PROTECTION_FLAGS)]) {
    for &(address, len, previous) in widened {
        let _ = change_protection(address, len, previous);
    }
}

#[expect(unsafe_code, reason = "copies into this process's own memory")]
fn copy_into_own_memory(address: u64, bytes: &[u8]) {
    // SAFETY: every page of the destination range was just reported committed and writable (or
    // made writable), and `bytes` is a live slice that cannot overlap game memory we write to.
    // A plain copy replaces WriteProcessMemory, which Wine routes through wineserver.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
}

#[expect(unsafe_code, reason = "flushes this process's instruction cache")]
fn flush_instruction_cache(address: u64, len: usize) {
    // SAFETY: the pseudo handle is always valid; the call only invalidates cached code.
    unsafe {
        FlushInstructionCache(GetCurrentProcess(), address as *const c_void, len);
    }
}
