use std::path::Path;
use std::process::Command;

use nt_launcher_core::GameProfile;

use crate::steam::hide_console_window;

const LINUX_OPENER: &str = "xdg-open";

pub fn game_running(profile: &GameProfile) -> bool {
    !game_pids(profile).is_empty()
}

pub fn game_pids(profile: &GameProfile) -> Vec<u32> {
    if cfg!(windows) {
        return windows_task_pids(profile.exe_names);
    }
    linux_game_pids(Path::new("/proc"), profile.exe_names)
}

pub fn end_process_group(leader_pid: u32) {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("taskkill");
        command.args(["/F", "/T", "/PID", &leader_pid.to_string()]);
        command
    } else {
        let mut command = Command::new("kill");
        command.args(["-KILL", "--", &format!("-{leader_pid}")]);
        command
    };
    hide_console_window(&mut command);
    let _ = command.status();
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

fn windows_task_pids(exe_names: &[&str]) -> Vec<u32> {
    exe_names
        .iter()
        .flat_map(|exe_name| {
            let mut command = Command::new("tasklist");
            command.args([
                "/FI",
                &format!("IMAGENAME eq {exe_name}"),
                "/FO",
                "CSV",
                "/NH",
            ]);
            hide_console_window(&mut command);
            command
                .output()
                .map(|output| {
                    pids_from_tasklist_csv(&String::from_utf8_lossy(&output.stdout), exe_names)
                })
                .unwrap_or_default()
        })
        .collect()
}

pub fn pids_from_tasklist_csv(text: &str, exe_names: &[&str]) -> Vec<u32> {
    text.lines()
        .filter(|line| exe_names.iter().any(|exe_name| line.contains(exe_name)))
        .filter_map(|line| line.split("\",\"").nth(1)?.parse().ok())
        .collect()
}

fn linux_game_pids(proc_root: &Path, exe_names: &[&str]) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let cmdline = std::fs::read(entry.path().join("cmdline")).ok()?;
            cmdline_runs_exe(&cmdline, exe_names).then_some(pid)
        })
        .collect()
}

pub fn cmdline_runs_exe(cmdline: &[u8], exe_names: &[&str]) -> bool {
    cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .any(|arg| {
            exe_names.iter().any(|exe_name| {
                arg.strip_suffix(exe_name)
                    .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with(['/', '\\']))
            })
        })
}
