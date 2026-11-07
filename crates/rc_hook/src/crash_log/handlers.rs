use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};

use rc_game::crash_report::{
    ACCESS_VIOLATION_CODE, CRASH_REPORT_LOG_LINES, CRASH_REPORT_MAX_VECTORED_REPORTS, CrashFacts,
    ModuleSpot, is_fatal_code, last_lines, report_text,
};
use windows_sys::Win32::Foundation::{HMODULE, MAX_PATH};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_POINTERS, SetUnhandledExceptionFilter,
};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::diagnostics::{log_line, log_path};

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const VECTORED_HANDLER_RUNS_FIRST: u32 = 1;

static VECTORED_REPORTS_WRITTEN: AtomicUsize = AtomicUsize::new(0);

pub fn install() {
    #[expect(
        unsafe_code,
        reason = "registers process-wide exception handlers through the Win32 API"
    )]
    // SAFETY: both handlers match the Win32 signatures, never unwind (panics are caught) and always
    // return EXCEPTION_CONTINUE_SEARCH, so the game's own handling and the crash itself go on.
    unsafe {
        AddVectoredExceptionHandler(VECTORED_HANDLER_RUNS_FIRST, Some(vectored_handler));
        SetUnhandledExceptionFilter(Some(last_chance_filter));
    }
    log_line("crash log: exception handlers installed");
}

extern "system" fn vectored_handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    let _ = panic::catch_unwind(|| report_if_ours(info));
    EXCEPTION_CONTINUE_SEARCH
}

extern "system" fn last_chance_filter(info: *const EXCEPTION_POINTERS) -> i32 {
    let _ = panic::catch_unwind(|| write_report(info, true));
    EXCEPTION_CONTINUE_SEARCH
}

#[expect(
    unsafe_code,
    reason = "reads the exception code before any loader call"
)]
fn report_if_ours(info: *const EXCEPTION_POINTERS) {
    // SAFETY: the system passes a valid EXCEPTION_POINTERS and record to every handler.
    let fatal = !info.is_null()
        && unsafe {
            let record = (*info).ExceptionRecord;
            !record.is_null() && is_fatal_code((*record).ExceptionCode as u32)
        };
    if !fatal {
        return;
    }
    let Some(facts) = read_facts(info, false) else {
        return;
    };
    if !facts.in_our_module {
        return;
    }
    if VECTORED_REPORTS_WRITTEN.fetch_add(1, Ordering::AcqRel) >= CRASH_REPORT_MAX_VECTORED_REPORTS
    {
        return;
    }
    write_facts(&facts);
}

fn write_report(info: *const EXCEPTION_POINTERS, last_chance: bool) {
    if let Some(facts) = read_facts(info, last_chance) {
        write_facts(&facts);
    }
}

fn write_facts(facts: &CrashFacts) {
    let log = std::fs::read_to_string(log_path()).unwrap_or_default();
    let tail = last_lines(&log, CRASH_REPORT_LOG_LINES);
    for line in report_text(facts, &tail).lines() {
        log_line(line);
    }
}

#[expect(
    unsafe_code,
    reason = "reads the exception record the system passed to the handler"
)]
fn read_facts(info: *const EXCEPTION_POINTERS, last_chance: bool) -> Option<CrashFacts> {
    if info.is_null() {
        return None;
    }
    // SAFETY: the system passes a valid EXCEPTION_POINTERS to every handler; null was checked.
    let record = unsafe { (*info).ExceptionRecord };
    if record.is_null() {
        return None;
    }
    // SAFETY: the record pointer was checked and stays valid for the duration of the handler.
    let record = unsafe { &*record };
    let code = record.ExceptionCode as u32;
    let address = record.ExceptionAddress as u64;
    let module = module_at(address);
    let own_base = module_at(read_facts as usize as u64).map(|spot| spot.base);
    let in_our_module = module
        .as_ref()
        .is_some_and(|spot| Some(spot.base) == own_base);
    let access = (code == ACCESS_VIOLATION_CODE && record.NumberParameters >= 2).then(|| {
        (
            record.ExceptionInformation[0] as u64,
            record.ExceptionInformation[1] as u64,
        )
    });
    Some(CrashFacts {
        code,
        address,
        // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
        thread_id: unsafe { GetCurrentThreadId() },
        module,
        in_our_module,
        access,
        last_chance,
    })
}

#[expect(
    unsafe_code,
    reason = "asks the loader which module holds an address and for that module's file name"
)]
fn module_at(address: u64) -> Option<ModuleSpot> {
    let mut handle: HMODULE = core::ptr::null_mut();
    // SAFETY: with FROM_ADDRESS the second argument is an address, not a string; the flags keep
    // the module's reference count unchanged, so no handle needs releasing.
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address as *const u16,
            &mut handle,
        )
    };
    if found == 0 || handle.is_null() {
        return None;
    }
    let mut buffer = [0_u16; MAX_PATH as usize];
    // SAFETY: the buffer holds MAX_PATH wide characters and its length is passed.
    let length = unsafe { GetModuleFileNameW(handle, buffer.as_mut_ptr(), MAX_PATH) } as usize;
    let path = String::from_utf16_lossy(&buffer[..length.min(buffer.len())]);
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .to_owned();
    Some(ModuleSpot {
        name,
        base: handle as usize as u64,
    })
}
