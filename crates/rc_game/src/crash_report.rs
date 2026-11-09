mod format;

pub use format::{
    CrashFacts, ModuleSpot, StackFrame, access_violation_kind, early_line, is_fatal_code,
    is_return_address_candidate, last_lines, report_text,
};

pub const CRASH_REPORT_LOG_LINES: usize = 20;
pub const CRASH_REPORT_MAX_VECTORED_REPORTS: usize = 4;
pub const CRASH_REPORT_MODULE_SPAN_BYTES: u64 = 0x0400_0000;
pub const CRASH_REPORT_STACK_FRAMES: usize = 16;
pub const CRASH_REPORT_STACK_SCAN_WORDS: usize = 512;
pub const BREAKPOINT_CODE: u32 = 0x8000_0003;
pub const ACCESS_VIOLATION_CODE: u32 = 0xC000_0005;
pub const IN_PAGE_ERROR_CODE: u32 = 0xC000_0006;
pub const ILLEGAL_INSTRUCTION_CODE: u32 = 0xC000_001D;
pub const STACK_OVERFLOW_CODE: u32 = 0xC000_00FD;
pub const HEAP_CORRUPTION_CODE: u32 = 0xC000_0374;
pub const FAIL_FAST_CODE: u32 = 0xC000_0409;
pub const INTEGER_DIVIDE_BY_ZERO_CODE: u32 = 0xC000_0094;
pub const PRIVILEGED_INSTRUCTION_CODE: u32 = 0xC000_0096;
