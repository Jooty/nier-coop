use std::path::Path;
use std::process::Command;

use crate::steam::EXE_NAME;
use crate::steam::hide_console_window;

const LINUX_OPENER: &str = "xdg-open";

pub fn game_running() -> bool {
    if cfg!(windows) {
        return windows_task_running();
    }
    process_listing_mentions_exe(Path::new("/proc"))
}

pub fn open_url(url: &str) -> std::io::Result<()> {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new(LINUX_OPENER);
        command.arg(url);
        command
    };
    hide_console_window(&mut command);
    command.spawn().map(|_| ())
}

fn windows_task_running() -> bool {
    let mut command = Command::new("tasklist");
    command.args(["/FI", &format!("IMAGENAME eq {EXE_NAME}"), "/NH"]);
    hide_console_window(&mut command);
    command
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(EXE_NAME))
        .unwrap_or(false)
}

fn process_listing_mentions_exe(proc_root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        std::fs::read(entry.path().join("cmdline"))
            .map(|bytes| cmdline_runs_exe(&bytes))
            .unwrap_or(false)
    })
}

pub fn cmdline_runs_exe(cmdline: &[u8]) -> bool {
    cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .any(|arg| {
            arg.strip_suffix(EXE_NAME)
                .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with(['/', '\\']))
        })
}
