use crate::crash_report::{
    ACCESS_VIOLATION_CODE, FAIL_FAST_CODE, HEAP_CORRUPTION_CODE, ILLEGAL_INSTRUCTION_CODE,
    IN_PAGE_ERROR_CODE, INTEGER_DIVIDE_BY_ZERO_CODE, PRIVILEGED_INSTRUCTION_CODE,
    STACK_OVERFLOW_CODE,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSpot {
    pub name: String,
    pub base: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrashFacts {
    pub code: u32,
    pub address: u64,
    pub thread_id: u32,
    pub module: Option<ModuleSpot>,
    pub in_our_module: bool,
    pub access: Option<(u64, u64)>,
    pub last_chance: bool,
}

pub fn is_fatal_code(code: u32) -> bool {
    matches!(
        code,
        ACCESS_VIOLATION_CODE
            | IN_PAGE_ERROR_CODE
            | ILLEGAL_INSTRUCTION_CODE
            | STACK_OVERFLOW_CODE
            | HEAP_CORRUPTION_CODE
            | FAIL_FAST_CODE
            | INTEGER_DIVIDE_BY_ZERO_CODE
            | PRIVILEGED_INSTRUCTION_CODE
    )
}

pub fn access_violation_kind(kind: u64) -> &'static str {
    match kind {
        0 => "read",
        1 => "write",
        8 => "execute",
        _ => "other",
    }
}

pub fn last_lines(text: &str, count: usize) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(count)..].to_vec()
}

pub fn report_text(facts: &CrashFacts, log_tail: &[&str]) -> String {
    let stage = if facts.last_chance {
        "unhandled exception"
    } else if facts.in_our_module {
        "exception in our own module"
    } else {
        "first-chance fatal exception in another module"
    };
    let mut text = format!(
        "crash: {stage} code 0x{:08X} at 0x{:016X} on thread {}\n",
        facts.code, facts.address, facts.thread_id
    );
    match &facts.module {
        Some(module) => text.push_str(&format!(
            "crash: module {} base 0x{:016X} rva 0x{:X}{}\n",
            module.name,
            module.base,
            facts.address.wrapping_sub(module.base),
            if facts.in_our_module {
                " (our hook)"
            } else {
                ""
            }
        )),
        None => text.push_str("crash: address is in no loaded module\n"),
    }
    if let Some((kind, target)) = facts.access {
        text.push_str(&format!(
            "crash: {} of 0x{target:016X}\n",
            access_violation_kind(kind)
        ));
    }
    text.push_str(&format!(
        "crash: last {} log lines follow\n",
        log_tail.len()
    ));
    for line in log_tail {
        text.push_str("crash:   ");
        text.push_str(line);
        text.push('\n');
    }
    text
}
