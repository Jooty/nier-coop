use rc_launcher::game_link::{GAME_LINK_FILE_NAME, clear_boot, clear_link, write_boot, write_link};

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
    std::fs::write(dir.join(GAME_LINK_FILE_NAME), "boot_slot = 1\n").unwrap();
    write_link(&dir, 5000, "Ayla").unwrap();
    clear_link(&dir).unwrap();
    let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
    assert!(text.contains("boot_slot = 1"));
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

#[test]
fn boot_keys_are_written_and_other_keys_kept() {
    let dir = fresh_dir("boot_write");
    std::fs::write(dir.join(GAME_LINK_FILE_NAME), "name = \"Ayla\"\n").unwrap();
    write_boot(&dir, Some(3), true).unwrap();
    let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
    assert!(text.contains("name = \"Ayla\""));
    assert!(text.contains("boot_slot = 3"));
    assert!(text.contains("block_saves = true"));
}

#[test]
fn a_host_boot_removes_a_stale_save_block() {
    let dir = fresh_dir("boot_host");
    write_boot(&dir, Some(1), true).unwrap();
    write_boot(&dir, None, false).unwrap();
    assert!(
        !dir.join(GAME_LINK_FILE_NAME).exists() || {
            let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
            !text.contains("block_saves") && !text.contains("boot_slot")
        }
    );
}

#[test]
fn clearing_boot_keeps_the_link() {
    let dir = fresh_dir("boot_clear");
    write_link(&dir, 5000, "Ayla").unwrap();
    write_boot(&dir, Some(0), true).unwrap();
    clear_boot(&dir).unwrap();
    let text = std::fs::read_to_string(dir.join(GAME_LINK_FILE_NAME)).unwrap();
    assert!(text.contains("link = 5000"));
    assert!(!text.contains("boot_slot"));
}
