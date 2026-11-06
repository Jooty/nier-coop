use rc_launcher::game_link::{GAME_LINK_FILE_NAME, clear_link, write_link};

use crate::scratch::fresh_dir;

#[test]
fn writing_the_link_creates_the_file_with_port_and_name() {
    let dir = fresh_dir("link_write");
    write_link(&dir, 47200, "Ayla").unwrap();
    let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
    assert!(text.contains("link = 47200"));
    assert!(text.contains("name = \"Ayla\""));
}

#[test]
fn clearing_the_link_keeps_other_keys() {
    let dir = fresh_dir("link_keep");
    std::fs::write(dir.join(GAME_LINK_FILE_NAME), "harness = true\n").unwrap();
    write_link(&dir, 5000, "Ayla").unwrap();
    clear_link(&dir).unwrap();
    let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
    assert!(text.contains("harness = true"));
    assert!(!text.contains("link"));
}

#[test]
fn clearing_a_link_only_file_removes_it() {
    let dir = fresh_dir("link_remove");
    write_link(&dir, 5000, "Ayla").unwrap();
    std::fs::write(dir.join(GAME_LINK_FILE_NAME), "link = 5000\n").unwrap();
    clear_link(&dir).unwrap();
    assert!(!dir.join(GAME_LINK_FILE_NAME).exists());
    clear_link(&dir).unwrap();
}
