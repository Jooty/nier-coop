use std::fs;
use std::path::PathBuf;

use nt_hook::net::{AUTO_CONFIG_FILE_NAME, OLD_AUTO_CONFIG_FILE_NAMES, read_config_text};

fn scratch_folder(test_name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("test-scratch")
        .join(test_name);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("scratch folder is creatable");
    folder
}

#[test]
fn an_older_link_file_name_is_read_when_the_new_one_is_missing() {
    for (index, old_name) in OLD_AUTO_CONFIG_FILE_NAMES.iter().enumerate() {
        let folder = scratch_folder(&format!("link_config_old_name_{index}"));
        fs::write(folder.join(old_name), "link = 47200\n").expect("writable");
        assert_eq!(
            read_config_text(&folder),
            Ok(Some("link = 47200\n".to_owned()))
        );
    }
}

#[test]
fn the_new_link_file_name_wins_over_an_older_one() {
    let folder = scratch_folder("link_config_new_name_wins");
    fs::write(folder.join(AUTO_CONFIG_FILE_NAME), "link = 1\n").expect("writable");
    fs::write(folder.join(OLD_AUTO_CONFIG_FILE_NAMES[0]), "link = 2\n").expect("writable");
    assert_eq!(read_config_text(&folder), Ok(Some("link = 1\n".to_owned())));
}

#[test]
fn no_link_file_reads_nothing() {
    let folder = scratch_folder("link_config_none");
    assert_eq!(read_config_text(&folder), Ok(None));
}
