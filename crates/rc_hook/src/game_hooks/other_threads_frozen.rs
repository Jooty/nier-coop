use core::ffi::c_void;
use core::mem::size_of;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread,
    THREAD_SUSPEND_RESUME,
};

pub(super) struct OtherThreadsFrozen {
    handles: Vec<HANDLE>,
}

impl OtherThreadsFrozen {
    #[expect(
        unsafe_code,
        reason = "enumerates this process's threads and suspends all but the caller"
    )]
    pub(super) fn freeze() -> Self {
        let handles = open_other_thread_handles();
        for handle in &handles {
            // SAFETY: each handle was opened above with suspend access and is closed only in Drop.
            unsafe { SuspendThread(*handle) };
        }
        Self { handles }
    }
}

impl Drop for OtherThreadsFrozen {
    #[expect(
        unsafe_code,
        reason = "resumes and closes the handles opened by freeze"
    )]
    fn drop(&mut self) {
        for handle in &self.handles {
            // SAFETY: each handle was opened by freeze and is closed exactly once, here.
            unsafe {
                ResumeThread(*handle);
                CloseHandle(*handle);
            }
        }
    }
}

#[expect(
    unsafe_code,
    reason = "walks a thread snapshot and opens thread handles"
)]
fn open_other_thread_handles() -> Vec<HANDLE> {
    let mut handles = Vec::new();
    // SAFETY: plain Win32 calls; the snapshot handle is closed before returning and the entry is
    // a zeroed plain struct whose size field is set before the first call.
    unsafe {
        let own_process = GetCurrentProcessId();
        let own_thread = GetCurrentThreadId();
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
            return handles;
        }
        let mut entry: THREADENTRY32 = core::mem::zeroed();
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut more = Thread32First(snapshot, &mut entry) != 0;
        while more {
            if entry.th32OwnerProcessID == own_process && entry.th32ThreadID != own_thread {
                let handle: *mut c_void = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                if !handle.is_null() {
                    handles.push(handle);
                }
            }
            more = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    handles
}
