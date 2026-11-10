use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::crash_report::{ExitCaller, at_exit_line, exit_call_line};
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, GetProcessId,
};
use windows_sys::core::BOOL;

use crate::crash_log::handlers::{scan_stack, write_durable_line};
use crate::detour::hook_address;
use crate::diagnostics::log_line;

const CURRENT_PROCESS_PSEUDO_HANDLE: isize = -1;

static ORIGINAL_EXIT_PROCESS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_TERMINATE_PROCESS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type ExitProcessFn = unsafe extern "system" fn(u32);
type TerminateProcessFn = unsafe extern "system" fn(HANDLE, u32) -> BOOL;

#[expect(
    unsafe_code,
    reason = "registers a C runtime exit handler so a normal exit is told apart from an abort"
)]
unsafe extern "C" {
    fn atexit(callback: extern "C" fn()) -> i32;
}

pub(super) fn install() {
    #[expect(unsafe_code, reason = "calls the C runtime's atexit")]
    // SAFETY: the callback is a plain function that never unwinds and may run at process exit.
    let registered = unsafe { atexit(at_exit_handler) };
    if registered != 0 {
        log_line("crash log: the exit handler did not register");
    }
    hook_kernel32_export(
        windows_sys::s!("ExitProcess"),
        "kernel32 ExitProcess",
        exit_process_detour as *const () as *mut c_void,
        &ORIGINAL_EXIT_PROCESS,
    );
    hook_kernel32_export(
        windows_sys::s!("TerminateProcess"),
        "kernel32 TerminateProcess",
        terminate_process_detour as *const () as *mut c_void,
        &ORIGINAL_TERMINATE_PROCESS,
    );
}

#[expect(unsafe_code, reason = "looks up an export of the loaded kernel32.dll")]
fn hook_kernel32_export(
    export_name: *const u8,
    label: &str,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) {
    // SAFETY: the module name is a static NUL-terminated string and `export_name` comes from `s!`.
    let address = unsafe {
        let module = GetModuleHandleW(windows_sys::w!("kernel32.dll"));
        if module.is_null() {
            log_line(&format!("hooks: no {label} (kernel32.dll is not loaded)"));
            return;
        }
        GetProcAddress(module, export_name)
    };
    let Some(address) = address else {
        log_line(&format!("hooks: no {label} (export not found)"));
        return;
    };
    #[expect(unsafe_code, reason = "hooks a kernel32 export")]
    // SAFETY: the replacement has the same signature as the export it replaces.
    unsafe {
        hook_address(label, address as usize, replacement, original)
    };
}

extern "C" fn at_exit_handler() {
    #[expect(unsafe_code, reason = "reads the current thread id")]
    // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
    let thread_id = unsafe { GetCurrentThreadId() };
    write_durable_line(&at_exit_line(thread_id));
}

fn log_exit_call(api: &str, exit_code: u32) {
    let marker = 0_u8;
    let frames = scan_stack(ptr::addr_of!(marker) as usize as u64);
    let caller = frames.first().map(|frame| ExitCaller {
        module: frame.module.name.as_str(),
        rva: frame.address.wrapping_sub(frame.module.base),
    });
    #[expect(unsafe_code, reason = "reads the current thread id")]
    // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
    let thread_id = unsafe { GetCurrentThreadId() };
    write_durable_line(&exit_call_line(api, exit_code, thread_id, caller));
}

fn handle_is_this_process(process: HANDLE) -> bool {
    if process as isize == CURRENT_PROCESS_PSEUDO_HANDLE {
        return true;
    }
    #[expect(unsafe_code, reason = "asks which process a handle names")]
    // SAFETY: GetProcessId only reads the handle and returns 0 for one it cannot use.
    let (named, own) = unsafe { (GetProcessId(process), GetCurrentProcessId()) };
    named == own
}

#[expect(
    unsafe_code,
    reason = "calls the original ExitProcess through the trampoline MinHook built"
)]
extern "system" fn exit_process_detour(exit_code: u32) {
    let _ = std::panic::catch_unwind(|| log_exit_call("ExitProcess", exit_code));
    let original = ORIGINAL_EXIT_PROCESS.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original ExitProcess, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, ExitProcessFn>(original)(exit_code) };
}

#[expect(
    unsafe_code,
    reason = "calls the original TerminateProcess through the trampoline MinHook built"
)]
extern "system" fn terminate_process_detour(process: HANDLE, exit_code: u32) -> BOOL {
    let own = std::panic::catch_unwind(|| handle_is_this_process(process)).unwrap_or(false);
    if own {
        let _ = std::panic::catch_unwind(|| log_exit_call("TerminateProcess", exit_code));
    }
    let original = ORIGINAL_TERMINATE_PROCESS.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original TerminateProcess, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, TerminateProcessFn>(original)(process, exit_code) }
}
