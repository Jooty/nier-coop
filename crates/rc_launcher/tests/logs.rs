use rc_launcher::app::Role;
use rc_launcher::logs::{
    LogWriter, game_log_name, launcher_log_name, path_as_the_game_sees_it, rotate_log,
};

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
    let mut first = LogWriter::start(&dir, "launcher.log").unwrap();
    first.write_line("one").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("launcher.log")).unwrap(),
        "one\n"
    );
    let mut second = LogWriter::start(&dir, "launcher.log").unwrap();
    second.write_line("two").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("launcher.log")).unwrap(),
        "two\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("launcher.log.1")).unwrap(),
        "one\n"
    );
}

#[cfg(unix)]
#[test]
fn unix_paths_become_wine_drive_paths() {
    let path = std::path::Path::new("/home/me/.local/share/replicant-coop/logs/game-host.log");
    assert_eq!(
        path_as_the_game_sees_it(path),
        "Z:\\home\\me\\.local\\share\\replicant-coop\\logs\\game-host.log"
    );
}
