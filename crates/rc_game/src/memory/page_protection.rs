const PAGE_READONLY: u32 = 0x02;
const PAGE_READWRITE: u32 = 0x04;
const PAGE_WRITECOPY: u32 = 0x08;
const PAGE_EXECUTE: u32 = 0x10;
const PAGE_EXECUTE_READ: u32 = 0x20;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const PAGE_EXECUTE_WRITECOPY: u32 = 0x80;
const PAGE_GUARD: u32 = 0x100;
const PAGE_ACCESS_BITS: u32 = 0xFF;

/// How a committed page with a given Win32 protection can take a write from inside the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAccess {
    AlreadyWritable,
    /// The page must be switched to this protection for the write and switched back after.
    WidenTo(u32),
    Refused,
}

/// Picks the least widening a write needs: a data page gains write access but never execute.
pub fn write_access(protection: u32) -> WriteAccess {
    if protection & PAGE_GUARD != 0 {
        return WriteAccess::Refused;
    }
    let modifiers = protection & !PAGE_ACCESS_BITS;
    match protection & PAGE_ACCESS_BITS {
        PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY => {
            WriteAccess::AlreadyWritable
        }
        PAGE_READONLY => WriteAccess::WidenTo(PAGE_READWRITE | modifiers),
        PAGE_EXECUTE | PAGE_EXECUTE_READ => {
            WriteAccess::WidenTo(PAGE_EXECUTE_READWRITE | modifiers)
        }
        _ => WriteAccess::Refused,
    }
}

pub fn is_executable(protection: u32) -> bool {
    matches!(
        protection & PAGE_ACCESS_BITS,
        PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY
    )
}
