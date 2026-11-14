use nt_hook::memory::{WriteAccess, is_executable, write_access};

const PAGE_NOACCESS: u32 = 0x01;
const PAGE_READONLY: u32 = 0x02;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_WRITECOPY: u32 = 0x08;
const PAGE_EXECUTE_READ: u32 = 0x20;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const PAGE_GUARD: u32 = 0x100;
const PAGE_NOCACHE: u32 = 0x200;

#[test]
fn a_writable_page_is_written_without_a_protection_change() {
    for protection in [PAGE_READWRITE, PAGE_WRITECOPY, PAGE_EXECUTE_READWRITE] {
        assert_eq!(write_access(protection), WriteAccess::AlreadyWritable);
    }
}

#[test]
fn a_read_only_data_page_gains_write_but_never_execute() {
    assert_eq!(
        write_access(PAGE_READONLY),
        WriteAccess::WidenTo(PAGE_READWRITE)
    );
}

#[test]
fn a_code_page_keeps_execute_when_widened() {
    assert_eq!(
        write_access(PAGE_EXECUTE_READ),
        WriteAccess::WidenTo(PAGE_EXECUTE_READWRITE)
    );
}

#[test]
fn widening_keeps_the_cache_modifier() {
    assert_eq!(
        write_access(PAGE_READONLY | PAGE_NOCACHE),
        WriteAccess::WidenTo(PAGE_READWRITE | PAGE_NOCACHE)
    );
}

#[test]
fn guard_and_no_access_pages_are_refused() {
    assert_eq!(write_access(PAGE_NOACCESS), WriteAccess::Refused);
    assert_eq!(
        write_access(PAGE_READWRITE | PAGE_GUARD),
        WriteAccess::Refused
    );
}

#[test]
fn only_execute_pages_need_an_instruction_cache_flush() {
    assert!(is_executable(PAGE_EXECUTE_READ));
    assert!(!is_executable(PAGE_READWRITE));
}
