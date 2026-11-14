use nt_launcher::app::Role;
use nt_launcher::logs::{LogWriter, game_log_name, launcher_log_name, rotate_log};

use crate::scratch::fresh_dir;

#[test]
fn log_names_follow_the_role() {
    assert_eq!(game_log_name(Role::Host), "game-host.log");
    assert_eq!(game_log_name(Role::Guest), "game-guest.log");
    assert_eq!(launcher_log_name(Role::Host), "launcher-host.log");
    assert_eq!(launcher_log_name(Role::Guest), "launcher-guest.log");
}

#[test]
fn rotation_keeps_five_files_and_drops_the_oldest() {
    let dir = fresh_dir("log_rotate");
    let path = dir.join("game-host.log");
    for session in 0..7 {
        std::fs::write(&path, format!("session {session}")).unwrap();
        rotate_log(&path).unwrap();
    }
    std::fs::write(&path, "session 7").unwrap();
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    assert_eq!(read("game-host.log"), "session 7");
    assert_eq!(read("game-host.log.1"), "session 6");
    assert_eq!(read("game-host.log.4"), "session 3");
    assert!(!dir.join("game-host.log.5").exists());
}

#[test]
fn writer_starts_a_fresh_file_and_keeps_the_last_session() {
    let dir = fresh_dir("log_writer");
    let lines_of = |name: &str| -> Vec<String> {
        std::fs::read_to_string(dir.join(name))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    };
    let mut first = LogWriter::start(&dir, "launcher-host.log").unwrap();
    first.write_line("one").unwrap();
    let first_lines = lines_of("launcher-host.log");
    assert_eq!(first_lines.len(), 2);
    assert!(first_lines[0].starts_with("log opened "));
    assert!(first_lines[0].ends_with("role host"));
    assert!(first_lines[1].ends_with(" one"));
    let mut second = LogWriter::start(&dir, "launcher-host.log").unwrap();
    second.write_line("two").unwrap();
    assert!(lines_of("launcher-host.log")[1].ends_with(" two"));
    assert_eq!(lines_of("launcher-host.log.1"), first_lines);
}

#[test]
fn every_written_line_starts_with_a_utc_time_stamp() {
    let dir = fresh_dir("log_writer_stamp");
    let mut writer = LogWriter::start(&dir, "launcher.log").unwrap();
    writer.write_line("hello").unwrap();
    let text = std::fs::read_to_string(dir.join("launcher.log")).unwrap();
    let line = text.lines().nth(1).unwrap();
    let bytes = line.as_bytes();
    assert!(bytes[..12].iter().enumerate().all(|(at, byte)| match at {
        2 | 5 => *byte == b':',
        8 => *byte == b'.',
        _ => byte.is_ascii_digit(),
    }));
    assert_eq!(&line[12..], " hello");
}

#[cfg(unix)]
#[test]
fn unix_paths_become_wine_drive_paths() {
    let path = std::path::Path::new("/home/me/.local/share/niertogether/logs/game-host.log");
    assert_eq!(
        nt_launcher::logs::path_as_the_game_sees_it(path),
        "Z:\\home\\me\\.local\\share\\niertogether\\logs\\game-host.log"
    );
}
