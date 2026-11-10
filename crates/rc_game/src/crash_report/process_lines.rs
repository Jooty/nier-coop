pub struct ExitCaller<'a> {
    pub module: &'a str,
    pub rva: u64,
}

pub fn panic_line(message: &str, location: Option<(&str, u32, u32)>, thread: &str) -> String {
    match location {
        Some((file, line, column)) => {
            format!("panic: {message} at {file}:{line}:{column} (thread {thread})")
        }
        None => format!("panic: {message} at an unknown place (thread {thread})"),
    }
}

pub fn detach_line(reserved_is_null: bool, thread_id: u32) -> String {
    let how = if reserved_is_null {
        "the DLL was unloaded with FreeLibrary"
    } else {
        "the game process is ending (DLL_PROCESS_DETACH with a process-exit reserved pointer)"
    };
    format!("process: {how}, on thread {thread_id}")
}

pub fn exit_call_line(
    api: &str,
    exit_code: u32,
    thread_id: u32,
    caller: Option<ExitCaller<'_>>,
) -> String {
    let from = match caller {
        Some(caller) => format!("{} rva 0x{:X}", caller.module, caller.rva),
        None => "no known module".to_owned(),
    };
    format!(
        "process: {api} called with exit code {exit_code} on thread {thread_id}, first return address in {from}"
    )
}

pub fn at_exit_line(thread_id: u32) -> String {
    format!("process: C runtime exit handlers are running on thread {thread_id}")
}
