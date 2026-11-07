use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

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
const FILTER_RECHECK_INTERVAL: Duration = Duration::from_secs(1);
const NO_PREVIOUS_FILTER: usize = 0;

static VECTORED_REPORTS_WRITTEN: AtomicUsize = AtomicUsize::new(0);
static FILTER_TO_CHAIN_TO: AtomicUsize = AtomicUsize::new(NO_PREVIOUS_FILTER);

pub fn install() {
    #[expect(
        unsafe_code,
        reason = "registers process-wide exception handlers through the Win32 API"
    )]
    // SAFETY: both handlers match the Win32 signatures, never unwind (panics are caught) and always
    // return EXCEPTION_CONTINUE_SEARCH, so the game's own handling and the crash itself go on.
    unsafe {
        AddVectoredExceptionHandler(VECTORED_HANDLER_RUNS_FIRST, Some(vectored_handler));
    }
    take_over_unhandled_filter();
    let watcher = thread::Builder::new()
        .name("crash-log-filter-watch".to_owned())
        .spawn(watch_unhandled_filter);
    if watcher.is_err() {
        log_line("crash log: the filter watcher thread did not start");
    }
    log_line("crash log: exception handlers installed");
}

fn watch_unhandled_filter() {
    loop {
        thread::sleep(FILTER_RECHECK_INTERVAL);
        if take_over_unhandled_filter() {
            log_line("crash log: the game replaced the unhandled exception filter, ours is back");
        }
    }
}

#[expect(
    unsafe_code,
    reason = "installs the unhandled exception filter through the Win32 API"
)]
fn take_over_unhandled_filter() -> bool {
    // SAFETY: the filter matches the Win32 signature, never unwinds (panics are caught) and
    // chains to the filter it replaced.
    let previous = unsafe { SetUnhandledExceptionFilter(Some(last_chance_filter)) };
    let ours = last_chance_filter as *const () as usize;
    let previous = previous.map_or(NO_PREVIOUS_FILTER, |filter| filter as usize);
    if previous == ours {
        return false;
    }
    FILTER_TO_CHAIN_TO.store(previous, Ordering::Release);
    previous != NO_PREVIOUS_FILTER
}

extern "system" fn vectored_handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    let _ = panic::catch_unwind(|| report_first_chance(info));
    EXCEPTION_CONTINUE_SEARCH
}

#[expect(
    unsafe_code,
    reason = "calls the unhandled exception filter this one replaced"
)]
extern "system" fn last_chance_filter(info: *const EXCEPTION_POINTERS) -> i32 {
    let _ = panic::catch_unwind(|| write_report(info, true));
    let chained = FILTER_TO_CHAIN_TO.load(Ordering::Acquire);
    if chained == NO_PREVIOUS_FILTER {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    // SAFETY: the address came from SetUnhandledExceptionFilter, so it is a filter with this
    // signature, and it is never ours (take_over_unhandled_filter skips that case).
    let filter: unsafe extern "system" fn(*const EXCEPTION_POINTERS) -> i32 =
        unsafe { core::mem::transmute(chained) };
    // SAFETY: see above; the exception pointers are the ones the system passed to this filter.
    unsafe { filter(info) }
}

#[expect(
    unsafe_code,
    reason = "reads the exception code before any loader call"
)]
fn report_first_chance(info: *const EXCEPTION_POINTERS) {
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
        write_durable_line(line);
    }
}

fn write_durable_line(line: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = writeln!(file, "{line}");
        let _ = file.sync_all();
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
    let own_base = module_at(read_facts as *const () as usize as u64).map(|spot| spot.base);
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
