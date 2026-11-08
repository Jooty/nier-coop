use std::path::Path;
use std::process::Command;

use crate::steam::EXE_NAME;
use crate::steam::hide_console_window;

const LINUX_OPENER: &str = "xdg-open";

pub fn game_running() -> bool {
    !game_pids().is_empty()
}

pub fn game_pids() -> Vec<u32> {
    if cfg!(windows) {
        return windows_task_pids();
    }
    linux_game_pids(Path::new("/proc"))
}

pub fn end_processes(pids: &[u32]) {
    for pid in pids {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("taskkill");
            command.args(["/F", "/PID", &pid.to_string()]);
            command
        } else {
            let mut command = Command::new("kill");
            command.args(["-KILL", &pid.to_string()]);
            command
        };
        hide_console_window(&mut command);
        let _ = command.status();
    }
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

fn windows_task_pids() -> Vec<u32> {
    let mut command = Command::new("tasklist");
    command.args([
        "/FI",
        &format!("IMAGENAME eq {EXE_NAME}"),
        "/FO",
        "CSV",
        "/NH",
    ]);
    hide_console_window(&mut command);
    command
        .output()
        .map(|output| pids_from_tasklist_csv(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

pub fn pids_from_tasklist_csv(text: &str) -> Vec<u32> {
    text.lines()
        .filter(|line| line.contains(EXE_NAME))
        .filter_map(|line| line.split("\",\"").nth(1)?.parse().ok())
        .collect()
}

fn linux_game_pids(proc_root: &Path) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let cmdline = std::fs::read(entry.path().join("cmdline")).ok()?;
            cmdline_runs_exe(&cmdline).then_some(pid)
        })
        .collect()
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
